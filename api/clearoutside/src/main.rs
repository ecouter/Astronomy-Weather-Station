// CLI example for the ClearOutside API
//
// Usage:
//   cargo run -- 43.16 -75.84 midday
//   cargo run -- --help

use clearoutside::{create_api, update_and_pull, ClearOutsideError};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // Parse command line arguments
    let (lat, lon, view) = if args.len() >= 4 {
        // Use provided coordinates and view
        let lat = args[1].clone();
        let lon = args[2].clone();
        let view = args[3].clone();
        (lat, lon, view)
    } else {
        // Default example usage
        println!("ClearOutside Rust API - Functional wrapper for Python scraper");
        println!();
        println!("Usage: {} <latitude> <longitude> <view>", args[0]);
        println!("  latitude:  e.g., 43.16");
        println!("  longitude: e.g., -75.84");
        println!("  view:     midday | midnight | current");
        println!();
        println!("Example:");
        println!("  {} 43.16 -75.84 midday", args[0]);
        println!();
        println!("Using default coordinates (43.16, -75.84) with midday view...");

        ("43.16".to_string(), "-75.84".to_string(), "midday".to_string())
    };

    println!("🌟 Fetching ClearOutside data for lat={}, lon={}, view={}", lat, lon, view);

    // Create API state (functional - no global state)
    let api_state = create_api(&lat, &lon, &view);

    match update_and_pull(&api_state).await {
        Ok((_, data)) => {
            println!("✅ Successfully retrieved forecast data!");
            println!();
            println!("📊 General Info:");
            println!("   Last updated: {} {}", data.gen_info.last_gen.date, data.gen_info.last_gen.time);
            println!("   Timezone: {}", data.gen_info.timezone);
            println!("   Forecast range: {} to {}", data.gen_info.forecast.from_day, data.gen_info.forecast.to_day);
            println!();
            println!("🌌 Sky Quality:");
            println!("   Magnitude: {}", data.sky_quality.magnitude);
            println!("   Bortle class: {}", data.sky_quality.bortle_class);
            println!("   Brightness: {} {}", data.sky_quality.brightness[0], data.sky_quality.brightness[1]);
            println!("   Artificial brightness: {} {}", data.sky_quality.artif_brightness[0], data.sky_quality.artif_brightness[1]);
            println!();

            // Show data for first day as example
            if let Some((day_key, day_data)) = data.forecast.iter().next() {
                println!("📅 {} ({} {}):", day_key, day_data.date.long, day_data.date.long);
                println!("   🌅 Sun: rise={}, set={}, transit={}",
                    day_data.sun.rise, day_data.sun.set, day_data.sun.transit);
                println!("   🌙 Moon: rise={}, set={}, phase={} ({})",
                    day_data.moon.rise, day_data.moon.set,
                    day_data.moon.phase.name, day_data.moon.phase.percentage);

                // Show forecast for midday hour (12) as example
                if let Some(hour_12) = day_data.hours.get("12") {
                    println!("   🕐 Midday hour (12:00):");
                    println!("     Conditions: {}", hour_12.conditions);
                    println!("     Clouds: total={}%, low={}%, mid={}%, high={}%",
                        hour_12.total_clouds, hour_12.low_clouds, hour_12.mid_clouds, hour_12.high_clouds);
                    println!("     Temperature: {}°C (feels {}°C, dew point {}°C)",
                        hour_12.temperature.general, hour_12.temperature.feels_like, hour_12.temperature.dew_point);
                    println!("     Wind: {} {} km/h", hour_12.wind.direction, hour_12.wind.speed);
                    println!("     Precipitation: {} ({}% chance, {}mm)",
                        hour_12.prec_type, hour_12.prec_probability, hour_12.prec_amount);
                    println!("     Humidity: {}%, Pressure: {}mb, Ozone: {}du",
                        hour_12.rel_humidity, hour_12.pressure, hour_12.ozone);
                }

                println!();
                println!("💾 Total hours in forecast: {}", day_data.hours.len());
                println!("📈 Other days available: {}", data.forecast.len() - 1);
            }

            println!();
            println!("✨ Data processing complete - functional API design ensures no global state!");
        }
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            match e {
                ClearOutsideError::ModuleNotFound(_) => {
                    eprintln!("💡 Make sure Python package 'clear-outside-apy' is installed:");
                    eprintln!("   pip install clear-outside-apy");
                    eprintln!("   or");
                    eprintln!("   pip install git+https://github.com/TheElevatedOne/ClearOutsideAPY.git");
                }
                ClearOutsideError::PythonNotInstalled => {
                    eprintln!("💡 Make sure Python 3 is installed and available as 'python3'");
                }
                _ => {}
            }
            std::process::exit(1);
        }
    }

    Ok(())
}
