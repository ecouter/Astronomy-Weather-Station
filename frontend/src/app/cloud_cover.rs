use std::sync::Mutex;
use once_cell::sync::Lazy;
use chrono::Timelike;
use slint::ComponentHandle;
use crate::MainWindow;
use crate::app::utils::{decode_png_to_slint_image, decode_png_to_slint_image_with_black_transparency};

pub static CLOUD_COVER_IMAGES: Lazy<Mutex<Vec<Vec<u8>>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static CLOUD_COVER_INDEX: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));

pub fn setup_cloud_cover_callbacks(main_window: &MainWindow) {
    let main_window_weak1 = main_window.as_weak();
    main_window.on_cloud_cover_previous(move || {
        debug!("Cloud cover previous button clicked");
        let window_weak = main_window_weak1.clone();
        slint::invoke_from_event_loop(move || {
            debug!("Processing cloud cover previous in event loop");
            let window = window_weak.upgrade();
            if let Some(window) = window {
                debug!("Window upgraded successfully");
                // Use try_lock to avoid blocking the UI thread
                if let Ok(images) = CLOUD_COVER_IMAGES.try_lock() {
                    debug!("CLOUD_COVER_IMAGES lock acquired, {} images", images.len());
                    if let Ok(mut index) = CLOUD_COVER_INDEX.try_lock() {
                        debug!("CLOUD_COVER_INDEX lock acquired, current index: {}", *index);
                        if !images.is_empty() {
                            if *index == 0 {
                                *index = images.len() - 1;
                            } else {
                                *index -= 1;
                            }
                            debug!("New index: {}", *index);
                            // Decode image directly in the UI thread - should be fast for small images
                            let current_image_data = &images[*index];
                            let counter_text = format!("+{}h", *index + 1);
                            match decode_png_to_slint_image_with_black_transparency(current_image_data) {
                                Ok(slint_image) => {
                                    window.set_cloud_cover_image(slint_image);
                                    window.set_cloud_cover_counter(counter_text.into());
                                }
                                Err(e) => {
                                    error!("Failed to decode cloud cover image: {}", e);
                                }
                            }
                        } else {
                            debug!("No cloud cover images available");
                        }
                    } else {
                        debug!("Failed to acquire CLOUD_COVER_INDEX lock");
                    }
                } else {
                    debug!("Failed to acquire CLOUD_COVER_IMAGES lock");
                }
            } else {
                debug!("Failed to upgrade window weak reference");
            }
        }).unwrap();
    });

    let main_window_weak2 = main_window.as_weak();
    main_window.on_cloud_cover_next(move || {
        debug!("Cloud cover next button clicked");
        let window_weak = main_window_weak2.clone();
        slint::invoke_from_event_loop(move || {
            debug!("Processing cloud cover next in event loop");
            let window = window_weak.upgrade();
            if let Some(window) = window {
                debug!("Window upgraded successfully");
                // Use try_lock to avoid blocking the UI thread
                if let Ok(images) = CLOUD_COVER_IMAGES.try_lock() {
                    debug!("CLOUD_COVER_IMAGES lock acquired, {} images", images.len());
                    if let Ok(mut index) = CLOUD_COVER_INDEX.try_lock() {
                        debug!("CLOUD_COVER_INDEX lock acquired, current index: {}", *index);
                        if !images.is_empty() {
                            *index = (*index + 1) % images.len();
                            debug!("New index: {}", *index);
                            // Decode image directly in the UI thread - should be fast for small images
                            let current_image_data = &images[*index];
                            let counter_text = format!("+{}h", *index + 1);
                            match decode_png_to_slint_image_with_black_transparency(current_image_data) {
                                Ok(slint_image) => {
                                    window.set_cloud_cover_image(slint_image);
                                    window.set_cloud_cover_counter(counter_text.into());
                                }
                                Err(e) => {
                                    error!("Failed to decode cloud cover image: {}", e);
                                }
                            }
                        } else {
                            debug!("No cloud cover images available");
                        }
                    } else {
                        debug!("Failed to acquire CLOUD_COVER_INDEX lock");
                    }
                } else {
                    debug!("Failed to acquire CLOUD_COVER_IMAGES lock");
                }
            } else {
                debug!("Failed to upgrade window weak reference");
            }
        }).unwrap();
    });
}

pub async fn fetch_cloud_cover_images(lat: f64, lon: f64) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
    use geomet::{GeoMetAPI, BoundingBox};
    use chrono::{Utc, Duration};

    println!("Fetching cloud cover images...");

    // Calculate current UTC time, round to next hour for forecast
    let now = Utc::now();
    let current_hour = now.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();

    // Bounding box: ~5° radius around coordinates
    let bbox = BoundingBox::new(lon - 12.7, lon + 12.7, lat - 5.0, lat + 5.0);

    // Image dimensions for cloud cover (16:9 ratio for left section)
    let width = 1280;
    let height = 720; // 400 * 9/16 = 225

    let api = GeoMetAPI::new()?;

    // Fetch 24 hours of HRDPS.CONTINENTAL_NT images concurrently using multiple threads
    let mut tasks = Vec::new();

    for hour_offset in 1..=24 {
        let forecast_time = current_hour + Duration::hours(hour_offset);
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
    let mut cloud_images = vec![Vec::new(); 25]; // Index 0 unused, 1-24 for hours
    for task in tasks {
        if let Ok(Some((hour_offset, data))) = task.await {
            cloud_images[hour_offset as usize] = data;
        }
    }

    // Remove empty entries and keep only successful fetches
    let cloud_images: Vec<Vec<u8>> = cloud_images.into_iter().filter(|img| !img.is_empty()).collect();

    println!("Cloud cover images fetched successfully ({} images)", cloud_images.len());
    Ok(cloud_images)
}

pub fn set_cloud_cover_images(main_window: &MainWindow, images: Vec<Vec<u8>>) {
    // Update global storage
    {
        let mut stored_images = CLOUD_COVER_IMAGES.lock().unwrap();
        *stored_images = images;
    }

    // Reset index to 0
    {
        let mut index = CLOUD_COVER_INDEX.lock().unwrap();
        *index = 0;
    }

    // Update UI with first image
    update_cloud_cover_display(main_window);
}

pub fn update_cloud_cover_display(main_window: &MainWindow) {
    let images = CLOUD_COVER_IMAGES.lock().unwrap();
    let index = CLOUD_COVER_INDEX.lock().unwrap();

    if !images.is_empty() && *index < images.len() {
        let current_image_data = &images[*index];
        let counter_text = format!("+{}h", *index + 1);

        debug!("Displaying cloud cover image: {} (index: {})", counter_text, *index);

        // Decode the PNG data to Slint image
        match decode_png_to_slint_image_with_black_transparency(current_image_data) {
            Ok(slint_image) => {
                main_window.set_cloud_cover_image(slint_image);
                main_window.set_cloud_cover_counter(counter_text.into());
            }
            Err(e) => {
                error!("Failed to decode cloud cover image: {}", e);
            }
        }
    } else {
        debug!("No cloud cover images available for display ({} images, index {})", images.len(), *index);
    }
}
