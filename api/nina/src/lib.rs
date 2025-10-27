use serde::{Deserialize, Serialize};

/// Generic API response wrapper from NINA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NinaResponse<T> {
    #[serde(rename = "Response")]
    pub response: T,
    #[serde(rename = "Error")]
    pub error: String,
    #[serde(rename = "StatusCode")]
    pub status_code: u16,
    #[serde(rename = "Success")]
    pub success: bool,
    #[serde(rename = "Type")]
    pub r#type: String,
}

/// RMS data for guiding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RmsData {
    #[serde(rename = "RA")]
    pub ra: f64,
    #[serde(rename = "Dec")]
    pub dec: f64,
    #[serde(rename = "Total")]
    pub total: f64,
    #[serde(rename = "RAText")]
    pub ra_text: String,
    #[serde(rename = "DecText")]
    pub dec_text: String,
    #[serde(rename = "TotalText")]
    pub total_text: String,
    #[serde(rename = "PeakRAText")]
    pub peak_ra_text: String,
    #[serde(rename = "PeakDecText")]
    pub peak_dec_text: String,
    #[serde(rename = "Scale")]
    pub scale: f64,
    #[serde(rename = "PeakRA")]
    pub peak_ra: f64,
    #[serde(rename = "PeakDec")]
    pub peak_dec: f64,
    #[serde(rename = "DataPoints")]
    pub data_points: u32,
}

/// Individual guide step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideStep {
    #[serde(rename = "Id")]
    pub id: u32,
    #[serde(rename = "IdOffsetLeft")]
    pub id_offset_left: f64,
    #[serde(rename = "IdOffsetRight")]
    pub id_offset_right: f64,
    #[serde(rename = "RADistanceRaw")]
    pub ra_distance_raw: f64,
    #[serde(rename = "RADistanceRawDisplay")]
    pub ra_distance_raw_display: f64,
    #[serde(rename = "RADuration")]
    pub ra_duration: i32,
    #[serde(rename = "DECDistanceRaw")]
    pub dec_distance_raw: f64,
    #[serde(rename = "DECDistanceRawDisplay")]
    pub dec_distance_raw_display: f64,
    #[serde(rename = "DECDuration")]
    pub dec_duration: i32,
    #[serde(rename = "Dither")]
    pub dither: String,
}

/// Complete guiding graph data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideStepsHistory {
    #[serde(rename = "RMS")]
    pub rms: RmsData,
    #[serde(rename = "Interval")]
    pub interval: u32,
    #[serde(rename = "MaxY")]
    pub max_y: i32,
    #[serde(rename = "MinY")]
    pub min_y: i32,
    #[serde(rename = "MaxDurationY")]
    pub max_duration_y: i32,
    #[serde(rename = "MinDurationY")]
    pub min_duration_y: i32,
    #[serde(rename = "GuideSteps")]
    pub guide_steps: Vec<GuideStep>,
    #[serde(rename = "HistorySize")]
    pub history_size: u32,
    #[serde(rename = "PixelScale")]
    pub pixel_scale: f64,
    #[serde(rename = "Scale")]
    pub scale: u32,
}

/// Parameters for prepared image request
#[derive(Debug, Clone, Default)]
pub struct PreparedImageParams {
    pub resize: Option<bool>,
    pub quality: Option<i32>,
    pub size: Option<String>,
    pub scale: Option<f64>,
    pub factor: Option<f64>,
    pub black_clipping: Option<f64>,
    pub unlinked: Option<bool>,
    pub debayer: Option<bool>,
    pub bayer_pattern: Option<String>,
    pub auto_prepare: Option<bool>,
}

/// Fetch guiding graph data from NINA
pub async fn fetch_guiding_graph(base_url: &str) -> Result<GuideStepsHistory, anyhow::Error> {
    let url = format!("{}/equipment/guider/graph", base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    let nina_response: NinaResponse<GuideStepsHistory> = response.json().await?;
    if nina_response.success {
        Ok(nina_response.response)
    } else {
        Err(anyhow::anyhow!("NINA API error: {}", nina_response.error))
    }
}

/// Fetch prepared image from NINA as bytes
pub async fn fetch_prepared_image(base_url: &str, params: &PreparedImageParams) -> Result<Vec<u8>, anyhow::Error> {
    let mut url = format!("{}/prepared-image", base_url.trim_end_matches('/'));
    let mut query_params = Vec::new();

    if let Some(resize) = params.resize {
        query_params.push(("resize".to_string(), resize.to_string()));
    }
    if let Some(quality) = params.quality {
        query_params.push(("quality".to_string(), quality.to_string()));
    }
    if let Some(ref size) = params.size {
        query_params.push(("size".to_string(), size.clone()));
    }
    if let Some(scale) = params.scale {
        query_params.push(("scale".to_string(), scale.to_string()));
    }
    if let Some(factor) = params.factor {
        query_params.push(("factor".to_string(), factor.to_string()));
    }
    if let Some(black_clipping) = params.black_clipping {
        query_params.push(("blackClipping".to_string(), black_clipping.to_string()));
    }
    if let Some(unlinked) = params.unlinked {
        query_params.push(("unlinked".to_string(), unlinked.to_string()));
    }
    if let Some(debayer) = params.debayer {
        query_params.push(("debayer".to_string(), debayer.to_string()));
    }
    if let Some(ref bayer_pattern) = params.bayer_pattern {
        query_params.push(("bayerPattern".to_string(), bayer_pattern.clone()));
    }
    if let Some(auto_prepare) = params.auto_prepare {
        query_params.push(("autoPrepare".to_string(), auto_prepare.to_string()));
    }

    // Always stream to get binary data
    query_params.push(("stream".to_string(), "true".to_string()));

    if !query_params.is_empty() {
        url.push('?');
        let query_string = query_params.into_iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(&v)))
            .collect::<Vec<_>>()
            .join("&");
        url.push_str(&query_string);
    }

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}
