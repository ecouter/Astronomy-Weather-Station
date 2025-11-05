use crate::MainWindow;
use crate::app::coordinates::load_coordinates;

pub async fn load_map_image(main_window: &MainWindow) -> Result<slint::Image, Box<dyn std::error::Error>> {
    use openstreetmap::OpenStreetMapAPI;

    println!("Loading map image...");

    // Load coordinates
    let (lat, lon) = load_coordinates(main_window)?;

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
    let bbox = (lat - 5.0, lon - 12.7, lat + 5.0, lon + 12.7);

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
