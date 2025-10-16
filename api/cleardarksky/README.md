# ClearDarkSky API

A Rust library for fetching sky charts from ClearDarkSky.com.

## Features

- **fetch_nearest_sky_chart_location**: Find the nearest sky chart location based on latitude and longitude coordinates
- **fetch_clear_sky_chart**: Download and save sky chart GIF images

## Usage

```rust
use cleardarksky::ClearDarkSkyAPI;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let api = ClearDarkSkyAPI::new();

    // Find nearest sky chart location
    let location = api.fetch_nearest_sky_chart_location(45.50, -73.57).await?;
    println!("Nearest location: {}", location);

    // Download the sky chart
    let filename = api.fetch_clear_sky_chart(&location).await?;
    println!("Saved to: {}", filename);

    Ok(())
}
```

## API Functions

### fetch_nearest_sky_chart_location(latitude: f64, longitude: f64)

- **Input**: Latitude and longitude coordinates (f64)
- **Output**: Location identifier string ending with "csk.gif" (e.g., "ThrsQCcsk.gif")
- **Description**: Makes a GET request to ClearDarkSky's find_chart.py endpoint and parses the HTML response to extract the nearest sky chart location code.

### fetch_clear_sky_chart(location_number: &str)

- **Input**: Location identifier string (must end with "csk.gif")
- **Output**: Filename where the GIF was saved
- **Description**: Downloads the sky chart GIF from cleardarksky.com and saves it to a file with the same name as the location identifier.

## Dependencies

- `reqwest`: HTTP client for making requests
- `scraper`: HTML parsing library
- `tokio`: Async runtime
- `anyhow`: Error handling

## Building

```bash
cargo build
```

## Testing

```bash
cargo test
```

## Running the Example

```bash
cargo run
