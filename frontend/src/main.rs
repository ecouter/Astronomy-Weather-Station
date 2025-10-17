slint::include_modules!();

use std::time::Duration;
use std::thread;
use std::sync::mpsc;

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
    });

    main_window.set_loading(false);

    // Channel for communication between background thread and UI thread
    let (tx, rx) = mpsc::channel();

    // Spawn background thread for periodic updates
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

    // Handle updates in the UI thread
    let main_window_weak = main_window.as_weak();
    let _update_handle = thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        while let Ok(()) = rx.recv() {
            let main_window = main_window_weak.upgrade();
            if let Some(window) = main_window {
                rt.block_on(async {
                    if let Err(e) = update_weather_images(&window).await {
                        eprintln!("Failed to update images: {}", e);
                        window.set_error_message(format!("Failed to update images: {}", e).into());
                    }
                });
            } else {
                // Window has been destroyed
                break;
            }
        }
    });

    println!("Weather station frontend started successfully");
    main_window.run()
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
    let bbox = BoundingBox::new(lon - 5.0, lon + 5.0, lat - 5.0, lat + 5.0);

    // Image dimensions for 16:9 ratio
    let width = 320;
    let height = 180;

    let api = GeoMetAPI::new()?;

    // Fetch images concurrently
    let (top_left_data, top_right_data, bottom_left_data, bottom_right_data, legend_data) = tokio::try_join!(
        api.get_wms_image("GOES-East_1km_VisibleIRSandwich-NightMicrophysicsIR", &goes_time_str, bbox.clone(), width, height),
        api.get_wms_image("GOES-East_2km_NightMicrophysics", &goes_time_str, bbox.clone(), width, height),
        api.get_wms_image("GOES-East_1km_NaturalColor", &goes_time_str, bbox.clone(), width, height),
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
    let blended_bottom_right = blend_images(&bottom_right_data, &bottom_left_data, 0.8, 0.2)?;

    // Update UI
    main_window.set_top_left_image(top_left_image);
    main_window.set_top_right_image(top_right_image);
    main_window.set_bottom_left_image(bottom_left_image);
    main_window.set_bottom_right_image(blended_bottom_right);
    main_window.set_legend_image(legend_image);

    // Clear any previous error
    main_window.set_error_message("".into());

    println!("Weather images updated successfully");
    Ok(())
}
