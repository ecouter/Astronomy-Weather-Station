use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use chrono::Timelike;
use slint::ComponentHandle;
use crate::MainWindow;
use crate::app::utils::{decode_png_to_slint_image, decode_png_to_slint_image_with_black_transparency};

pub static CLOUD_COVER_IMAGES: Lazy<Mutex<HashMap<u32, Vec<u8>>>> = Lazy::new(|| Mutex::new(HashMap::new()));
pub static CLOUD_COVER_CURRENT_HOUR: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(1)); // Start from +1h

pub fn setup_cloud_cover_callbacks(main_window: &MainWindow) {
    let main_window_weak1 = main_window.as_weak();
    main_window.on_cloud_cover_previous(move || {
        debug!("Cloud cover previous button clicked");
        let window_weak = main_window_weak1.clone();
        slint::invoke_from_event_loop(move || {
            handle_cloud_cover_navigation(&window_weak, -1);
        }).unwrap();
    });

    let main_window_weak2 = main_window.as_weak();
    main_window.on_cloud_cover_next(move || {
        debug!("Cloud cover next button clicked");
        let window_weak = main_window_weak2.clone();
        slint::invoke_from_event_loop(move || {
            handle_cloud_cover_navigation(&window_weak, 1);
        }).unwrap();
    });
}

fn handle_cloud_cover_navigation(window_weak: &slint::Weak<MainWindow>, direction: i32) {
    debug!("Processing cloud cover navigation with direction: {}", direction);
    let window = window_weak.upgrade();
    if let Some(window) = window {
        debug!("Window upgraded successfully");
        // Use try_lock to avoid blocking the UI thread
        if let Ok(images) = CLOUD_COVER_IMAGES.try_lock() {
            debug!("CLOUD_COVER_IMAGES lock acquired, {} images", images.len());
            if let Ok(mut current_hour) = CLOUD_COVER_CURRENT_HOUR.try_lock() {
                debug!("CLOUD_COVER_CURRENT_HOUR lock acquired, current hour: {}", *current_hour);

                // Calculate new hour, wrap around 0-48 range
                let new_hour = if direction > 0 {
                    if *current_hour >= 48 {
                        0
                    } else {
                        *current_hour + 1
                    }
                } else {
                    if *current_hour == 0 {
                        48
                    } else {
                        *current_hour - 1
                    }
                };

                *current_hour = new_hour;
                debug!("New current hour: {}", *current_hour);

                let counter_text = format!("+{}h", *current_hour);

                // Check if we have the image for this hour
                if let Some(current_image_data) = images.get(&new_hour) {
                    debug!("Image already available for hour {}", new_hour);
                    // Decode image directly in the UI thread - should be fast for small images
                    match decode_png_to_slint_image_with_black_transparency(current_image_data) {
                        Ok(slint_image) => {
                            window.set_cloud_cover_image(slint_image);
                            window.set_cloud_cover_counter(counter_text.into());
                        }
                        Err(e) => {
                            error!("Failed to decode cloud cover image: {}", e);
                            window.set_cloud_cover_counter("Image unavailable".into());
                        }
                    }
                } else {
                    debug!("Image not available for hour {}, triggering lazy fetch", new_hour);
                    // Show counter text immediately
                    window.set_cloud_cover_counter(counter_text.into());

                    // Trigger lazy fetch in background thread
                    lazy_fetch_cloud_cover_image(window_weak.clone(), new_hour);
                }
            } else {
                debug!("Failed to acquire CLOUD_COVER_CURRENT_HOUR lock");
            }
        } else {
            debug!("Failed to acquire CLOUD_COVER_IMAGES lock");
        }
    } else {
        debug!("Failed to upgrade window weak reference");
    }
}

fn lazy_fetch_cloud_cover_image(window_weak: slint::Weak<MainWindow>, hour: u32) {
    debug!("Lazy fetching cloud cover image for hour {}", hour);

    // Get coordinates from the app (similar to how precipitation does it)
    let coordinates_opt = if let Some(window) = window_weak.upgrade() {
        crate::app::coordinates::load_coordinates(&window).ok()
    } else {
        None
    };

    if let Some((lat, lon)) = coordinates_opt {
        let window_weak_for_thread = window_weak.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                if let Ok(image_data) = fetch_single_cloud_cover_image(lat, lon, hour).await {
                    debug!("Successfully lazy fetched image for hour {}", hour);
                    // Store the image in the cache
                    {
                        let mut images = CLOUD_COVER_IMAGES.lock().unwrap();
                        images.insert(hour, image_data.clone());
                    }

                    // Update display if we're still on the same hour
                    slint::invoke_from_event_loop(move || {
                        if let Some(window) = window_weak_for_thread.upgrade() {
                            if let Ok(current_hour) = CLOUD_COVER_CURRENT_HOUR.try_lock() {
                                if *current_hour == hour {
                                    debug!("Still on hour {}, updating display with lazy fetched image", hour);
                                    match decode_png_to_slint_image_with_black_transparency(&image_data) {
                                        Ok(slint_image) => {
                                            window.set_cloud_cover_image(slint_image);
                                            window.set_cloud_cover_counter(format!("+{}h", hour).into());
                                        }
                                        Err(e) => {
                                            error!("Failed to decode lazy fetched cloud cover image: {}", e);
                                            window.set_cloud_cover_counter("Image unavailable".into());
                                        }
                                    }
                                } else {
                                    debug!("Hour changed from {} to {}, not updating display", hour, *current_hour);
                                }
                            }
                        }
                    }).ok();
                } else {
                    error!("Failed to lazy fetch cloud cover image for hour {}", hour);
                }
            });
        });
    } else {
        error!("Could not load coordinates for lazy cloud cover fetch");
    }
}

pub async fn fetch_single_cloud_cover_image(lat: f64, lon: f64, hour_offset: u32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use geomet::{GeoMetAPI, BoundingBox};
    use chrono::{Utc, Duration};

    debug!("Fetching single cloud cover image for hour +{}", hour_offset);

    // Calculate current UTC time, round to next hour for forecast
    let now = Utc::now();
    let current_hour = now.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();

    // Bounding box: ~5° radius around coordinates
    let bbox = BoundingBox::new(lon - 12.7, lon + 12.7, lat - 5.0, lat + 5.0);

    // Image dimensions for cloud cover (16:9 ratio for left section)
    let width = 1280;
    let height = 720;

    let forecast_time = current_hour + Duration::hours(hour_offset as i64);
    let time_str = forecast_time.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let api = GeoMetAPI::new()?;

    let image_data = api.get_wms_image("HRDPS.CONTINENTAL_NT", &time_str, bbox, width, height).await?;

    debug!("Successfully fetched cloud cover image for hour +{}", hour_offset);

    Ok(image_data)
}

pub async fn fetch_cloud_cover_images(lat: f64, lon: f64) -> Result<HashMap<u32, Vec<u8>>, Box<dyn std::error::Error>> {
    use geomet::{GeoMetAPI, BoundingBox};
    use chrono::{Utc, Duration};

    println!("Fetching initial cloud cover images (0-24h)...");

    // Calculate current UTC time, round to next hour for forecast
    let now = Utc::now();
    let current_hour = now.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();

    // Bounding box: ~5° radius around coordinates
    let bbox = BoundingBox::new(lon - 12.7, lon + 12.7, lat - 5.0, lat + 5.0);

    // Image dimensions for cloud cover (16:9 ratio for left section)
    let width = 1280;
    let height = 720;

    let api = GeoMetAPI::new()?;

    // Fetch initial 24 hours of HRDPS.CONTINENTAL_NT images concurrently using multiple threads
    let mut tasks = Vec::new();

    for hour_offset in 0..=24 { // Include hour 0 now
        let forecast_time = current_hour + Duration::hours(hour_offset as i64);
        let time_str = forecast_time.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let bbox_clone = bbox.clone();

        // Spawn a task for each hour (create new API instance per task)
        let task = tokio::spawn(async move {
            let api_instance = GeoMetAPI::new().unwrap();
            match api_instance.get_wms_image("HRDPS.CONTINENTAL_NT", &time_str, bbox_clone, width, height).await {
                Ok(data) => Some((hour_offset, data)),
                Err(e) => {
                    eprintln!("Failed to fetch image for hour +{}: {}", hour_offset, e);
                    None
                }
            }
        });
        tasks.push(task);
    }

    // Wait for all tasks to complete and collect results
    let mut cloud_images = HashMap::new();
    for task in tasks {
        if let Ok(Some((hour_offset, data))) = task.await {
            cloud_images.insert(hour_offset, data);
        }
    }

    println!("Cloud cover images fetched successfully ({} images)", cloud_images.len());
    Ok(cloud_images)
}

pub fn set_cloud_cover_images(main_window: &MainWindow, images: HashMap<u32, Vec<u8>>) {
    // Update global storage
    {
        let mut stored_images = CLOUD_COVER_IMAGES.lock().unwrap();
        *stored_images = images;
    }

    // Set current hour to 1 (to show +1h first)
    {
        let mut current_hour = CLOUD_COVER_CURRENT_HOUR.lock().unwrap();
        *current_hour = 1;
    }

    // Update UI with first image (+1h)
    update_cloud_cover_display(main_window);
}

pub fn set_cloud_cover_images_from_vec(main_window: &MainWindow, images: Vec<Vec<u8>>) {
    // Convert Vec<Vec<u8>> to HashMap<u32, Vec<u8>> (legacy compatibility)
    let mut image_map = HashMap::new();
    for (i, image) in images.into_iter().enumerate() {
        if i < 25 { // Assuming old format was hours 1-24, now include 0
            image_map.insert(i as u32, image);
        }
    }

    set_cloud_cover_images(main_window, image_map);
}

pub fn update_cloud_cover_display(main_window: &MainWindow) {
    let images = CLOUD_COVER_IMAGES.lock().unwrap();
    let current_hour = CLOUD_COVER_CURRENT_HOUR.lock().unwrap();

    if let Some(current_image_data) = images.get(&*current_hour) {
        let counter_text = format!("+{}h", *current_hour);

        debug!("Displaying cloud cover image: {} (hour: {})", counter_text, *current_hour);

        // Decode the PNG data to Slint image
        match decode_png_to_slint_image_with_black_transparency(current_image_data) {
            Ok(slint_image) => {
                main_window.set_cloud_cover_image(slint_image);
                main_window.set_cloud_cover_counter(counter_text.into());
            }
            Err(e) => {
                error!("Failed to decode cloud cover image: {}", e);
                main_window.set_cloud_cover_counter("Image unavailable".into());
            }
        }
    } else {
        // No image for current hour, trigger lazy fetch if not already fetching
        debug!("No image available for current hour {}, lazy fetch should be triggered by navigation", *current_hour);
        let counter_text = format!("+{}h", *current_hour);
        main_window.set_cloud_cover_counter(counter_text.into());
    }
}
