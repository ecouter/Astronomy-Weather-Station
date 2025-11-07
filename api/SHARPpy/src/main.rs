extern crate pretty_env_logger;
#[macro_use] extern crate log;

use sharppy::{generate_sounding, SoundingParams};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <latitude> <longitude> [output_file] [title]", args[0]);
        eprintln!("");
        eprintln!("Arguments:");
        eprintln!("  latitude     Latitude of the sounding location (required)");
        eprintln!("  longitude    Longitude of the sounding location (required)");
        eprintln!("  output_file  Output PNG file path (optional, default: sounding_gfs.png)");
        eprintln!("  title        Plot title (optional, auto-generated if not provided)");
        eprintln!("");
        eprintln!("Example:");
        eprintln!("  {} 45.5 -73.5 sounding.png \"Montreal Sounding\"", args[0]);
        std::process::exit(1);
    }

    // Parse required arguments
    let lat: f64 = args[1].parse().map_err(|_| "Invalid latitude")?;
    let lon: f64 = args[2].parse().map_err(|_| "Invalid longitude")?;

    // Parse optional arguments
    let output_file = args.get(3).map(|s| s.clone());
    let title = args.get(4).map(|s| s.clone());

    info!("Generating GFS sounding for location {:.3}N {:.3}{}",
          lat, lon.abs(), if lon >= 0.0 { "E" } else { "W" });

    if let Some(ref file) = output_file {
        info!("Output file: {}", file);
    }

    if let Some(ref t) = title {
        info!("Title: {}", t);
    }

    // Create sounding parameters
    let mut params = SoundingParams::new(lat, lon);

    if let Some(file) = output_file {
        params = params.with_output_file(file);
    }

    if let Some(t) = title {
        params = params.with_title(t);
    }

    // Generate the sounding
    match generate_sounding(params) {
        Ok(output_path) => {
            info!("✅ Successfully generated GFS sounding");
            info!("📊 Sounding plot saved to: {}", output_path);

            // Check if file exists and get size
            if let Ok(metadata) = std::fs::metadata(&output_path) {
                let size_kb = metadata.len() / 1024;
                info!("📁 File size: {} KB", size_kb);
            }

            info!("\n🎯 GFS atmospheric sounding generation completed successfully!");
        }
        Err(e) => {
            error!("❌ Error generating GFS sounding: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
