use geomet::{GeoMetAPI, BoundingBox};
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <command> [args...]", args[0]);
        eprintln!("Commands:");
        eprintln!("  capabilities [wms|wcs] - List available layers/coverages");
        eprintln!("  wms <layer> <time> <min_lon> <max_lon> <min_lat> <max_lat> [width] [height] - Fetch WMS image");
        eprintln!("  wcs <coverage_id> <time> <min_lon> <max_lon> <min_lat> <max_lat> [format] - Fetch WCS data");
        eprintln!("  point <coverage_id> <time> <lon> <lat> [format] - Fetch point data");
        eprintln!("");
        eprintln!("Examples:");
        eprintln!("  {} capabilities wms", args[0]);
        eprintln!("  {} wms RDPS_10km_AirTemp_2m 2025-10-13T12:00:00Z -130 -60 20 60 800 600", args[0]);
        eprintln!("  {} wcs RDPS_10km_Precip-Accum24h 2025-10-13T12:00:00Z -130 -60 20 60", args[0]);
        eprintln!("  {} point RDPS_10km_AirTemp_2m 2025-10-13T12:00:00Z -75.7 45.4", args[0]);
        std::process::exit(1);
    }

    let command = &args[1];

    // Create API instance
    let api = GeoMetAPI::new()?;

    match command.as_str() {
        "capabilities" => {
            if args.len() < 3 {
                eprintln!("Specify 'wms' or 'wcs'");
                std::process::exit(1);
            }

            let service = &args[2];
            match service.as_str() {
                "wms" => {
                    println!("Fetching WMS capabilities...");
                    let caps = api.get_wms_capabilities().await?;
                    println!("Found {} RDPS layers:", caps.layers.len());
                    for layer in &caps.layers {
                        println!("  {}", layer.name);
                    }
                }
                "wcs" => {
                    println!("Fetching WCS capabilities...");
                    let caps = api.get_wcs_capabilities().await?;
                    println!("Found {} RDPS coverages:", caps.coverages.len());
                    for coverage in &caps.coverages {
                        println!("  {}", coverage.coverage_id);
                    }
                }
                _ => {
                    eprintln!("Invalid service. Use 'wms' or 'wcs'");
                    std::process::exit(1);
                }
            }
        }

        "wms" => {
            if args.len() < 9 {
                eprintln!("Not enough arguments for WMS command");
                std::process::exit(1);
            }

            let layer = &args[2];
            let time = &args[3];
            let min_lon: f64 = args[4].parse()?;
            let max_lon: f64 = args[5].parse()?;
            let min_lat: f64 = args[6].parse()?;
            let max_lat: f64 = args[7].parse()?;
            let width: u32 = args.get(8).unwrap_or(&"800".to_string()).parse()?;
            let height: u32 = args.get(9).unwrap_or(&"600".to_string()).parse()?;

            let bbox = BoundingBox::new(min_lon, max_lon, min_lat, max_lat);

            println!("Fetching WMS image for layer: {}", layer);
            println!("Time: {}", time);
            println!("Bounding box: {}", bbox.to_string());
            println!("Size: {}x{}", width, height);

            let image_data = api.get_wms_image(layer, time, bbox, width, height).await?;

            let filename = format!("{}_{}.png", layer, time.replace(":", "-"));
            std::fs::write(&filename, &image_data)?;
            println!("Image saved to: {}", filename);
        }

        "wcs" => {
            if args.len() < 9 {
                eprintln!("Not enough arguments for WCS command");
                std::process::exit(1);
            }

            let coverage_id = &args[2];
            let time = &args[3];
            let min_lon: f64 = args[4].parse()?;
            let max_lon: f64 = args[5].parse()?;
            let min_lat: f64 = args[6].parse()?;
            let max_lat: f64 = args[7].parse()?;
            let format = args.get(8).map(|s| s.as_str()).unwrap_or("application/x-netcdf");

            let bbox = BoundingBox::new(min_lon, max_lon, min_lat, max_lat);

            println!("Fetching WCS data for coverage: {}", coverage_id);
            println!("Time: {}", time);
            println!("Bounding box: {}", bbox.to_string());
            println!("Format: {}", format);

            let data = api.get_wcs_data(coverage_id, time, bbox, format).await?;

            let extension = if format.contains("netcdf") { "nc" } else { "tif" };
            let filename = format!("{}_{}.{}", coverage_id, time.replace(":", "-"), extension);
            std::fs::write(&filename, &data)?;
            println!("Data saved to: {}", filename);
        }

        "point" => {
            if args.len() < 6 {
                eprintln!("Not enough arguments for point command");
                std::process::exit(1);
            }

            let coverage_id = &args[2];
            let time = &args[3];
            let lon: f64 = args[4].parse()?;
            let lat: f64 = args[5].parse()?;
            let format = args.get(6).map(|s| s.as_str()).unwrap_or("application/x-netcdf");

            println!("Fetching point data for coverage: {}", coverage_id);
            println!("Time: {}", time);
            println!("Location: ({}, {})", lon, lat);
            println!("Format: {}", format);

            let data = api.get_point_data(coverage_id, time, lon, lat, format).await?;

            let extension = if format.contains("netcdf") { "nc" } else { "tif" };
            let filename = format!("{}_{}_{}_{}.{}", coverage_id, time.replace(":", "-"), lon, lat, extension);
            std::fs::write(&filename, &data)?;
            println!("Data saved to: {}", filename);
        }

        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    }

    Ok(())
}
