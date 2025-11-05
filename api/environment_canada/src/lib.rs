use anyhow::{anyhow, Result};
use reqwest::Client;
use std::fs;

/// Environment Canada astronomical forecast API client
pub struct EnvironmentCanadaAPI {
    client: Client,
    base_url: String,
}

/// Forecast types available from Environment Canada astronomical models
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ForecastType {
    Cloud,
    Seeing,
    Transparency,
    SurfaceWind,
    Temperature,
    RelativeHumidity,
}

/// Geographic regions for astronomical forecasts
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Region {
    Northeast,
    Northwest,
    Southeast,
    Southwest,
}

impl ForecastType {
    /// Get the URL suffix for this forecast type
    fn url_suffix(&self) -> &'static str {
        match self {
            ForecastType::Cloud => "_I_ASTRO_nt_",
            ForecastType::Seeing => "_I_ASTRO_seeing_",
            ForecastType::Transparency => "_I_ASTRO_transp_",
            ForecastType::SurfaceWind => "_I_ASTRO_uv_",
            ForecastType::Temperature => "_I_ASTRO_tt_",
            ForecastType::RelativeHumidity => "_I_ASTRO_hr_",
        }
    }

    /// Get the human-readable name for this forecast type
    pub fn name(&self) -> &'static str {
        match self {
            ForecastType::Cloud => "cloud",
            ForecastType::Seeing => "seeing",
            ForecastType::Transparency => "transparency",
            ForecastType::SurfaceWind => "surface_wind",
            ForecastType::Temperature => "temperature",
            ForecastType::RelativeHumidity => "relative_humidity",
        }
    }

    /// Check if this forecast type supports the given hours_after value
    fn validate_hours(&self, hours_after: u32) -> Result<()> {
        if hours_after < 1 || hours_after > 84 {
            return Err(anyhow!("hours_after must be between 001 and 084"));
        }

        // Seeing forecasts only support multiples of 3
        if *self == ForecastType::Seeing && hours_after % 3 != 0 {
            return Err(anyhow!("seeing forecasts only support multiples of 3 hours (003, 006, 009, ..., 084)"));
        }

        Ok(())
    }
}

impl Region {
    /// Get the region name for URL construction
    fn url_name(&self) -> &'static str {
        match self {
            Region::Northeast => "northeast",
            Region::Northwest => "northwest",
            Region::Southeast => "southeast",
            Region::Southwest => "southwest",
        }
    }
}

impl EnvironmentCanadaAPI {
    /// Create a new Environment Canada API client
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Client::new(),
            base_url: "https://weather.gc.ca/data/prog/regional".to_string(),
        })
    }

    /// Fetch and save a forecast PNG image
    /// Returns the file path and PNG data
    pub async fn fetch_and_save_forecast(
        &self,
        forecast_type: ForecastType,
        model_run: &str,
        region: Region,
        hours_after: u32,
    ) -> Result<(String, Vec<u8>)> {
        forecast_type.validate_hours(hours_after)?;

        let png_data = self.fetch_forecast(forecast_type, model_run, region, hours_after).await?;

        let filename = self.generate_filename(forecast_type, region, model_run, hours_after);
        fs::write(&filename, &png_data)?;

        Ok((filename, png_data))
    }

    /// Fetch forecast PNG data without saving to file
    pub async fn fetch_forecast(
        &self,
        forecast_type: ForecastType,
        model_run: &str,
        region: Region,
        hours_after: u32,
    ) -> Result<Vec<u8>> {
        forecast_type.validate_hours(hours_after)?;

        let url = self.build_url(forecast_type, model_run, region, hours_after);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch forecast: HTTP {} - URL: {}",
                response.status(),
                url
            ));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Convenience method for cloud forecasts
    pub async fn fetch_and_save_cloud_forecast(
        &self,
        model_run: &str,
        region: Region,
        hours_after: u32,
    ) -> Result<(String, Vec<u8>)> {
        self.fetch_and_save_forecast(ForecastType::Cloud, model_run, region, hours_after).await
    }

    /// Convenience method for seeing forecasts
    pub async fn fetch_and_save_seeing_forecast(
        &self,
        model_run: &str,
        region: Region,
        hours_after: u32,
    ) -> Result<(String, Vec<u8>)> {
        self.fetch_and_save_forecast(ForecastType::Seeing, model_run, region, hours_after).await
    }

    /// Convenience method for transparency forecasts
    pub async fn fetch_and_save_transparency_forecast(
        &self,
        model_run: &str,
        region: Region,
        hours_after: u32,
    ) -> Result<(String, Vec<u8>)> {
        self.fetch_and_save_forecast(ForecastType::Transparency, model_run, region, hours_after).await
    }

    /// Convenience method for surface wind forecasts
    pub async fn fetch_and_save_surface_wind_forecast(
        &self,
        model_run: &str,
        region: Region,
        hours_after: u32,
    ) -> Result<(String, Vec<u8>)> {
        self.fetch_and_save_forecast(ForecastType::SurfaceWind, model_run, region, hours_after).await
    }

    /// Convenience method for temperature forecasts
    pub async fn fetch_and_save_temperature_forecast(
        &self,
        model_run: &str,
        region: Region,
        hours_after: u32,
    ) -> Result<(String, Vec<u8>)> {
        self.fetch_and_save_forecast(ForecastType::Temperature, model_run, region, hours_after).await
    }

    /// Convenience method for relative humidity forecasts
    pub async fn fetch_and_save_relative_humidity_forecast(
        &self,
        model_run: &str,
        region: Region,
        hours_after: u32,
    ) -> Result<(String, Vec<u8>)> {
        self.fetch_and_save_forecast(ForecastType::RelativeHumidity, model_run, region, hours_after).await
    }

    /// Build the complete URL for a forecast request
    fn build_url(
        &self,
        forecast_type: ForecastType,
        model_run: &str,
        region: Region,
        hours_after: u32,
    ) -> String {
        let hours_str = format!("{:03}", hours_after);
        let region_part = match forecast_type {
            ForecastType::Cloud => region.url_name(),
            _ => "astro",
        };
        let suffix = forecast_type.url_suffix();

        format!(
            "{}/{}/{}_054_R1_north@america@{}{}{}.png",
            self.base_url, model_run, model_run, region_part, suffix, hours_str
        )
    }

    /// Generate a filename for the forecast image
    fn generate_filename(
        &self,
        forecast_type: ForecastType,
        region: Region,
        model_run: &str,
        hours_after: u32,
    ) -> String {
        let hours_str = format!("{:03}", hours_after);
        let type_name = forecast_type.name();
        let region_name = match forecast_type {
            ForecastType::Cloud => region.url_name(),
            _ => "astro",
        };

        format!("{}_{}_{}_{}.png", type_name, region_name, model_run, hours_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forecast_type_url_suffixes() {
        assert_eq!(ForecastType::Cloud.url_suffix(), "_I_ASTRO_nt_");
        assert_eq!(ForecastType::Seeing.url_suffix(), "_I_ASTRO_seeing_");
        assert_eq!(ForecastType::Transparency.url_suffix(), "_I_ASTRO_transp_");
        assert_eq!(ForecastType::SurfaceWind.url_suffix(), "_I_ASTRO_uv_");
        assert_eq!(ForecastType::Temperature.url_suffix(), "_I_ASTRO_tt_");
        assert_eq!(ForecastType::RelativeHumidity.url_suffix(), "_I_ASTRO_hr_");
    }

    #[test]
    fn test_region_url_names() {
        assert_eq!(Region::Northeast.url_name(), "northeast");
        assert_eq!(Region::Northwest.url_name(), "northwest");
        assert_eq!(Region::Southeast.url_name(), "southeast");
        assert_eq!(Region::Southwest.url_name(), "southwest");
    }

    #[test]
    fn test_hours_validation() {
        // Valid hours
        assert!(ForecastType::Cloud.validate_hours(1).is_ok());
        assert!(ForecastType::Cloud.validate_hours(84).is_ok());

        // Invalid hours
        assert!(ForecastType::Cloud.validate_hours(0).is_err());
        assert!(ForecastType::Cloud.validate_hours(85).is_err());

        // Seeing only multiples of 3
        assert!(ForecastType::Seeing.validate_hours(3).is_ok());
        assert!(ForecastType::Seeing.validate_hours(6).is_ok());
        assert!(ForecastType::Seeing.validate_hours(84).is_ok());
        assert!(ForecastType::Seeing.validate_hours(1).is_err());
        assert!(ForecastType::Seeing.validate_hours(2).is_err());
        assert!(ForecastType::Seeing.validate_hours(4).is_err());
    }

    #[test]
    fn test_url_construction() {
        let api = EnvironmentCanadaAPI::new().unwrap();

        let url = api.build_url(ForecastType::Cloud, "2025101500", Region::Northeast, 1);
        assert_eq!(url, "https://weather.gc.ca/data/prog/regional/2025101500/2025101500_054_R1_north@america@northeast_I_ASTRO_nt_001.png");

        let url = api.build_url(ForecastType::Seeing, "2025101506", Region::Northwest, 3);
        assert_eq!(url, "https://weather.gc.ca/data/prog/regional/2025101506/2025101506_054_R1_north@america@astro_I_ASTRO_seeing_003.png");
    }

    #[test]
    fn test_filename_generation() {
        let api = EnvironmentCanadaAPI::new().unwrap();

        let filename = api.generate_filename(ForecastType::Cloud, Region::Northeast, "2025101500", 1);
        assert_eq!(filename, "cloud_northeast_2025101500_001.png");

        let filename = api.generate_filename(ForecastType::Seeing, Region::Northwest, "2025101506", 3);
        assert_eq!(filename, "seeing_astro_2025101506_003.png");
    }

    #[tokio::test]
    async fn test_api_creation() {
        let api = EnvironmentCanadaAPI::new();
        assert!(api.is_ok());
    }
}
