# Environment Canada Astronomical API

A Rust library and command-line tool for fetching astronomical weather forecast data from Environment Canada's weather.gc.ca service. This API provides access to specialized astronomical forecasts including cloud cover, seeing conditions, sky transparency, surface wind, temperature, and relative humidity for astronomical observation planning.

## Features

- **6 forecast types**: Cloud cover, Seeing conditions, Sky transparency, Surface wind, Temperature, Relative humidity
- **PNG image fetching** - Direct download of forecast visualization images
- **4 geographic regions** - Northeast, Northwest, Southeast, Southwest Canada
- **Model run support** - 00, 06, 12, 18 UTC model runs
- **Hourly forecasts** - Up to 84 hours after model run (seeing: multiples of 3 hours)
- **Automatic file saving** - PNG images saved with descriptive filenames
- **CLI tool** - Easy-to-use command line interface

## Installation

### From Source

1. Ensure you have Rust installed (https://rustup.rs/)
2. Clone the repository
3. Build the project:

```bash
cd api/environment_canada
cargo build --release
```

## Usage

### Command Line Tool

```bash
# Basic usage - fetch cloud forecast
./target/release/environment_canada cloud northeast 2025101500 001

# Fetch seeing forecast (note: hours must be multiples of 3)
./target/release/environment_canada seeing northwest 2025101506 003

# Fetch temperature forecast
./target/release/environment_canada temperature southeast 2025101512 024

# Fetch surface wind forecast
./target/release/environment_canada surface_wind southwest 2025101518 048
```

### Library Usage

```rust
use environment_canada::{EnvironmentCanadaAPI, ForecastType, Region};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Create API instance
    let api = EnvironmentCanadaAPI::new()?;

    // Fetch and save a cloud forecast
    let (filename, png_data) = api.fetch_and_save_cloud_forecast(
        "2025101500",    // model run (YYYYMMDDHH)
        Region::Northeast, // region
        1                // hours after model run
    ).await?;

    println!("Saved forecast to: {}", filename);

    // Fetch any forecast type
    let (filename, png_data) = api.fetch_and_save_forecast(
        ForecastType::Seeing,
        "2025101506",
        Region::Northwest,
        3  // must be multiple of 3 for seeing
    ).await?;

    Ok(())
}
```

## Forecast Types

### Cloud Cover (`cloud`)
- **URL pattern**: `northeast_I_ASTRO_nt_{hours}.png`
- **Hours**: 001-084
- **Description**: Cloud cover forecast for astronomical observations

### Seeing Conditions (`seeing`)
- **URL pattern**: `astro_I_ASTRO_seeing_{hours}.png`
- **Hours**: 003, 006, 009, ..., 084 (multiples of 3)
- **Description**: Atmospheric seeing quality forecast

### Sky Transparency (`transparency`)
- **URL pattern**: `astro_I_ASTRO_transp_{hours}.png`
- **Hours**: 001-084
- **Description**: Sky transparency forecast

### Surface Wind (`surface_wind`)
- **URL pattern**: `astro_I_ASTRO_uv_{hours}.png`
- **Hours**: 001-084
- **Description**: Surface wind speed and direction forecast

### Temperature (`temperature`)
- **URL pattern**: `astro_I_ASTRO_tt_{hours}.png`
- **Hours**: 001-084
- **Description**: Temperature forecast

### Relative Humidity (`relative_humidity`)
- **URL pattern**: `astro_I_ASTRO_hr_{hours}.png`
- **Hours**: 001-084
- **Description**: Relative humidity forecast

## Regions

- **Northeast**: Eastern Canada (Ontario, Quebec, Atlantic provinces)
- **Northwest**: Western Canada (British Columbia, Alberta, Saskatchewan, Manitoba)
- **Southeast**: Southern Ontario and Quebec
- **Southwest**: Southern British Columbia and Alberta

## Model Runs

Environment Canada runs their astronomical models 4 times daily:
- **00 UTC** (8 PM EDT)
- **06 UTC** (2 AM EDT)
- **12 UTC** (8 AM EDT)
- **18 UTC** (2 PM EDT)

Format: `YYYYMMDDHH` (e.g., `2025101500` for October 15, 2025 at 00 UTC)

## Output

### File Naming Convention
```
{forecast_type}_{region}_{model_run}_{hours_after}.png
```

Examples:
- `cloud_northeast_2025101500_001.png`
- `seeing_northwest_2025101506_003.png`
- `temperature_southeast_2025101512_024.png`

### PNG Images
- Images are downloaded directly from weather.gc.ca
- Format: PNG with forecast visualization
- Size: Varies by forecast type and region

## API Reference

### EnvironmentCanadaAPI

#### Methods

- `new() -> Result<Self>` - Create a new API client
- `fetch_forecast(forecast_type, model_run, region, hours_after) -> Result<Vec<u8>>` - Fetch PNG data without saving
- `fetch_and_save_forecast(forecast_type, model_run, region, hours_after) -> Result<(String, Vec<u8>)>` - Fetch and save PNG

#### Convenience Methods

- `fetch_and_save_cloud_forecast(model_run, region, hours_after)`
- `fetch_and_save_seeing_forecast(model_run, region, hours_after)`
- `fetch_and_save_transparency_forecast(model_run, region, hours_after)`
- `fetch_and_save_surface_wind_forecast(model_run, region, hours_after)`
- `fetch_and_save_temperature_forecast(model_run, region, hours_after)`
- `fetch_and_save_relative_humidity_forecast(model_run, region, hours_after)`

### Enums

#### ForecastType
- `Cloud`, `Seeing`, `Transparency`, `SurfaceWind`, `Temperature`, `RelativeHumidity`

#### Region
- `Northeast`, `Northwest`, `Southeast`, `Southwest`

## Dependencies

- `reqwest`: HTTP client for fetching PNG images
- `anyhow`: Error handling
- `tokio`: Async runtime
- `serde`: Serialization (for future enhancements)

## Error Handling

The library provides comprehensive error handling for:
- Invalid forecast type/region combinations
- Invalid hours_after values (especially seeing forecasts)
- Network request failures
- HTTP error responses
- File I/O errors

All errors implement `std::error::Error` and can be easily handled in your application.

## Testing

```bash
# Run unit tests
cargo test

# Run specific test
cargo test test_url_construction
```

## License

This project is part of the RaspberryPi Astronomy Weather Station project.

## Related Projects

- [GeoMet API](../geomet/): General Environment Canada weather data (WMS/WCS)
- [MeteoBlue API](../meteoblue/): Astronomy seeing forecasts from meteoblue
- [ClearOutside API](../clearoutside/): Astronomy weather data from clearoutside.com
