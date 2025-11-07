use crate::MainWindow;
use crate::app::coordinates::load_coordinates;

pub async fn fetch_map_image(lat: f64, lon: f64) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use openstreetmap::OpenStreetMapAPI;

    println!("Fetching map image...");

    // Create filename based on coordinates
    let filename = format!("{}_{}.png", lat, lon);
    let filepath = std::path::Path::new("ui/images/").join(&filename);

    // Check if map already exists
    if filepath.exists() {
        println!("Map file {} already exists, loading from disk", filename);
        return Ok(std::fs::read(&filepath)?);
    }

    println!("Map file {} does not exist, fetching from OpenStreetMap API", filename);

    // Create API client
    let api = OpenStreetMapAPI::new();

    // Define bounding box around coordinates (~1° x 1°)
    let bbox = (lat - 5.0, lon - 12.7, lat + 5.0, lon + 12.7);

    // Download and save map (400x225 pixels, zoom level 10)
    api.download_and_save_map(bbox, 6, &filepath).await?;

    println!("Map saved to {:?}", filepath);

    // Read the saved PNG file
    Ok(std::fs::read(&filepath)?)
}

pub fn set_map_image(main_window: &MainWindow, image_data: Vec<u8>) {
    match crate::app::utils::decode_png_to_slint_image(&image_data) {
        Ok(image) => main_window.set_map_image(image),
        Err(e) => error!("Failed to decode map image: {}", e),
    }
}
