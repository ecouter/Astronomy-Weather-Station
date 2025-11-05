use slint::Image;
use crate::app::coordinates::load_coordinates;
use crate::app::utils::{decode_png_to_slint_image, blend_images};

pub async fn update_weather_images(main_window: &crate::MainWindow) -> Result<(), Box<dyn std::error::Error>> {
    use geomet::{GeoMetAPI, BoundingBox};
    use chrono::{Utc, Duration, Timelike};
    use std::sync::Mutex;
    use once_cell::sync::Lazy;

    println!("Updating weather images...");

    // Load coordinates
    let (lat, lon) = load_coordinates(main_window)?;

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
    let bbox = BoundingBox::new(lon - 12.7, lon + 12.7, lat - 5.0, lat + 5.0);

    // Image dimensions for 16:9 ratio
    let width = 1280;
    let height = 720;

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
