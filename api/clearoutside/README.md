# ClearOutside API

A Rust library and command-line tool for fetching astronomy weather forecast data from [ClearOutside](https://clearoutside.com).

## Features

- Fetch sky quality information (magnitude, Bortle class, brightness)
- Get detailed weather forecasts including:
  - Cloud cover (total, low, mid, high)
  - Visibility, fog, precipitation
  - Wind speed and direction
  - Temperature, humidity, pressure
  - Moon phase and rise/set times
  - Sun rise/set and twilight information
- Parse HTML data from ClearOutside website
- Export data to JSON format

## Installation

### From Source

1. Ensure you have Rust installed (https://rustup.rs/)
2. Clone the repository
3. Build the project:

```bash
cd api/clearoutside
cargo build --release
```

## Usage

### Command Line Tool

```bash
# Basic usage with default view (midday)
./target/release/clearoutside 45.50 -73.57

# Specify view type
./target/release/clearoutside 45.50 -73.57 current
./target/release/clearoutside 45.50 -73.57 midnight

# View options:
# - current: Current hour at the beginning
# - midday: Midday at the beginning (default)
# - midnight: Midnight at the beginning
```

### Library Usage

```rust
use clearoutside::ClearOutsideAPI;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Create API instance
    let mut api = ClearOutsideAPI::new("45.50", "-73.57", Some("midday")).await?;

    // Fetch forecast data
    let forecast = api.pull()?;

    // Access the data
    println!("Sky Quality: {}", forecast.sky_quality.magnitude);
    println!("Bortle Class: {}", forecast.sky_quality.bortle_class);

    for (day_key, day_info) in &forecast.forecast {
        println!("Day {}: {} - {}",
            day_key,
            day_info.date.long,
            day_info.date.short
        );
    }

    Ok(())
}
```

## Data Structures

### Main Types

- `ClearOutsideForecast`: Main structure containing all forecast data
- `SkyQuality`: Sky quality information (magnitude, Bortle class, brightness)
- `GeneralInfo`: General forecast information (generation time, period, timezone)
- `DayInfo`: Information for a specific day including sun, moon, and hourly data
- `HourlyData`: Detailed hourly weather information

### Example Output

```json
{
  "gen_info": {
    "last_gen": {
      "date": "2025-10-12",
      "time": "14:30"
    },
    "forecast": {
      "from_day": "2025-10-12",
      "to_day": "2025-10-18"
    },
    "timezone": "EDT"
  },
  "sky_quality": {
    "magnitude": "5.2",
    "bortle_class": "4",
    "brightness": ["21.5", "21.0"],
    "artif_brightness": ["20.8", "20.3"]
  },
  "forecast": {
    "day-0": {
      "date": {
        "long": "Sunday",
        "short": "10/12"
      },
      "sun": {
        "rise": "07:05",
        "set": "18:25",
        "transit": "12:45",
        "civil_dark": ["06:40", "18:50"],
        "nautical_dark": ["06:10", "19:20"],
        "astro_dark": ["05:40", "19:50"]
      },
      "moon": {
        "rise": "15:30",
        "set": "00:15",
        "phase": {
          "name": "Waxing Gibbous",
          "percentage": "78%"
        }
      },
      "hours": {}
    }
  }
}
```

## Dependencies

- `reqwest`: HTTP client for fetching data
- `scraper`: HTML parsing and CSS selector library
- `serde`: Serialization/deserialization framework
- `tokio`: Asynchronous runtime
- `anyhow`: Error handling
- `chrono`: Date and time handling

## License

This project is part of the RaspberryPi Astronomy Weather Station project.
```

## Error Handling

The library provides comprehensive error handling for:
- Invalid latitude/longitude parameters
- Network request failures
- HTML parsing errors
- Missing or malformed data

All errors implement `std::error::Error` and can be easily handled in your application.

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

## Related Projects

- [Meteoblue API](../meteoblue/): Similar Rust library for meteoblue astronomy seeing data
