use cleardarksky::ClearDarkSkyAPI;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("ClearDarkSky API Test");

    let api = ClearDarkSkyAPI::new();

    // Test coordinates (Montreal area)
    let latitude = 45.50;
    let longitude = -73.57;

    println!("Testing with coordinates: lat={}, lon={}", latitude, longitude);

    // Test fetch_nearest_sky_chart_location
    match api.fetch_nearest_sky_chart_location(latitude, longitude).await {
        Ok(location) => {
            println!("Found nearest sky chart location: {}", location);

            // Test fetch_clear_sky_chart
            match api.fetch_clear_sky_chart(&location).await {
                Ok(filename) => {
                    println!("Successfully saved sky chart to: {}", filename);
                }
                Err(e) => {
                    println!("Error fetching sky chart: {}", e);
                }
            }
        }
        Err(e) => {
            println!("Error finding sky chart location: {}", e);
        }
    }

    Ok(())
}
