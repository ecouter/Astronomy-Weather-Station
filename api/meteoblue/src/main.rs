use meteoblue::fetch_meteoblue_data;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments for latitude and longitude
    let args: Vec<String> = std::env::args().collect();

    let (lat, lon) = if args.len() >= 3 {
        // Custom coordinates from command line
        let lat: f64 = args[1].parse().map_err(|_| "Invalid latitude")?;
        let lon: f64 = args[2].parse().map_err(|_| "Invalid longitude")?;
        (lat, lon)
    } else {
        // Example usage: default coordinates from the sample file
        println!("Usage: {} <latitude> <longitude>", args[0]);
        println!("Using default coordinates (45.219N -73.111W)...");
        (45.219, -73.111)
    };

    println!("Fetching astronomy seeing data for {:.3}N {:.3}{}...",
             lat, lon.abs(), if lon >= 0.0 { "E" } else { "W" });

    match fetch_meteoblue_data(lat, lon).await {
        Ok(data) => {
            println!("✅ Successfully retrieved {} data points", data.len());

            // Print first few data points as example
            println!("\n📊 Sample forecast data:");
            for point in data.iter().take(5) {
                println!("  {} {:02}:00 - Seeing: {:.2}\" (indices: {}/{}, clouds: {}/{}/{}%, temp: {:.1}°C, humidity: {}%)",
                    point.day,
                    point.hour,
                    point.seeing_arcsec,
                    point.index1,
                    point.index2,
                    point.clouds_low_pct,
                    point.clouds_mid_pct,
                    point.clouds_high_pct,
                    point.temp_c,
                    point.humidity_pct
                );
            }

            // Find best seeing conditions
            let mut best_seeing = data.iter().max_by(|a, b| {
                // Better seeing = lower arc seconds + lower clouds + higher indices
                let a_score = (-a.seeing_arcsec as f32) + (a.index1 + a.index2) as f32
                            - (a.clouds_low_pct + a.clouds_mid_pct + a.clouds_high_pct) as f32;
                let b_score = (-b.seeing_arcsec as f32) + (b.index1 + b.index2) as f32
                            - (b.clouds_low_pct + b.clouds_mid_pct + b.clouds_high_pct) as f32;
                a_score.partial_cmp(&b_score).unwrap_or(std::cmp::Ordering::Equal)
            });

            if let Some(best) = best_seeing {
                let total_clouds = best.clouds_low_pct + best.clouds_mid_pct + best.clouds_high_pct;
                println!("\n🌟 Best observability conditions:");
                println!("  {} {:02}:00 - Seeing: {:.2}\", Indices: {}/{}, Clouds: {}%",
                    best.day, best.hour, best.seeing_arcsec, best.index1, best.index2, total_clouds);
            }

            println!("\n💾 Data saved to JSON file.");
        }
        Err(e) => {
            eprintln!("❌ Error fetching seeing data: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
