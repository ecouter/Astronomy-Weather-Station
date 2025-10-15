use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use serde_urlencoded;

/// GeoMet API client for accessing Environment Canada's weather services
pub struct GeoMetAPI {
    client: Client,
    base_url: String,
}

#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub min_lon: f64,
    pub max_lon: f64,
    pub min_lat: f64,
    pub max_lat: f64,
}

impl BoundingBox {
    pub fn new(min_lon: f64, max_lon: f64, min_lat: f64, max_lat: f64) -> Self {
        Self {
            min_lon,
            max_lon,
            min_lat,
            max_lat,
        }
    }

    pub fn to_string(&self) -> String {
        format!("{},{},{},{}", self.min_lon, self.min_lat, self.max_lon, self.max_lat)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WmsCapabilities {
    pub layers: Vec<WmsLayer>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WmsLayer {
    pub name: String,
    pub title: String,
    pub abstract_text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WcsCapabilities {
    pub coverages: Vec<WcsCoverage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WcsCoverage {
    pub coverage_id: String,
    pub title: Option<String>,
}

impl GeoMetAPI {
    /// Create a new GeoMet API client
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Client::new(),
            base_url: "https://geo.weather.gc.ca/geomet".to_string(),
        })
    }

    /// Get WMS capabilities
    pub async fn get_wms_capabilities(&self) -> Result<WmsCapabilities> {
        let url = format!("{}?SERVICE=WMS&VERSION=1.3.0&REQUEST=GetCapabilities", self.base_url);

        let response = self.client.get(&url).send().await?;
        let text = response.text().await?;

        // Parse XML to extract layer information
        self.parse_wms_capabilities(&text)
    }

    /// Get WCS capabilities
    pub async fn get_wcs_capabilities(&self) -> Result<WcsCapabilities> {
        let url = format!("{}?SERVICE=WCS&VERSION=2.0.1&REQUEST=GetCapabilities", self.base_url);

        let response = self.client.get(&url).send().await?;
        let text = response.text().await?;

        // Parse XML to extract coverage information
        self.parse_wcs_capabilities(&text)
    }

    /// Get raw WMS capabilities XML
    pub async fn get_wms_capabilities_raw(&self) -> Result<String> {
        let url = format!("{}?SERVICE=WMS&VERSION=1.3.0&REQUEST=GetCapabilities", self.base_url);

        let response = self.client.get(&url).send().await?;
        let text = response.text().await?;
        Ok(text)
    }

    /// Get raw WCS capabilities XML
    pub async fn get_wcs_capabilities_raw(&self) -> Result<String> {
        let url = format!("{}?SERVICE=WCS&VERSION=2.0.1&REQUEST=GetCapabilities", self.base_url);

        let response = self.client.get(&url).send().await?;
        let text = response.text().await?;
        Ok(text)
    }

    /// Fetch WMS image
    /// Note: For WMS 1.3.0 with EPSG:4326, BBOX order is minY,minX,maxY,maxX (lat,lon,lat,lon)
    pub async fn get_wms_image(
        &self,
        layer: &str,
        time: &str,
        bbox: BoundingBox,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>> {
        // For WMS 1.3.0 with EPSG:4326, BBOX format is: minY,minX,maxY,maxX
        let bbox_str = format!("{},{},{},{}", bbox.min_lat, bbox.min_lon, bbox.max_lat, bbox.max_lon);
        let width_str = width.to_string();
        let height_str = height.to_string();

        let mut params = HashMap::new();
        params.insert("SERVICE", "WMS");
        params.insert("VERSION", "1.3.0");
        params.insert("REQUEST", "GetMap");
        params.insert("LAYERS", layer);
        params.insert("STYLES", "");  // Use default style
        params.insert("CRS", "EPSG:4326");
        params.insert("BBOX", &bbox_str);
        params.insert("WIDTH", &width_str);
        params.insert("HEIGHT", &height_str);
        params.insert("FORMAT", "image/png");
        params.insert("TIME", time);

        let response = self.client.get(&self.base_url).query(&params).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("WMS request failed: {} - URL: {}", response.status(),
                format!("{}?{}", self.base_url, serde_urlencoded::to_string(&params).unwrap_or_default())));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Fetch WCS data with full 2.0.1 specification support
    pub async fn get_wcs_data(
        &self,
        coverage_id: &str,
        time: &str,
        bbox: BoundingBox,
        format: &str,
    ) -> Result<Vec<u8>> {
        self.get_wcs_data_advanced(
            coverage_id,
            time,
            bbox,
            format,
            None, // subsetting_crs
            None, // output_crs
            None, // resolution_x
            None, // resolution_y
            None, // size_x
            None, // size_y
            None, // interpolation
            None, // range_subset
        ).await
    }

    /// Fetch WCS data with advanced options (full WCS 2.0.1 compliance)
    pub async fn get_wcs_data_advanced(
        &self,
        coverage_id: &str,
        time: &str,
        bbox: BoundingBox,
        format: &str,
        subsetting_crs: Option<&str>,
        output_crs: Option<&str>,
        resolution_x: Option<f64>,
        resolution_y: Option<f64>,
        size_x: Option<u32>,
        size_y: Option<u32>,
        interpolation: Option<&str>,
        range_subset: Option<&str>,
    ) -> Result<Vec<u8>> {
        let x_subset = format!("x({:.6}, {:.6})", bbox.min_lon, bbox.max_lon);
        let y_subset = format!("y({:.6}, {:.6})", bbox.min_lat, bbox.max_lat);

        let mut params = HashMap::new();
        params.insert("SERVICE", "WCS");
        params.insert("VERSION", "2.0.1");
        params.insert("REQUEST", "GetCoverage");
        params.insert("COVERAGEID", coverage_id);

        // SUBSETTINGCRS (optional, defaults to EPSG:4326)
        if let Some(crs) = subsetting_crs {
            params.insert("SUBSETTINGCRS", crs);
        } else {
            params.insert("SUBSETTINGCRS", "EPSG:4326");
        }

        // OUTPUTCRS (optional, strongly recommended)
        if let Some(crs) = output_crs {
            params.insert("OUTPUTCRS", crs);
        }

        // SUBSET parameters
        params.insert("SUBSET", &x_subset);
        params.insert("SUBSET", &y_subset);

        // RESOLUTION or SIZE (mutually exclusive per axis)
        let mut resolution_params = Vec::new();
        let mut size_params = Vec::new();

        if let Some(res_x) = resolution_x {
            resolution_params.push(format!("x({:.6})", res_x));
        }
        if let Some(res_y) = resolution_y {
            resolution_params.push(format!("y({:.6})", res_y));
        }
        if let Some(sx) = size_x {
            size_params.push(format!("x({})", sx));
        }
        if let Some(sy) = size_y {
            size_params.push(format!("y({})", sy));
        }

        // Add resolution parameters
        for res_param in &resolution_params {
            params.insert("RESOLUTION", res_param);
        }

        // Add size parameters
        for size_param in &size_params {
            params.insert("SIZE", size_param);
        }

        // INTERPOLATION (optional, default NEAREST)
        if let Some(interp) = interpolation {
            params.insert("INTERPOLATION", interp);
        }

        // RANGESUBSET (optional)
        if let Some(range) = range_subset {
            params.insert("RANGESUBSET", range);
        }

        // TIME
        params.insert("TIME", time);

        // FORMAT
        params.insert("FORMAT", format);

        let response = self.client.get(&self.base_url).query(&params).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("WCS request failed: {} - URL: {}", response.status(),
                format!("{}?{}", self.base_url, serde_urlencoded::to_string(&params).unwrap_or_default())));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Get point data from WCS (single grid cell)
    pub async fn get_point_data(
        &self,
        coverage_id: &str,
        time: &str,
        lon: f64,
        lat: f64,
        format: &str,
    ) -> Result<Vec<u8>> {
        let x_subset = format!("x({:.6}, {:.6})", lon, lon);
        let y_subset = format!("y({:.6}, {:.6})", lat, lat);

        let mut params = HashMap::new();
        params.insert("SERVICE", "WCS");
        params.insert("VERSION", "2.0.1");
        params.insert("REQUEST", "GetCoverage");
        params.insert("COVERAGEID", coverage_id);
        params.insert("SUBSETTINGCRS", "EPSG:4326");
        params.insert("SUBSET", &x_subset);
        params.insert("SUBSET", &y_subset);
        params.insert("FORMAT", format);
        params.insert("TIME", time);

        let response = self.client.get(&self.base_url).query(&params).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("WCS point request failed: {}", response.status()));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Fetch WMS legend graphic
    /// Note: For WMS 1.3.0, GetLegendGraphic uses STYLE (singular) not STYLES (plural)
    pub async fn get_legend_graphic(
        &self,
        layer: &str,
        style: Option<&str>,
        format: &str,
        language: Option<&str>,
    ) -> Result<Vec<u8>> {
        let mut params = HashMap::new();
        params.insert("SERVICE", "WMS");
        params.insert("VERSION", "1.3.0");
        params.insert("REQUEST", "GetLegendGraphic");
        params.insert("LAYER", layer);
        params.insert("FORMAT", format);
        params.insert("SLD_VERSION", "1.1.0");

        // Optional parameters
        if let Some(style_name) = style {
            params.insert("STYLE", style_name);
        }
        if let Some(lang) = language {
            params.insert("LANG", lang);
        }

        let response = self.client.get(&self.base_url).query(&params).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("WMS GetLegendGraphic request failed: {} - URL: {}", response.status(),
                format!("{}?{}", self.base_url, serde_urlencoded::to_string(&params).unwrap_or_default())));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    // XML parsing helpers (simplified - would need proper XML parsing in production)
    fn parse_wms_capabilities(&self, xml: &str) -> Result<WmsCapabilities> {
        // This is a simplified parser - in production you'd use a proper XML library
        let mut layers = Vec::new();

        // Extract layer names from XML
        for line in xml.lines() {
            if line.contains("<Name>") && line.contains("</Name>") {
                if let Some(start) = line.find("<Name>") {
                    if let Some(end) = line.find("</Name>") {
                        let name = line[start + 6..end].trim();
                        if !name.is_empty() && (name.starts_with("RDPS") || name.starts_with("HRDPS")) {
                            layers.push(WmsLayer {
                                name: name.to_string(),
                                title: name.to_string(), // Simplified
                                abstract_text: None,
                            });
                        }
                    }
                }
            }
        }

        Ok(WmsCapabilities { layers })
    }

    fn parse_wcs_capabilities(&self, xml: &str) -> Result<WcsCapabilities> {
        // This is a simplified parser - in production you'd use a proper XML library
        let mut coverages = Vec::new();

        // Extract coverage IDs from XML
        for line in xml.lines() {
            if line.contains("<wcs:CoverageId>") && line.contains("</wcs:CoverageId>") {
                if let Some(start) = line.find("<wcs:CoverageId>") {
                    if let Some(end) = line.find("</wcs:CoverageId>") {
                        let coverage_id = line[start + 16..end].trim();
                        if !coverage_id.is_empty() && (coverage_id.starts_with("RDPS") || coverage_id.starts_with("HRDPS")) {
                            coverages.push(WcsCoverage {
                                coverage_id: coverage_id.to_string(),
                                title: Some(coverage_id.to_string()), // Simplified
                            });
                        }
                    }
                }
            }
        }

        Ok(WcsCapabilities { coverages })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounding_box() {
        let bbox = BoundingBox::new(-130.0, -60.0, 20.0, 60.0);
        assert_eq!(bbox.to_string(), "-130,20,-60,60");
    }

    #[tokio::test]
    async fn test_api_creation() {
        let api = GeoMetAPI::new();
        assert!(api.is_ok());
    }
}
