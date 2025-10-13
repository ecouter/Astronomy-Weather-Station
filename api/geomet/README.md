# GeoMet API

A Rust library for accessing Environment Canada's GeoMet weather services, providing access to RDPS (Regional Deterministic Prediction System) data through WMS (Web Map Service) and WCS (Web Coverage Service) APIs.

## Features

- **WMS Support**: Fetch weather map images (PNG/JPEG) for visualization
- **WCS Support**: Fetch gridded weather data (NetCDF/GeoTIFF) for analysis
- **RDPS Data**: Access to high-resolution Canadian weather model data
- **Flexible Parameters**: Support for various weather parameters, time steps, and spatial extents

## Usage

### Library Usage

```rust
use geomet::{GeoMetAPI, BoundingBox};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Create API instance
    let api = GeoMetAPI::new()?;

    // Define bounding box (min_lon, max_lon, min_lat, max_lat)
    let bbox = BoundingBox::new(-130.0, -60.0, 20.0, 60.0);

    // Fetch WMS image
    let image_data = api.get_wms_image(
        "RDPS_10km_AirTemp_2m",        // layer name
        "2025-10-13T12:00:00Z",       // ISO 8601 timestamp
        bbox,                          // spatial extent
        800,                          // image width
        600                           // image height
    ).await?;

    // Save image
    std::fs::write("weather_map.png", image_data)?;

    // Fetch WCS data
    let data = api.get_wcs_data(
        "RDPS_10km_Precip-Accum24h",  // coverage ID
        "2025-10-13T12:00:00Z",       // ISO 8601 timestamp
        bbox,                          // spatial extent
        "application/x-netcdf"         // output format
    ).await?;

    // Save NetCDF data
    std::fs::write("precipitation.nc", data)?;

    Ok(())
}
```

### Command Line Usage

```bash
# List available WMS layers
./geomet capabilities wms

# List available WCS coverages
./geomet capabilities wcs

# Fetch WMS image
./geomet wms RDPS_10km_AirTemp_2m 2025-10-13T12:00:00Z -130 -60 20 60 800 600

# Fetch WCS data
./geomet wcs RDPS_10km_Precip-Accum24h 2025-10-13T12:00:00Z -130 -60 20 60

# Fetch point data
./geomet point RDPS_10km_AirTemp_2m 2025-10-13T12:00:00Z -75.7 45.4
```

## Parameters

### Layer/Coverage Names
RDPS layer names follow a specific naming convention:

**Format**: `{MODEL}_{RESOLUTION}_{PARAMETER}_{LEVEL}{MODIFIERS}`

- **MODEL**: `RDPS` (Regional) or `HRDPS` (High Resolution)
- **RESOLUTION**: `10km` or `2.5km`
- **PARAMETER**: Weather variable (TT=temperature, WSPD=wind speed, etc.)
- **LEVEL**: Height or pressure level (_2m, _500mb, etc.)
- **MODIFIERS**: Optional suffixes (-Contour, -Accum24h, etc.)

**Examples**:
- `RDPS_10km_AirTemp_2m` - 2m air temperature
- `HRDPS_2.5km_WindSpeed_10m` - 10m wind speed (high resolution)
- `RDPS.CONTINENTAL.PRES_TT.500` - Temperature at 500mb pressure level
- `RDPS_10km_Precip-Accum24h` - 24-hour accumulated precipitation

### Time Parameters
Time must be specified in ISO 8601 format:

**Format**: `YYYY-MM-DDTHH:MM:SSZ`

- **YYYY**: 4-digit year
- **MM**: 2-digit month (01-12)
- **DD**: 2-digit day (01-31)
- **HH**: 2-digit hour (00-23)
- **MM**: 2-digit minute (00-59)
- **SS**: 2-digit second (00-59)
- **Z**: UTC timezone indicator

**Examples**:
- `2025-10-13T12:00:00Z` - October 13, 2025 at 12:00 UTC
- `2025-01-01T00:00:00Z` - New Year's Day 2025 at midnight UTC

### Spatial Parameters (Bounding Box)
Define rectangular geographic area. **Important**: For WMS 1.3.0 with EPSG:4326, the BBOX parameter uses axis order **minY,minX,maxY,maxX** (latitude,longitude,latitude,longitude).

- **min_lon**: Minimum longitude (western boundary, typically -180 to 180)
- **max_lon**: Maximum longitude (eastern boundary, typically -180 to 180)
- **min_lat**: Minimum latitude (southern boundary, typically -90 to 90)
- **max_lat**: Maximum latitude (northern boundary, typically -90 to 90)

**Examples**:
- Canada: `(-141.0, -52.0, 41.0, 83.0)`
- Ontario: `(-95.0, -74.0, 41.0, 57.0)`
- Point location: `(-75.7, -75.7, 45.4, 45.4)` (same coordinates for min/max)

**Note**: The BoundingBox struct internally handles the WMS 1.3.0 coordinate axis ordering automatically.

### Image Dimensions (WMS only)
- **width**: Image width in pixels (recommended: 800-2000)
- **height**: Image height in pixels (recommended: 600-1500)

### Output Formats (WCS only)
- `application/x-netcdf` - NetCDF format (recommended for analysis)
- `image/tiff` - GeoTIFF format
- `application/x-geotiff` - GeoTIFF format

## Available Layers

The API supports various RDPS layers including:

### Surface Weather
- **TT**: Air temperature (°C or K)
- **WSPD/WDIR**: Wind speed (m/s) and direction (°)
- **PR/RN/SN**: Precipitation (mm), Rain (mm), Snow (mm)
- **HU/HR**: Humidity (kg/kg or %), Relative humidity (%)
- **P0/PN**: Surface pressure (Pa), MSL pressure (Pa)

### Atmospheric Levels
- Pressure levels: 50mb, 100mb, 150mb, ..., 1000mb, 1015mb
- Parameters: TT (temperature), WSPD (wind speed), GZ (geopotential height), etc.

### Time Intervals
- **Instantaneous**: Current conditions
- **Accumulated**: PT1H (1h), PT3H (3h), PT6H (6h), PT12H (12h), PT24H (24h)

## API Reference

See the source code documentation for detailed API reference.

## License

This project is licensed under the MIT License.
