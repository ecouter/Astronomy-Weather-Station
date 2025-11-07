use slint::Image;
use crate::app::utils::{decode_png_to_slint_image, blend_images};

#[derive(Clone)]
pub struct WeatherData {
    pub top_left: Vec<u8>,
    pub top_right: Vec<u8>,
    pub bottom_left: Vec<u8>,
    pub bottom_right: Vec<u8>,
    pub legend: Vec<u8>,
}

pub async fn fetch_weather_images(lat: f64, lon: f64) -> Result<WeatherData, Box<dyn std::error::Error>> {
    use geomet::{GeoMetAPI, BoundingBox};
    use chrono::{Utc, Duration, Timelike};

    println!("Fetching weather images...");

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

    println!("Weather images fetched successfully");

    Ok(WeatherData {
        top_left: top_left_data,
        top_right: top_right_data,
        bottom_left: bottom_left_data,
        bottom_right: bottom_right_data,
        legend: legend_data,
    })
}

pub fn set_weather_images(main_window: &crate::MainWindow, data: WeatherData) {
    if let Ok(top_left) = decode_png_to_slint_image(&data.top_left) {
        main_window.set_top_left_image(top_left);
    }
    if let Ok(top_right) = decode_png_to_slint_image(&data.top_right) {
        main_window.set_top_right_image(top_right);
    }
    if let Ok(bottom_left) = decode_png_to_slint_image(&data.bottom_left) {
        main_window.set_bottom_left_image(bottom_left);
    }
    if let Ok(bottom_right) = decode_png_to_slint_image(&data.bottom_right) {
        main_window.set_bottom_right_image(bottom_right);
    }
    if let Ok(legend) = decode_png_to_slint_image(&data.legend) {
        main_window.set_legend_image(legend);
    }
    main_window.set_error_message("".into());
}
