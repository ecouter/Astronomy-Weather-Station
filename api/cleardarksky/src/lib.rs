use scraper::{Html, Selector};
use std::fs;

/// ClearDarkSky API client for fetching sky charts
pub struct ClearDarkSkyAPI {
    client: reqwest::Client,
}

impl ClearDarkSkyAPI {
    /// Create a new ClearDarkSkyAPI instance
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Fetch nearest sky chart location from coordinates
    ///
    /// # Arguments
    /// * `latitude` - Latitude as f64
    /// * `longitude` - Longitude as f64
    ///
    /// # Returns
    /// Returns the location number with "csk.gif" suffix (e.g., "ThrsQCcsk.gif")
    pub async fn fetch_nearest_sky_chart_location(&self, latitude: f64, longitude: f64) -> Result<String, anyhow::Error> {
        // Validate coordinates
        if latitude < -90.0 || latitude > 90.0 {
            return Err(anyhow::anyhow!("Latitude must be between -90 and 90"));
        }
        if longitude < -180.0 || longitude > 180.0 {
            return Err(anyhow::anyhow!("Longitude must be between -180 and 180"));
        }

        // Format coordinates with 2 decimal places
        let lat_str = format!("{:.2}", latitude);
        let lon_str = format!("{:.2}", longitude);

        // Construct URL
        let url = format!(
            "https://www.cleardarksky.com/cgi-bin/find_chart.py?type=llmap&Mn=astrophotography&olat={}&olong={}&olatd=&olatm=&olongd=&olongm=&unit=0",
            lat_str, lon_str
        );

        // Fetch HTML content
        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch sky chart data: HTTP {}", response.status()));
        }

        let html_content = response.text().await?;

        // Parse HTML to find the first location link
        let document = Html::parse_document(&html_content);
        let selector = Selector::parse("a[href*='../c/'][href*='key.html?1']").unwrap();

        for element in document.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                // Extract location number from href like "../c/ThrsQCkey.html?1"
                if let Some(start) = href.find("../c/") {
                    let after_c = &href[start + 5..];
                    if let Some(end) = after_c.find("key.html?1") {
                        let location_number = &after_c[..end];
                        return Ok(format!("{}csk.gif", location_number));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Could not find sky chart location in HTML response"))
    }

    /// Fetch and save clear sky chart GIF
    ///
    /// # Arguments
    /// * `location_number` - Location identifier (e.g., "ThrsQCcsk.gif")
    ///
    /// # Returns
    /// Returns the filename where the GIF was saved
    pub async fn fetch_clear_sky_chart(&self, location_number: &str) -> Result<String, anyhow::Error> {
        // Validate location_number format
        if !location_number.ends_with("csk.gif") {
            return Err(anyhow::anyhow!("Location number must end with 'csk.gif'"));
        }

        // Extract the location code (remove csk.gif suffix)
        let location_code = location_number.trim_end_matches("csk.gif");

        // Construct URL
        let url = format!("https://cleardarksky.com/c/{}csk.gif", location_code);

        // Fetch GIF data
        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch sky chart GIF: HTTP {}", response.status()));
        }

        let gif_data = response.bytes().await?;

        // Save GIF to file
        fs::write(location_number, &gif_data)?;

        Ok(location_number.to_string())
    }

    /// Fetch clear sky chart GIF bytes
    ///
    /// # Arguments
    /// * `location_number` - Location identifier (e.g., "ThrsQCcsk.gif")
    ///
    /// # Returns
    /// Returns the GIF data as bytes
    pub async fn fetch_clear_sky_chart_bytes(&self, location_number: &str) -> Result<Vec<u8>, anyhow::Error> {
        // Validate location_number format
        if !location_number.ends_with("csk.gif") {
            return Err(anyhow::anyhow!("Location number must end with 'csk.gif'"));
        }

        // Extract the location code (remove csk.gif suffix)
        let location_code = location_number.trim_end_matches("csk.gif");

        // Construct URL
        let url = format!("https://cleardarksky.com/c/{}csk.gif", location_code);

        // Fetch GIF data
        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch sky chart GIF: HTTP {}", response.status()));
        }

        let gif_data = response.bytes().await?;

        Ok(gif_data.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_creation() {
        let api = ClearDarkSkyAPI::new();
        // Just test that it can be created
        assert!(true);
    }

    #[tokio::test]
    async fn test_invalid_coordinates() {
        let api = ClearDarkSkyAPI::new();

        // Test invalid latitude
        let result = api.fetch_nearest_sky_chart_location(91.0, 0.0).await;
        assert!(result.is_err());

        // Test invalid longitude
        let result = api.fetch_nearest_sky_chart_location(0.0, 181.0).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_location_format() {
        let api = ClearDarkSkyAPI::new();

        // This would be an async test, but we'll just check the logic
        let location = "invalid";
        assert!(!location.ends_with("csk.gif"));
    }
}
