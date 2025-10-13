use clearoutside::ClearOutsideAPI;
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <latitude> <longitude> [view]", args[0]);
        eprintln!("  latitude: Latitude with 2 decimal places (e.g., 45.50)");
        eprintln!("  longitude: Longitude with 2 decimal places (e.g., -73.57)");
        eprintln!("  view: Optional view type - 'current', 'midday', or 'midnight' (default: midday)");
        std::process::exit(1);
    }

    let lat = &args[1];
    let long = &args[2];
    let view = if args.len() > 3 { Some(args[3].as_str()) } else { None };

    println!("Fetching ClearOutside data for coordinates: {}, {}", lat, long);

    // Create API instance
    let api = ClearOutsideAPI::new(lat, long, view).await?;

    // Fetch and parse data
    match api.pull() {
        Ok(forecast) => {
            // Save to JSON file
            let filename = format!("clearoutside_data.json");
            let json_data = serde_json::to_string_pretty(&forecast)?;

            std::fs::write(&filename, &json_data)?;
            println!("Data saved to: {}", filename);

            // Print summary
            println!("\n=== ClearOutside Forecast Summary ===");
            println!("Sky Quality:");
            println!("  Magnitude: {}", forecast.sky_quality.magnitude);
            println!("  Bortle Class: {}", forecast.sky_quality.bortle_class);
            println!("  Brightness: {:?}", forecast.sky_quality.brightness);
            println!("  Artificial Brightness: {:?}", forecast.sky_quality.artif_brightness);

            println!("\nGeneral Info:");
            println!("  Last Generated: {} {}",
                forecast.gen_info.last_gen.date,
                forecast.gen_info.last_gen.time
            );
            println!("  Forecast Period: {} to {}",
                forecast.gen_info.forecast.from_day,
                forecast.gen_info.forecast.to_day
            );
            println!("  Timezone: {}", forecast.gen_info.timezone);

            println!("\nForecast Days: {}", forecast.forecast.len());
            for (day_key, day_info) in &forecast.forecast {
                println!("\n{}:", day_key);
                println!("  Date: {} ({})",
                    day_info.date.long,
                    day_info.date.short
                );
                println!("  Moon: {} - {}% ({})",
                    day_info.moon.phase.name,
                    day_info.moon.phase.percentage,
                    if day_info.moon.rise.is_empty() { "No rise/set data".to_string() }
                    else { format!("{} to {}", day_info.moon.rise, day_info.moon.set) }
                );
                println!("  Sun: {} to {}",
                    day_info.sun.rise,
                    day_info.sun.set
                );
            }
        }
        Err(e) => {
            eprintln!("Error fetching forecast data: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
