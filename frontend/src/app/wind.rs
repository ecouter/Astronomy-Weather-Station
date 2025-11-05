use std::sync::Mutex;
use once_cell::sync::Lazy;
use chrono::Timelike;
use slint::ComponentHandle;
use crate::MainWindow;
use crate::app::utils::decode_png_to_slint_image;

pub static WIND_IMAGES: Lazy<Mutex<Vec<Vec<u8>>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static WIND_LEGEND: Lazy<Mutex<Vec<u8>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static WIND_INDEX: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));

pub fn setup_wind_callbacks(main_window: &MainWindow) {
    let main_window_weak3 = main_window.as_weak();
    main_window.on_wind_previous(move || {
        debug!("Wind previous button clicked");
        let window_weak = main_window_weak3.clone();
        slint::invoke_from_event_loop(move || {
            debug!("Processing wind previous in event loop");
            let window = window_weak.upgrade();
            if let Some(window) = window {
                debug!("Window upgraded successfully");
                // Use try_lock to avoid blocking the UI thread
                if let Ok(images) = WIND_IMAGES.try_lock() {
                    debug!("WIND_IMAGES lock acquired, {} images", images.len());
                    if let Ok(mut index) = WIND_INDEX.try_lock() {
                        debug!("WIND_INDEX lock acquired, current index: {}", *index);
                        if !images.is_empty() {
                            if *index == 0 {
                                *index = images.len() - 1;
                            } else {
                                *index -= 1;
                            }
                            debug!("New index: {}", *index);
                            // Decode image directly in the UI thread - should be fast for small images
                            let current_image_data = &images[*index];
                            let legend_data = {
                                let legend = WIND_LEGEND.lock().unwrap();
                                legend.clone()
                            };
                            let pressures = vec![100, 200, 300, 400, 500, 600, 700, 800, 900, 925, 950, 970, 985, 1000, 1015];
                            let pressure = pressures[*index];
                            let counter_text = format!("{} mb", pressure);
                            match decode_png_to_slint_image(current_image_data) {
                                Ok(slint_image) => {
                                    window.set_wind_image(slint_image);
                                    window.set_wind_counter(counter_text.into());
                                    if !legend_data.is_empty() {
                                        match decode_png_to_slint_image(&legend_data) {
                                            Ok(legend_image) => {
                                                window.set_wind_legend_image(legend_image);
                                            }
                                            Err(e) => {
                                                error!("Failed to decode wind legend: {}", e);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to decode wind image: {}", e);
                                }
                            }
                        } else {
                            debug!("No wind images available");
                        }
                    } else {
                        debug!("Failed to acquire WIND_INDEX lock");
                    }
                } else {
                    debug!("Failed to acquire WIND_IMAGES lock");
                }
            } else {
                debug!("Failed to upgrade window weak reference");
            }
        }).unwrap();
    });

    let main_window_weak4 = main_window.as_weak();
    main_window.on_wind_next(move || {
        debug!("Wind next button clicked");
        let window_weak = main_window_weak4.clone();
        slint::invoke_from_event_loop(move || {
            debug!("Processing wind next in event loop");
            let window = window_weak.upgrade();
            if let Some(window) = window {
                debug!("Window upgraded successfully");
                // Use try_lock to avoid blocking the UI thread
                if let Ok(images) = WIND_IMAGES.try_lock() {
                    debug!("WIND_IMAGES lock acquired, {} images", images.len());
                    if let Ok(mut index) = WIND_INDEX.try_lock() {
                        debug!("WIND_INDEX lock acquired, current index: {}", *index);
                        if !images.is_empty() {
                            *index = (*index + 1) % images.len();
                            debug!("New index: {}", *index);
                            // Decode image directly in the UI thread - should be fast for small images
                            let current_image_data = &images[*index];
                            let legend_data = {
                                let legend = WIND_LEGEND.lock().unwrap();
                                legend.clone()
                            };
                            let pressures = vec![100, 200, 300, 400, 500, 600, 700, 800, 900, 925, 950, 970, 985, 1000, 1015];
                            let pressure = pressures[*index];
                            let counter_text = format!("{} mb", pressure);
                            match decode_png_to_slint_image(current_image_data) {
                                Ok(slint_image) => {
                                    window.set_wind_image(slint_image);
                                    window.set_wind_counter(counter_text.into());
                                    if !legend_data.is_empty() {
                                        match decode_png_to_slint_image(&legend_data) {
                                            Ok(legend_image) => {
                                                window.set_wind_legend_image(legend_image);
                                            }
                                            Err(e) => {
                                                error!("Failed to decode wind legend: {}", e);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to decode wind image: {}", e);
                                }
                            }
                        } else {
                            debug!("No wind images available");
                        }
                    } else {
                        debug!("Failed to acquire WIND_INDEX lock");
                    }
                } else {
                    debug!("Failed to acquire WIND_IMAGES lock");
                }
            } else {
                debug!("Failed to upgrade window weak reference");
            }
        }).unwrap();
    });
}

pub async fn update_wind_images(main_window: &MainWindow) -> Result<(), Box<dyn std::error::Error>> {
    use geomet::{GeoMetAPI, BoundingBox};
    use chrono::{Utc, Duration};

    println!("Updating wind images...");

    // Load coordinates
    let (lat, lon) = crate::app::coordinates::load_coordinates(main_window)?;

    // Calculate current UTC time, round to nearest hour for forecast
    let now = Utc::now();
    let hrdps_time = now.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
    let hrdps_time_str = hrdps_time.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // Bounding box: ~5° radius around coordinates
    let bbox = BoundingBox::new(lon - 12.7, lon + 12.7, lat - 5.0, lat + 5.0);

    // Image dimensions for wind (16:9 ratio)
    let width = 1280;
    let height = 720;

    let api = GeoMetAPI::new()?;

    // Pressure levels
    let pressures = vec![100, 200, 300, 400, 500, 600, 700, 800, 900, 925, 950, 970, 985, 1000, 1015];

    // Fetch wind images concurrently
    let mut image_tasks = Vec::new();
    for &pressure in &pressures {
        let time_str = hrdps_time_str.clone();
        let bbox_clone = bbox.clone();
        let layer_name = format!("HRDPS.CONTINENTAL.PRES_UU.{}", pressure);
        let task = tokio::spawn(async move {
            let api_instance = GeoMetAPI::new().unwrap();
            match api_instance.get_wms_image(&layer_name, &time_str, bbox_clone, width, height).await {
                Ok(data) => Some(data),
                Err(e) => {
                    eprintln!("Failed to fetch wind image for {} mb: {}", pressure, e);
                    None
                }
            }
        });
        image_tasks.push(task);
    }

    // Fetch single legend
    let legend_task = tokio::spawn(async move {
        let api_instance = GeoMetAPI::new().unwrap();
        match api_instance.get_legend_graphic("HRDPS.CONTINENTAL.PRES_UU.1000", Some("WINDARROWKMH"), "image/png", Some("en")).await {
            Ok(data) => Some(data),
            Err(e) => {
                eprintln!("Failed to fetch wind legend: {}", e);
                None
            }
        }
    });

    // Wait for all image tasks
    let mut wind_images = Vec::new();
    for task in image_tasks {
        if let Ok(Some(data)) = task.await {
            wind_images.push(data);
        }
    }

    // Wait for legend task
    let legend_data = if let Ok(Some(data)) = legend_task.await {
        Some(data)
    } else {
        None
    };

    // Update global storage
    {
        let mut images = WIND_IMAGES.lock().unwrap();
        *images = wind_images;
    }
    if let Some(legend) = legend_data {
        let mut legend_store = WIND_LEGEND.lock().unwrap();
        *legend_store = legend;
    }

    // Reset index to 0
    {
        let mut index = WIND_INDEX.lock().unwrap();
        *index = 0;
    }

    // Update UI with first image
    update_wind_display(main_window);

    println!("Wind images updated successfully ({} images, {} legend)", WIND_IMAGES.lock().unwrap().len(), if WIND_LEGEND.lock().unwrap().is_empty() { 0 } else { 1 });
    Ok(())
}

pub fn update_wind_display(main_window: &MainWindow) {
    let images = WIND_IMAGES.lock().unwrap();
    let legend = WIND_LEGEND.lock().unwrap();
    let index = WIND_INDEX.lock().unwrap();

    if !images.is_empty() && *index < images.len() {
        let current_image_data = &images[*index];
        let pressures = vec![100, 200, 300, 400, 500, 600, 700, 800, 900, 925, 950, 970, 985, 1000, 1015];
        let pressure = pressures[*index];
        let counter_text = format!("{} mb", pressure);

        debug!("Displaying wind image: {} (index: {})", counter_text, *index);

        // Decode the PNG data to Slint image
        match decode_png_to_slint_image(current_image_data) {
            Ok(slint_image) => {
                main_window.set_wind_image(slint_image);
                main_window.set_wind_counter(counter_text.into());
                if !legend.is_empty() {
                    match decode_png_to_slint_image(&legend) {
                        Ok(legend_image) => {
                            main_window.set_wind_legend_image(legend_image);
                        }
                        Err(e) => {
                            error!("Failed to decode wind legend: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to decode wind image: {}", e);
            }
        }
    } else {
        debug!("No wind images available for display ({} images, index {})", images.len(), *index);
    }
}
