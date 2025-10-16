use environment_canada::{EnvironmentCanadaAPI, ForecastType, Region};
use std::env;
use chrono::{Datelike, Timelike, Utc};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    // Get the latest available model run time (always use a model that ran several hours ago)
    // Models run at 00, 06, 12, 18 UTC but data becomes available some time after
    let now = Utc::now();
    let current_hour = now.hour();

    // Always use the model run from 6+ hours ago to ensure data availability
    let (model_date, latest_model_hour) = match current_hour {
        0..=5 => (now - chrono::Duration::days(1), 18),  // Use yesterday 18:00
        6..=11 => (now, 0),    // Use today 00:00
        12..=17 => (now, 6),   // Use today 06:00
        18..=23 => (now, 12),  // Use today 12:00
        _ => (now, 12),        // Fallback
    };

    let latest_model_run = format!(
        "{:04}{:02}{:02}{:02}",
        model_date.year(),
        model_date.month(),
        model_date.day(),
        latest_model_hour
    );

    if args.len() < 4 {
        eprintln!("Usage: {} <forecast_type> <region> <hours_after>", args[0]);
        eprintln!("");
        eprintln!("Forecast types:");
        eprintln!("  cloud           - Cloud cover forecast");
        eprintln!("  seeing          - Seeing forecast (multiples of 3 hours only)");
        eprintln!("  transparency    - Sky transparency forecast");
        eprintln!("  surface_wind    - Surface wind forecast");
        eprintln!("  temperature     - Temperature forecast");
        eprintln!("  relative_humidity - Relative humidity forecast");
        eprintln!("");
        eprintln!("Regions:");
        eprintln!("  northeast, northwest, southeast, southwest");
        eprintln!("");
        eprintln!("Hours after: 001-084 (seeing: multiples of 3 only)");
        eprintln!("Model run: Automatically uses latest available (00, 06, 12, 18 UTC)");
        eprintln!("");
        eprintln!("Examples:");
        eprintln!("  {} cloud northeast 001", args[0]);
        eprintln!("  {} seeing northwest 003", args[0]);
        eprintln!("  {} temperature southeast 024", args[0]);
        std::process::exit(1);
    }

    let forecast_type_str = &args[1];
    let region_str = &args[2];
    let hours_after_str = &args[3];
    let model_run = &latest_model_run;

    // Parse forecast type
    let forecast_type = match forecast_type_str.as_str() {
        "cloud" => ForecastType::Cloud,
        "seeing" => ForecastType::Seeing,
        "transparency" => ForecastType::Transparency,
        "surface_wind" => ForecastType::SurfaceWind,
        "temperature" => ForecastType::Temperature,
        "relative_humidity" => ForecastType::RelativeHumidity,
        _ => {
            eprintln!("Invalid forecast type: {}", forecast_type_str);
            eprintln!("Use: cloud, seeing, transparency, surface_wind, temperature, relative_humidity");
            std::process::exit(1);
        }
    };

    // Parse region
    let region = match region_str.as_str() {
        "northeast" => Region::Northeast,
        "northwest" => Region::Northwest,
        "southeast" => Region::Southeast,
        "southwest" => Region::Southwest,
        _ => {
            eprintln!("Invalid region: {}", region_str);
            eprintln!("Use: northeast, northwest, southeast, southwest");
            std::process::exit(1);
        }
    };

    // Parse hours after
    let hours_after: u32 = match hours_after_str.parse() {
        Ok(h) => h,
        Err(_) => {
            eprintln!("Invalid hours_after: {} (must be a number)", hours_after_str);
            std::process::exit(1);
        }
    };

    // Create API instance
    let api = EnvironmentCanadaAPI::new()?;

    println!("Fetching {} forecast for {} region...", forecast_type_str, region_str);
    println!("Model run: {}", model_run);
    println!("Hours after: {:03}", hours_after);

    // Fetch and save the forecast
    let (filename, _data) = api.fetch_and_save_forecast(forecast_type, model_run, region, hours_after).await?;

    println!("✅ Successfully saved forecast to: {}", filename);

    Ok(())
}
