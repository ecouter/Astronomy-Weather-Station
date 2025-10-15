use geomet::{GeoMetAPI, BoundingBox};
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <command> [args...]", args[0]);
        eprintln!("Commands:");
        eprintln!("  capabilities [wms|wcs] [output_file] - List available layers/coverages or save to file");
        eprintln!("  wms <layer> <time> <min_lon> <max_lon> <min_lat> <max_lat> [width] [height] - Fetch WMS image");
        eprintln!("  wcs <coverage_id> <time> <min_lon> <max_lon> <min_lat> <max_lat> [format] [subset_crs] [output_crs] [res_x] [res_y] [size_x] [size_y] [interp] [range] - Fetch WCS data");
        eprintln!("  point <coverage_id> <time> <lon> <lat> [format] - Fetch point data");
        eprintln!("  legend <layer> [style] [format] [lang] - Fetch legend graphic");
        eprintln!("");
        eprintln!("Examples:");
        eprintln!("  {} capabilities wms", args[0]);
        eprintln!("  {} wms RDPS_10km_AirTemp_2m 2025-10-13T12:00:00Z -130 -60 20 60 800 600", args[0]);
        eprintln!("  {} wcs RDPS_10km_Precip-Accum24h 2025-10-13T12:00:00Z -130 -60 20 60 application/x-netcdf EPSG:4326 EPSG:4326 0.24 0.24", args[0]);
        eprintln!("  {} point RDPS_10km_AirTemp_2m 2025-10-13T12:00:00Z -75.7 45.4", args[0]);
        eprintln!("  {} legend RDPS_10km_AirTemp_2m TEMPERATURE-LINEAR image/png en", args[0]);
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
            let output_file = args.get(3).map(|s| s.as_str());

            match service.as_str() {
                "wms" => {
                    println!("Fetching WMS capabilities...");
                    let xml_content = api.get_wms_capabilities_raw().await?;

                    if let Some(filename) = output_file {
                        // Extract only layer names and titles, filter out non-layer elements
                        let mut filtered_content = String::new();
                        let mut in_layer = false;
                        let mut current_layer = String::new();

                        for line in xml_content.lines() {
                            let trimmed = line.trim();

                            // Start of a Layer element
                            if trimmed.contains("<Layer") {
                                in_layer = true;
                                current_layer.clear();
                            }

                            // End of a Layer element
                            if trimmed.contains("</Layer>") {
                                if in_layer && !current_layer.is_empty() {
                                    filtered_content.push_str(&current_layer);
                                    filtered_content.push('\n');
                                }
                                in_layer = false;
                                current_layer.clear();
                            }

                            // Extract Name and Title within Layer
                            if in_layer && (trimmed.contains("<Name>") || trimmed.contains("<Title>")) {
                                if let Some(start) = trimmed.find('<') {
                                    if let Some(end) = trimmed.find('>') {
                                        let tag_start = &trimmed[start..=end];
                                        if let Some(content_end) = trimmed.find("</") {
                                            let content = trimmed[end+1..content_end].trim();
                                            if !content.is_empty() && !content.contains("WMS") && !content.contains("Canadian Weather") {
                                                current_layer.push_str(&format!("{}: {}\n", tag_start.trim_matches('<').trim_matches('>'), content));
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        std::fs::write(filename, filtered_content)?;
                        println!("Filtered WMS capabilities saved to: {}", filename);
                    } else {
                        // Extract all layer names (not just RDPS)
                        let mut all_layers = Vec::new();
                        for line in xml_content.lines() {
                            if line.contains("<Name>") && line.contains("</Name>") {
                                if let Some(start) = line.find("<Name>") {
                                    if let Some(end) = line.find("</Name>") {
                                        let name = line[start + 6..end].trim();
                                        if !name.is_empty() {
                                            all_layers.push(name.to_string());
                                        }
                                    }
                                }
                            }
                        }
                        println!("Found {} total layers:", all_layers.len());
                        for layer in &all_layers {
                            println!("  {}", layer);
                        }
                    }
                }
                "wcs" => {
                    println!("Fetching WCS capabilities...");
                    let xml_content = api.get_wcs_capabilities_raw().await?;

                    if let Some(filename) = output_file {
                        std::fs::write(filename, &xml_content)?;
                        println!("WCS capabilities saved to: {}", filename);
                    } else {
                        // Extract all coverage IDs (not just RDPS)
                        let mut all_coverages = Vec::new();
                        for line in xml_content.lines() {
                            if line.contains("<wcs:CoverageId>") && line.contains("</wcs:CoverageId>") {
                                if let Some(start) = line.find("<wcs:CoverageId>") {
                                    if let Some(end) = line.find("</wcs:CoverageId>") {
                                        let coverage_id = line[start + 16..end].trim();
                                        if !coverage_id.is_empty() {
                                            all_coverages.push(coverage_id.to_string());
                                        }
                                    }
                                }
                            }
                        }
                        println!("Found {} total coverages:", all_coverages.len());
                        for coverage in &all_coverages {
                            println!("  {}", coverage);
                        }
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

            // Optional advanced parameters
            let subsetting_crs = args.get(9).map(|s| s.as_str());
            let output_crs = args.get(10).map(|s| s.as_str());
            let resolution_x = args.get(11).and_then(|s| s.parse().ok());
            let resolution_y = args.get(12).and_then(|s| s.parse().ok());
            let size_x = args.get(13).and_then(|s| s.parse().ok());
            let size_y = args.get(14).and_then(|s| s.parse().ok());
            let interpolation = args.get(15).map(|s| s.as_str());
            let range_subset = args.get(16).map(|s| s.as_str());

            let bbox = BoundingBox::new(min_lon, max_lon, min_lat, max_lat);

            println!("Fetching WCS data for coverage: {}", coverage_id);
            println!("Time: {}", time);
            println!("Bounding box: {}", bbox.to_string());
            println!("Format: {}", format);

            if let Some(crs) = subsetting_crs {
                println!("Subsetting CRS: {}", crs);
            }
            if let Some(crs) = output_crs {
                println!("Output CRS: {}", crs);
            }
            if let Some(rx) = resolution_x {
                println!("Resolution X: {}", rx);
            }
            if let Some(ry) = resolution_y {
                println!("Resolution Y: {}", ry);
            }
            if let Some(sx) = size_x {
                println!("Size X: {}", sx);
            }
            if let Some(sy) = size_y {
                println!("Size Y: {}", sy);
            }
            if let Some(interp) = interpolation {
                println!("Interpolation: {}", interp);
            }
            if let Some(range) = range_subset {
                println!("Range subset: {}", range);
            }

            let data = api.get_wcs_data_advanced(
                coverage_id, time, bbox, format,
                subsetting_crs, output_crs, resolution_x, resolution_y,
                size_x, size_y, interpolation, range_subset
            ).await?;

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

        "legend" => {
            if args.len() < 3 {
                eprintln!("Not enough arguments for legend command");
                std::process::exit(1);
            }

            let layer = &args[2];
            let style = args.get(3).map(|s| s.as_str());
            let format = args.get(4).map(|s| s.as_str()).unwrap_or("image/png");
            let language = args.get(5).map(|s| s.as_str());

            println!("Fetching legend for layer: {}", layer);
            if let Some(s) = style {
                println!("Style: {}", s);
            }
            println!("Format: {}", format);
            if let Some(lang) = language {
                println!("Language: {}", lang);
            }

            let legend_data = api.get_legend_graphic(layer, style, format, language).await?;

            let style_suffix = style.map(|s| format!("_{}", s)).unwrap_or_default();
            let lang_suffix = language.map(|l| format!("_{}", l)).unwrap_or_default();
            let extension = if format.contains("png") { "png" } else { "jpg" };
            let filename = format!("legend_{}{}{}.{}", layer, style_suffix, lang_suffix, extension);
            std::fs::write(&filename, &legend_data)?;
            println!("Legend saved to: {}", filename);
        }

        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    }

    Ok(())
}
