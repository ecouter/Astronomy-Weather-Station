use image::{DynamicImage, GenericImage};
use reqwest::Client;
use std::path::Path;
use futures::future::try_join_all;

/// OpenStreetMap tile API client
pub struct OpenStreetMapAPI {
    client: Client,
}

impl OpenStreetMapAPI {
    /// Create a new OpenStreetMapAPI instance
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Convert lat/lon to OSM tile number at zoom z (pure function)
    pub fn lat_lon_to_tile(lat_deg: f64, lon_deg: f64, zoom: u32) -> (u32, u32) {
        let lat_rad = lat_deg.to_radians();
        let n = 2u32.pow(zoom);
        let x = ((lon_deg + 180.0) / 360.0 * n as f64).floor() as u32;
        let y = ((1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0 * n as f64).floor() as u32;
        (x, y)
    }

    /// Convert lat/lon to pixel coordinates at zoom z (pure function)
    pub fn lat_lon_to_pixel(lat_deg: f64, lon_deg: f64, zoom: u32) -> (f64, f64) {
        let lat_rad = lat_deg.to_radians();
        let n = 256.0 * 2u32.pow(zoom) as f64;
        let x = ((lon_deg + 180.0) / 360.0) * n;
        let y = ((1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0) * n;
        (x, y)
    }

    /// Download a single tile as an image (async)
    pub async fn download_tile(&self, z: u32, x: u32, y: u32) -> Result<DynamicImage, anyhow::Error> {
        let url = format!("https://b.tile.openstreetmap.de/{}/{}/{}.png", z, x, y);
        println!("Downloading {}", url);
        let resp = self.client.get(&url).send().await?;
        if resp.status().is_success() {
            let bytes = resp.bytes().await?;
            Ok(image::load_from_memory(&bytes)?)
        } else {
            Err(anyhow::anyhow!("Failed to download tile: HTTP {}", resp.status()))
        }
    }

    /// Download all tiles for a bounding box and return as a vector of rows (functional style)
    pub async fn download_tiles(&self, bbox: (f64, f64, f64, f64), zoom: u32) -> Result<Vec<Vec<DynamicImage>>, anyhow::Error> {
        let (lat_min, lon_min, lat_max, lon_max) = bbox;

        let (x0, y0) = Self::lat_lon_to_tile(lat_max, lon_min, zoom);
        let (x1, y1) = Self::lat_lon_to_tile(lat_min, lon_max, zoom);

        let x_start = x0.min(x1);
        let x_end = x0.max(x1);
        let y_start = y0.min(y1);
        let y_end = y0.max(y1);

        // Create all tile futures in a flat vector
        let mut all_futures = Vec::new();
        let mut positions = Vec::new();

        for y in y_start..=y_end {
            for x in x_start..=x_end {
                all_futures.push(self.download_tile(zoom, x, y));
                positions.push((y, x));
            }
        }

        // Download all tiles in parallel
        let tiles_flat: Vec<DynamicImage> = try_join_all(all_futures).await?;

        // Reconstruct the 2D structure
        let mut tiles = Vec::new();
        let mut idx = 0;
        for _ in y_start..=y_end {
            let mut row = Vec::new();
            for _ in x_start..=x_end {
                row.push(tiles_flat[idx].clone());
                idx += 1;
            }
            tiles.push(row);
        }

        Ok(tiles)
    }

    /// Stitch tiles into a single image (functional style)
    pub fn stitch_tiles(tiles: &[Vec<DynamicImage>]) -> DynamicImage {
        let tile_size = 256;
        let width = tiles[0].len() as u32 * tile_size;
        let height = tiles.len() as u32 * tile_size;

        let mut final_image = DynamicImage::new_rgb8(width, height);

        tiles.iter().enumerate().for_each(|(row_idx, row)| {
            row.iter().enumerate().for_each(|(col_idx, tile)| {
                let _ = final_image.copy_from(tile, col_idx as u32 * tile_size, row_idx as u32 * tile_size);
            });
        });

        final_image
    }

    /// Download and stitch map for bounding box
    pub async fn download_map(&self, bbox: (f64, f64, f64, f64), zoom: u32) -> Result<DynamicImage, anyhow::Error> {
        let tiles = self.download_tiles(bbox, zoom).await?;
        Ok(Self::stitch_tiles(&tiles))
    }

    /// Download and save cropped map to file (cropped to exact bounding box)
    pub async fn download_and_save_map(&self, bbox: (f64, f64, f64, f64), zoom: u32, output_path: &Path) -> Result<(), anyhow::Error> {
        let (lat1, lon1, lat2, lon2) = bbox;

        // Normalize bbox: ensure lat_min <= lat_max, lon_min <= lon_max
        let lat_min = lat1.min(lat2);
        let lat_max = lat1.max(lat2);
        let lon_min = lon1.min(lon2);
        let lon_max = lon1.max(lon2);

        let normalized_bbox = (lat_min, lon_min, lat_max, lon_max);

        // Get pixel coordinates of bbox corners
        let x_left = Self::lat_lon_to_pixel(lat_min, lon_min, zoom).0; // western edge
        let x_right = Self::lat_lon_to_pixel(lat_min, lon_max, zoom).0; // eastern edge
        let y_top = Self::lat_lon_to_pixel(lat_max, lon_min, zoom).1; // northern edge
        let y_bottom = Self::lat_lon_to_pixel(lat_min, lon_min, zoom).1; // southern edge

        // Get tile coordinates of the top-left tile (northernmost, westernmost)
        let tile_x_min = Self::lat_lon_to_tile(lat_max, lon_min, zoom).0; // western tile
        let tile_y_min = Self::lat_lon_to_tile(lat_max, lon_min, zoom).1; // northern tile

        // Pixel offset within the stitched image
        let offset_x = Self::lat_lon_to_pixel(lat_max, lon_min, zoom).0 - (tile_x_min as f64 * 256.0);
        let offset_y = Self::lat_lon_to_pixel(lat_max, lon_min, zoom).1 - (tile_y_min as f64 * 256.0);

        // Width and height in pixels
        let width_px = (x_right - x_left).max(1.0) as u32;
        let height_px = (y_bottom - y_top).max(1.0) as u32;

        // Download and stitch the full tiles
        let mut stitched_image = self.download_map(normalized_bbox, zoom).await?;

        // Crop to exact bounding box
        let cropped_image = stitched_image.crop(offset_x as u32, offset_y as u32, width_px, height_px);

        cropped_image.save(output_path)?;
        println!("Saved cropped map to {:?}", output_path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lat_lon_to_tile() {
        // Test with known values
        let (x, y) = OpenStreetMapAPI::lat_lon_to_tile(0.0, 0.0, 0);
        assert_eq!(x, 0);
        assert_eq!(y, 0);
    }

    #[test]
    fn test_api_creation() {
        let api = OpenStreetMapAPI::new();
        // Just test that it can be created
        assert!(true);
    }
}
