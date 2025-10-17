use std::process::Command;

fn main() {
    // Create placeholder images first so SLINT can compile
    create_placeholder_images();

    // Compile the SLINT UI
    slint_build::compile("ui/main.slint").unwrap();

    // Fetch real weather data and update images
    if let Err(e) = fetch_and_save_weather_data() {
        eprintln!("Warning: Failed to fetch weather data during build: {}", e);
        // Continue with placeholder images
    }
}

fn create_placeholder_images() {
    let images_dir = "ui/images";
    std::fs::create_dir_all(images_dir).unwrap();

    // Create 320x180 placeholder images (light gray)
    let placeholder = create_placeholder_png(320, 180);

    std::fs::write("ui/images/top_left.png", &placeholder).unwrap();
    std::fs::write("ui/images/top_right.png", &placeholder).unwrap();
    std::fs::write("ui/images/bottom_left.png", &placeholder).unwrap();
    std::fs::write("ui/images/bottom_right.png", &placeholder).unwrap();
    std::fs::write("ui/images/legend.png", &placeholder).unwrap();

    println!("Created placeholder images");
}

fn create_placeholder_png(width: u32, height: u32) -> Vec<u8> {
    use std::io::Cursor;

    // Create a simple PNG with the specified dimensions
    // For simplicity, we'll create a minimal valid PNG
    let mut png_data = Vec::new();

    // PNG signature
    png_data.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

    // IHDR chunk
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());  // width
    ihdr.extend_from_slice(&height.to_be_bytes()); // height
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // bit depth, color type, compression, filter, interlace

    let ihdr_crc = crc32fast::hash(&ihdr);
    png_data.extend_from_slice(&(13u32.to_be_bytes())); // chunk length
    png_data.extend_from_slice(b"IHDR");
    png_data.extend_from_slice(&ihdr);
    png_data.extend_from_slice(&ihdr_crc.to_be_bytes());

    // IEND chunk
    png_data.extend_from_slice(&(0u32.to_be_bytes())); // chunk length
    png_data.extend_from_slice(b"IEND");
    let iend_crc = crc32fast::hash(b"");
    png_data.extend_from_slice(&iend_crc.to_be_bytes());

    png_data
}

fn fetch_and_save_weather_data() -> Result<(), Box<dyn std::error::Error>> {
    use geomet::{GeoMetAPI, BoundingBox};
    use chrono::{Utc, Duration, Timelike};

    println!("Fetching weather data during build...");

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

    // Create a simple async runtime for this build-time fetch
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let api = GeoMetAPI::new()?;

        // Fetch images concurrently
        let (top_left, top_right, bottom_left, bottom_right, legend) = tokio::try_join!(
            api.get_wms_image("GOES-East_1km_VisibleIRSandwich-NightMicrophysicsIR", &goes_time_str, bbox.clone(), width, height),
            api.get_wms_image("GOES-East_2km_NightMicrophysics", &goes_time_str, bbox.clone(), width, height),
            api.get_wms_image("GOES-East_1km_NaturalColor", &goes_time_str, bbox.clone(), width, height),
            api.get_wms_image("HRDPS.CONTINENTAL_PN-SLP", &hrdps_time_str, bbox.clone(), width, height),
            api.get_legend_graphic("HRDPS.CONTINENTAL_PN-SLP", Some("PRESSURE4"), "image/png", Some("en"))
        )?;

        // Save images to ui directory where SLINT expects them
        std::fs::write("ui/images/top_left.png", top_left)?;
        std::fs::write("ui/images/top_right.png", top_right)?;
        std::fs::write("ui/images/bottom_left.png", bottom_left)?;
        std::fs::write("ui/images/bottom_right.png", bottom_right)?;
        std::fs::write("ui/images/legend.png", legend)?;

        println!("Weather data fetched and saved successfully during build");
        Ok(())
    })
}
