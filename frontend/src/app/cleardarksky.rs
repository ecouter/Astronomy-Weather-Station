use crate::MainWindow;
use crate::app::coordinates::load_coordinates;
use crate::app::utils::decode_gif_to_slint_image;

pub async fn load_cleardarksky_image(main_window: &MainWindow) -> Result<slint::Image, Box<dyn std::error::Error>> {
    use cleardarksky::ClearDarkSkyAPI;

    println!("Loading ClearDarkSky image...");

    // Load coordinates - this will show popup if file not found
    let (lat, lon) = load_coordinates(main_window)?;

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
