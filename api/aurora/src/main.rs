extern crate pretty_env_logger;
#[macro_use] extern crate log;

use aurora::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    info!("🛰️  Testing Aurora API functions...");

    // Test aurora forecast
    info!("🌌 Fetching aurora forecast...");
    match fetch_aurora_forecast().await {
        Ok(data) => info!("✅ Aurora forecast: {} bytes", data.len()),
        Err(e) => error!("❌ Aurora forecast failed: {}", e),
    }

    // Test ACE real-time solar wind
    info!("🌞 Fetching ACE real-time solar wind...");
    match fetch_ace_real_time_solar_wind().await {
        Ok(data) => info!("✅ ACE solar wind: {} bytes", data.len()),
        Err(e) => error!("❌ ACE solar wind failed: {}", e),
    }

    // Test DSCOVR solar wind
    info!("🛰️  Fetching DSCOVR solar wind...");
    match fetch_dscovr_solar_wind().await {
        Ok(data) => info!("✅ DSCOVR solar wind: {} bytes", data.len()),
        Err(e) => error!("❌ DSCOVR solar wind failed: {}", e),
    }

    // Test space weather overview
    info!("🌍 Fetching space weather overview...");
    match fetch_space_weather_overview().await {
        Ok(data) => info!("✅ Space weather overview: {} bytes", data.len()),
        Err(e) => error!("❌ Space weather overview failed: {}", e),
    }

    // Test ACE EPAM
    info!("⚡ Fetching ACE EPAM...");
    match fetch_ace_epam().await {
        Ok(data) => info!("✅ ACE EPAM: {} bytes", data.len()),
        Err(e) => error!("❌ ACE EPAM failed: {}", e),
    }

    // Test Canadian magnetic observatories
    info!("🇨🇦 Fetching Canadian magnetic observatories...");
    match fetch_canadian_magnetic().await {
        Ok(data) => info!("✅ Canadian magnetic: {} bytes", data.len()),
        Err(e) => error!("❌ Canadian magnetic failed: {}", e),
    }

    // Test alerts timeline
    info!("🚨 Fetching alerts timeline...");
    match fetch_alerts_timeline().await {
        Ok(data) => info!("✅ Alerts timeline: {} bytes", data.len()),
        Err(e) => error!("❌ Alerts timeline failed: {}", e),
    }

    info!("🎉 Aurora API testing complete!");
    Ok(())
}
