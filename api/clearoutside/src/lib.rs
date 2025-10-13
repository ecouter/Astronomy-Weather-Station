// ClearOutside API - Functional Rust wrapper for Python clearoutside scraper
//
// This module provides a pure functional API that calls the existing Python
// clearoutside scraper and parses its JSON output into strongly-typed Rust structures.
//
// Design principles:
// - No global state - all functions take parameters and return values
// - Functional composition - functions take ApiState and return Result<ApiState, Error>
// - Strong typing - complete data model matching Python output format

use serde::{Deserialize, Serialize};
use std::process::Command;
use std::fmt;

/// API state holding configuration and cached data
#[derive(Clone, Debug)]
pub struct ApiState {
    /// Latitude with 2 decimal places (e.g., "43.16")
    pub lat: String,
    /// Longitude with 2 decimal places (e.g., "-75.84")
    pub lon: String,
    /// View type: "midday", "midnight", "current"
    pub view: String,
    /// Cached JSON string from Python scraper (None until first update)
    cached_data: Option<String>,
}

/// Main forecast data structure matching Python output
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ClearOutsideData {
    #[serde(rename = "gen-info")]
    pub gen_info: GenInfo,
    #[serde(rename = "sky-quality")]
    pub sky_quality: SkyQuality,
    /// Forecast data for each day (day-0 to day-6)
    pub forecast: std::collections::HashMap<String, DayForecast>,
}

/// General information about the forecast
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GenInfo {
    #[serde(rename = "last-gen")]
    pub last_gen: Timestamp,
    pub forecast: ForecastRange,
    pub timezone: String,
}

/// Timestamp structure
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Timestamp {
    pub date: String, // dd/MM/yy format
    pub time: String, // HH:mm:ss format
}

/// Forecast date range
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ForecastRange {
    #[serde(rename = "from-day")]
    pub from_day: String, // dd/MM/yy format
    #[serde(rename = "to-day")]
    pub to_day: String, // dd/MM/yy format
}

/// Sky quality information
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SkyQuality {
    pub magnitude: String,
    pub bortle_class: String,
    pub brightness: Vec<String>, // [value, unit]
    #[serde(rename = "artif-brightness")]
    pub artif_brightness: Vec<String>, // [value, unit]
}

/// Forecast data for a single day
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DayForecast {
    pub date: DateInfo,
    pub sun: SunData,
    pub moon: MoonData,
    /// Hourly data keyed by hour string (e.g., "12", "13", etc.)
    pub hours: std::collections::HashMap<String, HourData>,
}

/// Date information for a day
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DateInfo {
    pub long: String,   // e.g., "Wednesday"
    pub short: String,  // e.g., "19"
}

/// Sun data for the day
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SunData {
    pub rise: String,      // HH:mm format (24h)
    pub set: String,       // HH:mm format (24h)
    pub transit: String,   // HH:mm format (24h)
    pub civil_dark: Vec<String>,   // [start, end] in HH:mm format
    pub nautical_dark: Vec<String>, // [start, end] in HH:mm format
    pub astro_dark: Vec<String>,    // [start, end] in HH:mm format
}

/// Moon data for the day
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MoonData {
    pub rise: String,    // HH:mm format or "--:--" if not rising
    pub set: String,     // HH:mm format or "--:--" if not setting
    pub phase: MoonPhase,
}

/// Moon phase information
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MoonPhase {
    pub name: String,     // e.g., "Waning Gibbous"
    pub percentage: String, // e.g., "53%"
}

/// Hourly weather data
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct HourData {
    pub conditions: String,        // "bad", "good", etc.
    #[serde(rename = "total-clouds")]
    pub total_clouds: u8,          // 0-100%
    #[serde(rename = "low-clouds")]
    pub low_clouds: u8,            // 0-100%
    #[serde(rename = "mid-clouds")]
    pub mid_clouds: u8,            // 0-100%
    #[serde(rename = "high-clouds")]
    pub high_clouds: u8,           // 0-100%
    pub visibility: f32,           // km (0.0 if data missing)
    pub fog: u8,                   // 0-100%
    #[serde(rename = "prec-type")]
    pub prec_type: String,         // "none", "rain", "snow", etc.
    #[serde(rename = "prec-probability")]
    pub prec_probability: u8,      // 0-100%
    #[serde(rename = "prec-amount")]
    pub prec_amount: f32,          // mm
    pub wind: WindData,
    pub frost: String,
    pub temperature: TemperatureData,
    #[serde(rename = "rel-humidity")]
    pub rel_humidity: u8,          // 0-100%
    pub pressure: u16,             // millibars
    pub ozone: u16,                // Dobson units
}

/// Wind information
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WindData {
    pub speed: f32,         // km/h
    pub direction: String,  // cardinal direction
}

/// Temperature information
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TemperatureData {
    pub general: i8,        // °C degrees Celsius
    #[serde(rename = "feels-like")]
    pub feels_like: i8,     // °C
    #[serde(rename = "dew-point")]
    pub dew_point: i8,      // °C
}

/// Error types for the API
#[derive(Debug)]
pub enum ClearOutsideError {
    PythonNotInstalled,
    ModuleNotFound(String),
    ScraperFailed(String),
    JsonParseError(String),
    IoError(std::io::Error),
}

impl fmt::Display for ClearOutsideError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClearOutsideError::PythonNotInstalled => write!(f, "Python 3 not found"),
            ClearOutsideError::ModuleNotFound(name) => write!(f, "Python module '{}' not found", name),
            ClearOutsideError::ScraperFailed(msg) => write!(f, "Scraper failed: {}", msg),
            ClearOutsideError::JsonParseError(msg) => write!(f, "JSON parse error: {}", msg),
            ClearOutsideError::IoError(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl std::error::Error for ClearOutsideError {}

impl From<std::io::Error> for ClearOutsideError {
    fn from(err: std::io::Error) -> Self {
        ClearOutsideError::IoError(err)
    }
}

/// Creates a new API state with the given parameters
/// No global state is used - this function returns a new state struct
pub fn create_api(lat: &str, lon: &str, view: &str) -> ApiState {
    ApiState {
        lat: lat.to_string(),
        lon: lon.to_string(),
        view: view.to_string(),
        cached_data: None,
    }
}

/// Executes the Python scraper and returns the JSON output as a string
async fn execute_python_scraper(
    lat: &str,
    lon: &str,
    view: &str,
) -> Result<String, ClearOutsideError> {
    let output = Command::new("python3")
        .args(&["-c", &format!(r#"
import sys
import json
sys.path.append('.')  # Add current directory to Python path

try:
    from clear_outside_apy import ClearOutsideAPY
    api = ClearOutsideAPY('{}', '{}', '{}')
    api.update()
    result = api.pull()
    print(json.dumps(result, indent=None, separators=(',', ':')))
except ImportError as e:
    print("ERROR_MODULE: clear_outside_apy")
    sys.exit(1)
except Exception as e:
    print("ERROR_SCRAPER:", str(e))
    sys.exit(1)
"#,
        lat, lon, view)])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.contains("ERROR_MODULE") {
            return Err(ClearOutsideError::ModuleNotFound("clear_outside_apy".to_string()));
        } else if stdout.contains("ERROR_SCRAPER:") {
            let err_msg = stdout.replace("ERROR_SCRAPER:", "").trim().to_string();
            return Err(ClearOutsideError::ScraperFailed(err_msg));
        } else {
            return Err(ClearOutsideError::ScraperFailed(stderr.to_string()));
        }
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Updates the API state by calling the Python scraper and caching the JSON data
/// Returns a new ApiState with updated cached_data
pub async fn update(api_state: &ApiState) -> Result<ApiState, ClearOutsideError> {
    let json_data = execute_python_scraper(&api_state.lat, &api_state.lon, &api_state.view).await?;

    // Validate that we can parse the JSON
    serde_json::from_str::<serde_json::Value>(&json_data)
        .map_err(|e| ClearOutsideError::JsonParseError(e.to_string()))?;

    Ok(ApiState {
        lat: api_state.lat.clone(),
        lon: api_state.lon.clone(),
        view: api_state.view.clone(),
        cached_data: Some(json_data),
    })
}

/// Parses the cached JSON data into the strongly-typed ClearOutsideData structure
pub fn pull(api_state: &ApiState) -> Result<ClearOutsideData, ClearOutsideError> {
    let json_str = api_state.cached_data.as_ref()
        .ok_or_else(|| ClearOutsideError::ScraperFailed("No cached data - call update() first".to_string()))?;

    serde_json::from_str(json_str)
        .map_err(|e| ClearOutsideError::JsonParseError(e.to_string()))
}

/// Convenience function that combines update and pull
/// More efficient than calling both separately
pub async fn update_and_pull(api_state: &ApiState) -> Result<(ApiState, ClearOutsideData), ClearOutsideError> {
    let updated_state = update(api_state).await?;
    let data = pull(&updated_state)?;
    Ok((updated_state, data))
}
