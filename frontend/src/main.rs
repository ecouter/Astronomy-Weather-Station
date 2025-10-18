slint::include_modules!();

use std::time::Duration;
use std::thread;
use std::sync::mpsc;
use std::sync::Mutex;
use std::rc::Rc;
use once_cell::sync::Lazy;
use chrono::Timelike;

fn main() -> Result<(), slint::PlatformError> {
    println!("Starting weather station frontend...");

    let main_window = MainWindow::new()?;

    // Start the async runtime for image fetching
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Initial image load
    rt.block_on(async {
        if let Err(e) = update_weather_images(&main_window).await {
            eprintln!("Failed to load initial images: {}", e);
            main_window.set_error_message(format!("Failed to load images: {}", e).into());
        }
        if let Err(e) = update_cloud_cover_images(&main_window).await {
            eprintln!("Failed to load initial cloud cover images: {}", e);
            main_window.set_error_message(format!("Failed to load cloud cover images: {}", e).into());
        }
        match load_map_image().await {
            Ok(map_image) => {
                main_window.set_map_image(map_image);
            }
            Err(e) => {
                eprintln!("Failed to load map image: {}", e);
                main_window.set_error_message(format!("Failed to load map image: {}", e).into());
            }
        }
        match load_cleardarksky_image().await {
            Ok(cleardarksky_image) => {
                main_window.set_cleardarksky_image(cleardarksky_image);
            }
            Err(e) => {
                eprintln!("Failed to load ClearDarkSky image: {}", e);
                main_window.set_error_message(format!("Failed to load ClearDarkSky image: {}", e).into());
            }
        }
    });

    main_window.set_loading(false);

    // Channel for communication between background thread and UI thread
    let (tx, rx) = mpsc::channel();
    // Channel for cloud cover cycling
    let (cloud_tx, cloud_rx) = mpsc::channel();

    // Spawn background thread for periodic weather updates
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut interval = tokio::time::interval(Duration::from_secs(600)); // 10 minutes
            loop {
                interval.tick().await;
                // Signal the UI thread to update images
                if tx.send(()).is_err() {
                    // UI thread has shut down
                    break;
                }
            }
        });
    });

    // Spawn background thread for cloud cover updates (hourly)
    let cloud_tx_clone = cloud_tx.clone();
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // 1 hour
            loop {
                interval.tick().await;
                // Signal the UI thread to update cloud cover images
                if cloud_tx_clone.send("update").is_err() {
                    // UI thread has shut down
                    break;
                }
            }
        });
    });

    // Spawn background thread for ClearOutside data updates (hourly)
    let clearoutside_tx = cloud_tx.clone();
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // 1 hour
            loop {
                interval.tick().await;
                // Signal the UI thread to update ClearOutside data
                if clearoutside_tx.send("clearoutside_update").is_err() {
                    // UI thread has shut down
                    break;
                }
            }
        });
    });

    // Spawn background thread for cloud cover cycling (every 2 seconds)
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        println!("Cloud cover cycling thread started");
        rt.block_on(async {
            let mut interval = tokio::time::interval(Duration::from_secs(2)); // 2 seconds
            loop {
                interval.tick().await;
                println!("Sending cycle signal");
                // Signal the UI thread to cycle cloud cover display
                if cloud_tx.send("cycle").is_err() {
                    println!("Failed to send cycle signal, UI thread may have shut down");
                    // UI thread has shut down
                    break;
                }
            }
        });
        println!("Cloud cover cycling thread ended");
    });

    // Handle cloud cover updates directly in the main thread using invoke_from_event_loop
    let main_window_weak = main_window.as_weak();
    let _cloud_update_handle = thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        println!("Cloud cover update thread started");
        while let Ok(signal) = cloud_rx.recv() {
            println!("Received cloud signal: {}", signal);
            let window_weak = main_window_weak.clone();
            match signal {
                "update" => {
                    println!("Processing cloud update signal");
                    // Use invoke_from_event_loop to run async code in the UI thread
                    slint::invoke_from_event_loop(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let window = window_weak.upgrade();
                        if let Some(window) = window {
                            rt.block_on(async {
                                if let Err(e) = update_cloud_cover_images(&window).await {
                                    eprintln!("Failed to update cloud cover images: {}", e);
                                    window.set_error_message(format!("Failed to update cloud cover images: {}", e).into());
                                }
                                // Also update ClearDarkSky chart
                                match load_cleardarksky_image().await {
                                    Ok(cleardarksky_image) => {
                                        window.set_cleardarksky_image(cleardarksky_image);
                                        println!("Updated ClearDarkSky chart");
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to update ClearDarkSky image: {}", e);
                                        window.set_error_message(format!("Failed to update ClearDarkSky image: {}", e).into());
                                    }
                                }
                            });
                        }
                    }).unwrap();
                }
                "clearoutside_update" => {
                    println!("Processing ClearOutside update signal");
                    // Use invoke_from_event_loop to run async code in the UI thread
                    slint::invoke_from_event_loop(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let window = window_weak.upgrade();
                        if let Some(window) = window {
                            rt.block_on(async {
                                if let Err(e) = update_clearoutside_data(&window).await {
                                    eprintln!("Failed to update ClearOutside data: {}", e);
                                    window.set_error_message(format!("Failed to update ClearOutside data: {}", e).into());
                                }
                            });
                        }
                    }).unwrap();
                }
                "cycle" => {
                    println!("Processing cloud cycle signal");
                    // Use invoke_from_event_loop to update UI in the UI thread
                    slint::invoke_from_event_loop(move || {
                        let window = window_weak.upgrade();
                        if let Some(window) = window {
                            update_cloud_cover_display(&window);
                        }
                    }).unwrap();
                }
                _ => {
                    println!("Unknown cloud signal: {}", signal);
                }
            }
        }
        println!("Cloud cover update thread ended");
    });

    // Keep the main window alive by storing it
    let _main_window_handle = main_window.as_weak();

    // Handle weather updates in the UI thread
    let main_window_weak2 = main_window.as_weak();
    let _weather_update_handle = thread::spawn(move || {
        while let Ok(()) = rx.recv() {
            let window_weak = main_window_weak2.clone();
            // Use invoke_from_event_loop to run async code in the UI thread
            slint::invoke_from_event_loop(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let window = window_weak.upgrade();
                if let Some(window) = window {
                    rt.block_on(async {
                        if let Err(e) = update_weather_images(&window).await {
                            eprintln!("Failed to update images: {}", e);
                            window.set_error_message(format!("Failed to update images: {}", e).into());
                        }
                    });
                }
            }).unwrap();
        }
    });

    println!("Weather station frontend started successfully");

    // Run the main window - this blocks until the window is closed
    let result = main_window.run();

    println!("Main window closed, shutting down threads...");
    result
}

fn decode_png_to_slint_image(png_data: &[u8]) -> Result<slint::Image, Box<dyn std::error::Error>> {
    use image::ImageFormat;

    // Decode the PNG data
    let img = image::load_from_memory_with_format(png_data, ImageFormat::Png)?;

    // Convert to RGBA8 format
    let rgba_img = img.to_rgba8();

    // Get dimensions
    let width = rgba_img.width() as u32;
    let height = rgba_img.height() as u32;

    // Convert to raw pixel data (RGBA format)
    let raw_pixels: Vec<u8> = rgba_img.into_raw();

    // Create Slint image from the pixel buffer (RGBA format)
    let pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&raw_pixels, width, height);
    Ok(slint::Image::from_rgba8(pixel_buffer))
}

async fn update_clearoutside_data(main_window: &MainWindow) -> Result<(), Box<dyn std::error::Error>> {
    use clearoutside::ClearOutsideAPI;

    // Load coordinates
    let coords_content = std::fs::read_to_string("../coordinates.json")?;
    let coords: serde_json::Value = serde_json::from_str(&coords_content)?;
    let lat: f64 = coords["lat"].as_str().unwrap().parse()?;
    let lon: f64 = coords["lon"].as_str().unwrap().parse()?;

    // Format coordinates for ClearOutside (lat.lon with 2 decimals)
    let lat_str = format!("{:.2}", lat);
    let lon_str = format!("{:.2}", lon);

    // Create API instance
    let api = ClearOutsideAPI::new(&lat_str, &lon_str, Some("midnight")).await?;

    // Fetch and parse data
    let forecast = api.pull()?;

    // Process data for night hours display
    let night_conditions = process_clearoutside_data(&forecast)?;
    update_clearoutside_display(main_window, night_conditions);

    Ok(())
}

fn process_clearoutside_data(forecast: &clearoutside::ClearOutsideForecast) -> Result<Vec<NightCondition>, Box<dyn std::error::Error>> {
    let mut night_conditions = Vec::new();

    // Sort days by key (day-0, day-1, etc.)
    let mut sorted_days: Vec<_> = forecast.forecast.iter().collect();
    sorted_days.sort_by_key(|(k, _)| *k);

    for (i, (day_key, day_info)) in sorted_days.iter().enumerate() {
        // Parse sunset time for current day
        let sunset_hour = parse_hour(&day_info.sun.set)?;

        // Get hours after sunset from current day
        let evening_hours: Vec<_> = day_info.hours.iter()
            .filter(|(hour_str, _)| {
                if let Ok(hour) = hour_str.parse::<u32>() {
                    hour > sunset_hour
                } else {
                    false
                }
            })
            .collect();

        // Add evening hours from current day
        for (hour_str, hourly_data) in &evening_hours {
            night_conditions.push(NightCondition {
                day: format!("night-{}", i), // Use night-X instead of day-X
                hour: hour_str.parse().unwrap_or(0),
                condition: hourly_data.conditions.clone(),
                total_clouds: hourly_data.total_clouds.parse().unwrap_or(0),
                is_evening: true, // Evening hours (after sunset)
            });
        }

        // Get hours before sunrise from next day (if available)
        if i + 1 < sorted_days.len() {
            let next_day_info = &sorted_days[i + 1].1;
            let next_sunrise_hour = parse_hour(&next_day_info.sun.rise)?;

            let morning_hours: Vec<_> = next_day_info.hours.iter()
                .filter(|(hour_str, _)| {
                    if let Ok(hour) = hour_str.parse::<u32>() {
                        hour < next_sunrise_hour && hour <= 11 // Only morning hours 0-11
                    } else {
                        false
                    }
                })
                .collect();

            // Add morning hours from next day, but associate with current night
            for (hour_str, hourly_data) in &morning_hours {
                night_conditions.push(NightCondition {
                    day: format!("night-{}", i), // Associate with current night
                    hour: hour_str.parse().unwrap_or(0),
                    condition: hourly_data.conditions.clone(),
                    total_clouds: hourly_data.total_clouds.parse().unwrap_or(0),
                    is_evening: false, // Morning hours (before sunrise)
                });
            }
        }
    }

    // Sort all night conditions by night, then by evening/morning, then by hour
    night_conditions.sort_by(|a, b| {
        let night_cmp = a.day.cmp(&b.day);
        if night_cmp == std::cmp::Ordering::Equal {
            // Within the same night, evening hours come before morning hours
            match (a.is_evening, b.is_evening) {
                (true, false) => std::cmp::Ordering::Less,   // evening before morning
                (false, true) => std::cmp::Ordering::Greater, // morning after evening
                _ => a.hour.cmp(&b.hour), // same type (both evening or both morning), sort by hour
            }
        } else {
            night_cmp
        }
    });

    Ok(night_conditions)
}

fn parse_hour(time_str: &str) -> Result<u32, Box<dyn std::error::Error>> {
    // Parse time like "18:11" to get hour
    let hour_str = time_str.split(':').next().ok_or("Invalid time format")?;
    Ok(hour_str.parse()?)
}

#[derive(Clone)]
struct NightCondition {
    day: String,
    hour: u32,
    condition: String,
    total_clouds: u32,
    is_evening: bool, // true for evening hours (after sunset), false for morning hours (before sunrise)
}

fn update_clearoutside_display(main_window: &MainWindow, conditions: Vec<NightCondition>) {
    println!("Night conditions: {:?}", conditions.len());
    for condition in &conditions {
        println!("{} {}: {} - {}", condition.day, condition.hour, condition.condition, condition.total_clouds);
    }

    // Group conditions by day
    use std::collections::HashMap;
    let mut conditions_by_day: HashMap<String, Vec<&NightCondition>> = HashMap::new();

    for condition in &conditions {
        conditions_by_day.entry(condition.day.clone()).or_insert(Vec::new()).push(condition);
    }

    // Sort days
    let mut sorted_days: Vec<_> = conditions_by_day.keys().collect();
    sorted_days.sort();

    // Set data for each day
    for (day_idx, day_key) in sorted_days.iter().enumerate() {
        if let Some(day_conditions) = conditions_by_day.get(*day_key) {
            // Conditions are already sorted by the global sorting logic
            let conditions_vec: Vec<slint::SharedString> = day_conditions.iter()
                .map(|c| c.condition.clone().into())
                .collect();
            let clouds_vec: Vec<i32> = day_conditions.iter()
                .map(|c| c.total_clouds as i32)
                .collect();
            let hours_vec: Vec<i32> = day_conditions.iter()
                .map(|c| c.hour as i32)
                .collect();

            match day_idx {
                0 => {
                    main_window.set_day0_conditions(Rc::new(slint::VecModel::from(conditions_vec)).into());
                    main_window.set_day0_clouds(Rc::new(slint::VecModel::from(clouds_vec)).into());
                    main_window.set_day0_hours(Rc::new(slint::VecModel::from(hours_vec)).into());
                }
                1 => {
                    main_window.set_day1_conditions(Rc::new(slint::VecModel::from(conditions_vec)).into());
                    main_window.set_day1_clouds(Rc::new(slint::VecModel::from(clouds_vec)).into());
                    main_window.set_day1_hours(Rc::new(slint::VecModel::from(hours_vec)).into());
                }
                2 => {
                    main_window.set_day2_conditions(Rc::new(slint::VecModel::from(conditions_vec)).into());
                    main_window.set_day2_clouds(Rc::new(slint::VecModel::from(clouds_vec)).into());
                    main_window.set_day2_hours(Rc::new(slint::VecModel::from(hours_vec)).into());
                }
                3 => {
                    main_window.set_day3_conditions(Rc::new(slint::VecModel::from(conditions_vec)).into());
                    main_window.set_day3_clouds(Rc::new(slint::VecModel::from(clouds_vec)).into());
                    main_window.set_day3_hours(Rc::new(slint::VecModel::from(hours_vec)).into());
                }
                4 => {
                    main_window.set_day4_conditions(Rc::new(slint::VecModel::from(conditions_vec)).into());
                    main_window.set_day4_clouds(Rc::new(slint::VecModel::from(clouds_vec)).into());
                    main_window.set_day4_hours(Rc::new(slint::VecModel::from(hours_vec)).into());
                }
                5 => {
                    main_window.set_day5_conditions(Rc::new(slint::VecModel::from(conditions_vec)).into());
                    main_window.set_day5_clouds(Rc::new(slint::VecModel::from(clouds_vec)).into());
                    main_window.set_day5_hours(Rc::new(slint::VecModel::from(hours_vec)).into());
                }
                6 => {
                    main_window.set_day6_conditions(Rc::new(slint::VecModel::from(conditions_vec)).into());
                    main_window.set_day6_clouds(Rc::new(slint::VecModel::from(clouds_vec)).into());
                    main_window.set_day6_hours(Rc::new(slint::VecModel::from(hours_vec)).into());
                }
                _ => {}
            }
        }
    }
}

fn decode_gif_to_slint_image(gif_data: &[u8]) -> Result<slint::Image, Box<dyn std::error::Error>> {
    use std::io::Cursor;

    // Decode the GIF data
    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = decoder.read_info(Cursor::new(gif_data))?;

    // Read the first frame
    if let Some(frame) = decoder.read_next_frame()? {
        // Get dimensions
        let width = frame.width as u32;
        let height = frame.height as u32;

        // The frame buffer contains RGBA data
        let raw_pixels = frame.buffer.clone();

        // Create Slint image from the pixel buffer (RGBA format)
        let pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&raw_pixels, width, height);
        Ok(slint::Image::from_rgba8(pixel_buffer))
    } else {
        Err("No frames in GIF".into())
    }
}

async fn load_cleardarksky_image() -> Result<slint::Image, Box<dyn std::error::Error>> {
    use cleardarksky::ClearDarkSkyAPI;

    println!("Loading ClearDarkSky image...");

    // Load coordinates
    let coords_content = std::fs::read_to_string("../coordinates.json")?;
    let coords: serde_json::Value = serde_json::from_str(&coords_content)?;
    let lat: f64 = coords["lat"].as_str().unwrap().parse()?;
    let lon: f64 = coords["lon"].as_str().unwrap().parse()?;

    // Create API client
    let api = ClearDarkSkyAPI::new();

    // Fetch nearest sky chart location
    let location = api.fetch_nearest_sky_chart_location(lat, lon).await?;
    println!("Fetched ClearDarkSky location: {}", location);

    // Fetch GIF data
    let gif_data = api.fetch_clear_sky_chart_bytes(&location).await?;
    println!("Fetched ClearDarkSky GIF data ({} bytes)", gif_data.len());

    // Decode the GIF to Slint image
    decode_gif_to_slint_image(&gif_data)
}

fn blend_images(image1_data: &[u8], image2_data: &[u8], weight1: f32, weight2: f32) -> Result<slint::Image, Box<dyn std::error::Error>> {
    use image::ImageFormat;

    // Decode both PNG images
    let img1 = image::load_from_memory_with_format(image1_data, ImageFormat::Png)?;
    let img2 = image::load_from_memory_with_format(image2_data, ImageFormat::Png)?;

    // Convert to RGBA8 format
    let rgba1 = img1.to_rgba8();
    let rgba2 = img2.to_rgba8();

    // Ensure images have the same dimensions
    let width = rgba1.width();
    let height = rgba1.height();
    if rgba2.width() != width || rgba2.height() != height {
        return Err("Images must have the same dimensions for blending".into());
    }

    // Get raw pixel data
    let pixels1 = rgba1.into_raw();
    let pixels2 = rgba2.into_raw();

    // Create blended pixel data
    let mut blended_pixels = Vec::with_capacity(pixels1.len());

    for (p1, p2) in pixels1.chunks(4).zip(pixels2.chunks(4)) {
        // Blend each RGBA component
        let r = (p1[0] as f32 * weight1 + p2[0] as f32 * weight2) as u8;
        let g = (p1[1] as f32 * weight1 + p2[1] as f32 * weight2) as u8;
        let b = (p1[2] as f32 * weight1 + p2[2] as f32 * weight2) as u8;
        let a = (p1[3] as f32 * weight1 + p2[3] as f32 * weight2) as u8;

        blended_pixels.extend_from_slice(&[r, g, b, a]);
    }

    // Create Slint image from blended pixel buffer
    let pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&blended_pixels, width as u32, height as u32);
    Ok(slint::Image::from_rgba8(pixel_buffer))
}

async fn update_weather_images(main_window: &MainWindow) -> Result<(), Box<dyn std::error::Error>> {
    use geomet::{GeoMetAPI, BoundingBox};
    use chrono::{Utc, Duration, Timelike};
    use slint::Image;
    use std::sync::Mutex;
    use once_cell::sync::Lazy;

    println!("Updating weather images...");

    // Load coordinates
    let coords_content = std::fs::read_to_string("../coordinates.json")?;
    let coords: serde_json::Value = serde_json::from_str(&coords_content)?;
    let lat: f64 = coords["lat"].as_str().unwrap().parse()?;
    let lon: f64 = coords["lon"].as_str().unwrap().parse()?;

    // Calculate current UTC time for different data types
    let now = Utc::now();

    // GOES data: available up to 30 minutes prior, releases every 10 minutes
    let thirty_min_ago = now - Duration::minutes(30);
    let minutes = thirty_min_ago.minute();
    let rounded_minutes = (minutes / 10) * 10;
    let goes_time = thirty_min_ago.with_minute(rounded_minutes).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
    let goes_time_str = goes_time.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // HRDPS data: hourly data, round to nearest hour
    let hrdps_time = thirty_min_ago.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
    let hrdps_time_str = hrdps_time.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // Bounding box: ~5° radius around coordinates
    let bbox = BoundingBox::new(lon - 8.9, lon + 8.9, lat - 5.0, lat + 5.0);

    // Image dimensions for 16:9 ratio
    let width = 320;
    let height = 180;

    let api = GeoMetAPI::new()?;

    // Fetch images concurrently
    let (top_left_data, top_right_data, bottom_left_data, bottom_right_data, legend_data) = tokio::try_join!(
        api.get_wms_image("GOES-East_1km_VisibleIRSandwich-NightMicrophysicsIR", &goes_time_str, bbox.clone(), width, height),
        api.get_wms_image("GOES-East_2km_NightMicrophysics", &goes_time_str, bbox.clone(), width, height),
        api.get_wms_image("GOES-East_1km_DayVis-NightIR", &goes_time_str, bbox.clone(), width, height),
        api.get_wms_image("HRDPS.CONTINENTAL_PN-SLP", &hrdps_time_str, bbox.clone(), width, height),
        api.get_legend_graphic("HRDPS.CONTINENTAL_PN-SLP", Some("PRESSURE4"), "image/png", Some("en"))
    )?;

    // Decode PNG images and convert to Slint format
    let top_left_image = decode_png_to_slint_image(&top_left_data)?;
    let top_right_image = decode_png_to_slint_image(&top_right_data)?;
    let bottom_left_image = decode_png_to_slint_image(&bottom_left_data)?;
    let bottom_right_image = decode_png_to_slint_image(&bottom_right_data)?;
    let legend_image = decode_png_to_slint_image(&legend_data)?;

    // Blend bottom right image: 80% bottom right + 20% bottom left
    //let blended_bottom_right = blend_images(&bottom_right_data, &bottom_left_data, 0.8, 0.2)?;

    // Update UI
    main_window.set_top_left_image(top_left_image);
    main_window.set_top_right_image(top_right_image);
    main_window.set_bottom_left_image(bottom_left_image);
    main_window.set_bottom_right_image(bottom_right_image);
    main_window.set_legend_image(legend_image);

    // Clear any previous error
    main_window.set_error_message("".into());

    println!("Weather images updated successfully");
    Ok(())
}

// Global storage for cloud cover images and current index
static CLOUD_COVER_IMAGES: Lazy<Mutex<Vec<Vec<u8>>>> = Lazy::new(|| Mutex::new(Vec::new()));
static CLOUD_COVER_INDEX: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));

async fn update_cloud_cover_images(main_window: &MainWindow) -> Result<(), Box<dyn std::error::Error>> {
    use geomet::{GeoMetAPI, BoundingBox};
    use chrono::{Utc, Duration};

    println!("Updating cloud cover images...");

    // Load coordinates
    let coords_content = std::fs::read_to_string("../coordinates.json")?;
    let coords: serde_json::Value = serde_json::from_str(&coords_content)?;
    let lat: f64 = coords["lat"].as_str().unwrap().parse()?;
    let lon: f64 = coords["lon"].as_str().unwrap().parse()?;

    // Calculate current UTC time, round to next hour for forecast
    let now = Utc::now();
    let current_hour = now.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();

    // Bounding box: ~5° radius around coordinates
    let bbox = BoundingBox::new(lon - 8.9, lon + 8.9, lat - 5.0, lat + 5.0);

    // Image dimensions for cloud cover (16:9 ratio for left section)
    let width = 400;
    let height = 225; // 400 * 9/16 = 225

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

    // Update global storage
    {
        let mut images = CLOUD_COVER_IMAGES.lock().unwrap();
        *images = cloud_images;
    }

    // Reset index to 0
    {
        let mut index = CLOUD_COVER_INDEX.lock().unwrap();
        *index = 0;
    }

    // Update UI with first image
    update_cloud_cover_display(main_window);

    println!("Cloud cover images updated successfully ({} images)", CLOUD_COVER_IMAGES.lock().unwrap().len());
    Ok(())
}

fn update_cloud_cover_display(main_window: &MainWindow) {
    let images = CLOUD_COVER_IMAGES.lock().unwrap();
    let mut index = CLOUD_COVER_INDEX.lock().unwrap();

    if !images.is_empty() {
        let current_image_data = &images[*index];
        let counter_text = format!("+{}h", *index + 1);

        println!("Displaying cloud cover image: {} (index: {})", counter_text, *index);

        // Decode the PNG data to Slint image
        match decode_png_to_slint_image(current_image_data) {
            Ok(slint_image) => {
                main_window.set_cloud_cover_image(slint_image);
                main_window.set_cloud_cover_counter(counter_text.into());
            }
            Err(e) => {
                eprintln!("Failed to decode cloud cover image: {}", e);
            }
        }

        // Cycle to next image
        *index = (*index + 1) % images.len();
        println!("Next index will be: {}", *index);
    } else {
        println!("No cloud cover images available for display");
    }
}

async fn load_map_image() -> Result<slint::Image, Box<dyn std::error::Error>> {
    use openstreetmap::OpenStreetMapAPI;

    println!("Loading map image...");

    // Load coordinates
    let coords_content = std::fs::read_to_string("../coordinates.json")?;
    let coords: serde_json::Value = serde_json::from_str(&coords_content)?;
    let lat: f64 = coords["lat"].as_str().unwrap().parse()?;
    let lon: f64 = coords["lon"].as_str().unwrap().parse()?;

    // Create filename based on coordinates
    let filename = format!("{}_{}.png", lat, lon);
    let filepath = std::path::Path::new("ui/images/").join(&filename);

    // Check if map already exists
    if filepath.exists() {
        println!("Map file {} already exists, loading from disk", filename);
        let img = image::open(&filepath)?;
        let rgba_img = img.to_rgba8();
        let width = rgba_img.width() as u32;
        let height = rgba_img.height() as u32;
        let raw_pixels: Vec<u8> = rgba_img.into_raw();
        let pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&raw_pixels, width, height);
        return Ok(slint::Image::from_rgba8(pixel_buffer));
    }

    println!("Map file {} does not exist, fetching from OpenStreetMap API", filename);

    // Create API client
    let api = OpenStreetMapAPI::new();

    // Define bounding box around coordinates (~1° x 1°)
    let bbox = (lat - 5.0, lon - 8.9, lat + 5.0, lon + 8.9);

    // Download and save map (400x225 pixels, zoom level 10)
    api.download_and_save_map(bbox, 6, &filepath).await?;

    println!("Map saved to {:?}", filepath);

    // Load the saved image
    let img = image::open(&filepath)?;
    let rgba_img = img.to_rgba8();
    let width = rgba_img.width() as u32;
    let height = rgba_img.height() as u32;
    let raw_pixels: Vec<u8> = rgba_img.into_raw();
    let pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&raw_pixels, width, height);
    Ok(slint::Image::from_rgba8(pixel_buffer))
}
