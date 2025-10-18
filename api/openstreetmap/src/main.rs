use openstreetmap::OpenStreetMapAPI;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("OpenStreetMap Tile Downloader Test");

    let api = OpenStreetMapAPI::new();

    // Example bounding box (lat_min, lon_min, lat_max, lon_max)
    let bbox = (45.5 - 5.0, -74.5 + 8.9, 45.5 + 5.0, -75.5 - 8.9); // Montreal Large area
    let zoom = 6;

    println!("Testing with bounding box: {:?}", bbox);
    println!("Zoom level: {}", zoom);

    // Download and save the map
    let output_path = Path::new("output.png");
    match api.download_and_save_map(bbox, zoom, output_path).await {
        Ok(_) => {
            println!("Successfully downloaded and saved map!");
        }
        Err(e) => {
            println!("Error downloading map: {}", e);
        }
    }

    Ok(())
}
