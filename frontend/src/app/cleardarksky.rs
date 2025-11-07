use crate::MainWindow;
use crate::app::coordinates::load_coordinates;
use crate::app::utils::decode_gif_to_slint_image;

pub async fn fetch_cleardarksky_image(lat: f64, lon: f64) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use cleardarksky::ClearDarkSkyAPI;

    println!("Fetching ClearDarkSky image...");

    // Create API client
    let api = ClearDarkSkyAPI::new();

    // Fetch nearest sky chart location
    let location = api.fetch_nearest_sky_chart_location(lat, lon).await?;
    println!("Fetched ClearDarkSky location: {}", location);

    // Fetch GIF data
    let gif_data = api.fetch_clear_sky_chart_bytes(&location).await?;
    println!("Fetched ClearDarkSky GIF data ({} bytes)", gif_data.len());

    Ok(gif_data)
}

pub fn set_cleardarksky_image(main_window: &MainWindow, image_data: Vec<u8>) {
    match decode_gif_to_slint_image(&image_data) {
        Ok(image) => main_window.set_cleardarksky_image(image),
        Err(e) => error!("Failed to decode ClearDarkSky image: {}", e),
    }
}
