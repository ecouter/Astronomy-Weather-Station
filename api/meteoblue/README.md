# MeteoBlue

A Rust library and CLI tool to scrape astronomy seeing forecast data from meteoblue's "Astronomy Seeing" API. This is a reverse engineering scraper that extracts seeing conditions (atmospheric turbulence affecting astronomical observation quality) for given coordinates.

## Features

- **Scrapes meteoblue astronomy seeing data** - Extracts forecast data for atmospheric seeing conditions
- **Coordinates-based** - Works with any latitude/longitude coordinates
- **Comprehensive data extraction** - Parses cloud cover, seeing quality, atmospheric indices, temperature, humidity, and more
- **JSON export** - Automatically saves data to JSON files for later use
- **CLI tool** - Easy-to-use command line interface with coordinate arguments

## Data Extracted

For each forecast time point, the following data is extracted:

- **Date and hour** of forecast
- **Seeing quality** in arc seconds (lower is better)
- **Seeing indices** (1-5 scale for atmospheric stability)
- **Cloud cover** (low, mid, high layers in percent)
- **Jet stream** speed (when >20 m/s affects seeing negatively)
- **Bad atmospheric layers** (bottom/top heights, temperature gradients)
- **Ground temperature** and humidity

## Installation

```bash
# Clone the repository
git clone <repository-url>
cd api/meteoblue

# Build
cargo build --release
```

## Usage

### Command Line Tool

```bash
# Use default coordinates (45.219N, -73.111W)
cargo run

# Or specify custom coordinates (latitude longitude)
cargo run -- 48.0 -123.0

# Build and run the binary directly
./target/release/meteoblue 60.0 18.0
```

### Library Usage

```rust
use meteoblue::fetch_meteoblue_data;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lat = 45.219;
    let lon = -73.111;

    let data = fetch_meteoblue_data(lat, lon).await?;
    for point in &data {
        println!("{} {}:00 - Seeing: {:.2}\"",
            point.day, point.hour, point.seeing_arcsec);
    }

    Ok(())
}
```

## Output Examples

```
✅ Successfully retrieved 71 data points

📊 Sample forecast data:
  2025-10-12 02:00 - Seeing: 2.42" (indices: 3/1, clouds: 56/30/0%, temp: 14.0°C, humidity: 82%)
  2025-10-12 03:00 - Seeing: 2.48" (indices: 3/1, clouds: 10/10/0%, temp: 13.0°C, humidity: 85%)
  ...

🌟 Best observability conditions:
  2025-10-12 16:00 - Seeing: 2.28", Indices: 3/1, Clouds: 0%
```

## Dependencies

- `reqwest` - HTTP client for making requests
- `scraper` - HTML parsing and scraping
- `serde` & `serde_json` - JSON serialization
- `anyhow` - Error handling
- `tokio` - Async runtime

## Data Structure

The JSON data is organized by day, with each day containing an array of hourly data points:

```json
{
  "2025-10-12": [
    {
      "hour": 2,
      "clouds_low_pct": 56,
      "clouds_mid_pct": 30,
      "clouds_high_pct": 0,
      "seeing_arcsec": 2.42,
      "index1": 3,
      "index2": 1,
      "jetstream_ms": 12.0,
      "bad_layers_bot_km": 2.0,
      "bad_layers_top_km": 5.7,
      "bad_layers_k_per_100m": null,
      "temp_c": 14.0,
      "humidity_pct": 82
    },
    {
      "hour": 3,
      "clouds_low_pct": 10,
      "clouds_mid_pct": 10,
      "clouds_high_pct": 0,
      "seeing_arcsec": 2.48,
      "index1": 3,
      "index2": 1,
      "jetstream_ms": 13.0,
      "bad_layers_bot_km": 9.4,
      "bad_layers_top_km": 10.6,
      "bad_layers_k_per_100m": null,
      "temp_c": 13.0,
      "humidity_pct": 85
    }
    // ... more hours for this day
  ],
  "2025-10-13": [
    // ... hourly data for the next day
  ]
}
```

## Notes

- This is a reverse engineering scraper - data availability depends on meteoblue's website structure
- The tool saves data to `seeing_{lat}_{lon}.json` files
- Coordinates are formatted with 3 decimal precision in the URL
- West longitudes are handled properly (converted to E format for the URL)
- Data covers approximately 3-5 days of forecast depending on the location

## Testing

```bash
# Run unit tests
cargo test

# Run with sample HTML file (no network required)
cargo test test_parse_seeing_sample_html
