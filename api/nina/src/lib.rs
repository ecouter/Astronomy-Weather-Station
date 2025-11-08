#[macro_use] extern crate log;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use futures_util::StreamExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

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

/// Image statistics from websocket events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageStatistics {
    #[serde(rename = "ExposureTime")]
    pub exposure_time: f64,
    #[serde(rename = "Index")]
    pub index: f64,
    #[serde(rename = "Filter")]
    pub filter: String,
    #[serde(rename = "RmsText")]
    pub rms_text: String,
    #[serde(rename = "Temperature")]
    pub temperature: f64,
    #[serde(rename = "CameraName")]
    pub camera_name: String,
    #[serde(rename = "Gain")]
    pub gain: f64,
    #[serde(rename = "Offset")]
    pub offset: f64,
    #[serde(rename = "Date")]
    pub date: String,
    #[serde(rename = "TelescopeName")]
    pub telescope_name: String,
    #[serde(rename = "FocalLength")]
    pub focal_length: f64,
    #[serde(rename = "StDev")]
    pub st_dev: f64,
    #[serde(rename = "Mean")]
    pub mean: f64,
    #[serde(rename = "Median")]
    pub median: f64,
    #[serde(rename = "Stars")]
    pub stars: f64,
    #[serde(rename = "HFR")]
    pub hfr: f64,
    #[serde(rename = "IsBayered")]
    pub is_bayered: bool,
}

/// Image save event from websocket (with statistics)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSaveEvent {
    #[serde(rename = "Event")]
    pub event: String,
    #[serde(rename = "ImageStatistics")]
    pub image_statistics: ImageStatistics,
}

/// Simple image prepared event from websocket (no statistics)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImagePreparedEvent {
    #[serde(rename = "Event")]
    pub event: String,
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
    info!("Starting image fetch from base URL: {}", base_url);
    let mut url = format!("{}/v2/api/prepared-image", base_url.trim_end_matches('/'));
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

    info!("Fetching image from URL: {}", url);
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    let status = response.status();
    info!("Image fetch response status: {}", status);

    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Failed to read error response".to_string());
        error!("Image fetch failed with status {}: {}", status, error_text);
        return Err(anyhow::anyhow!("HTTP {}: {}", status, error_text));
    }

    let bytes = response.bytes().await?;
    info!("Successfully fetched {} bytes of image data", bytes.len());
    Ok(bytes.to_vec())
}

/// Spawn a websocket listener for a NINA instance
/// Returns a JoinHandle and a Sender to stop the listener
pub fn spawn_nina_websocket_listener<F>(
    base_url: String,
    on_image_prepared: F,
) -> (std::thread::JoinHandle<()>, std::sync::mpsc::Sender<()>)
where
    F: Fn(ImagePreparedEvent) + Send + Sync + 'static,
{
    let on_image_prepared = Arc::new(on_image_prepared);
    let (stop_tx, stop_rx) = std::sync::mpsc::channel();

    let handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
        info!("NINA websocket task spawned successfully - starting listener");
        let host_port = base_url
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .split('/')
            .next()
            .unwrap_or("");
        let ws_url = format!("ws://{}/v2/socket", host_port);

        info!("Starting NINA websocket listener task for base URL: {}", base_url);
        info!("Connecting to NINA websocket at: {}", ws_url);

        let mut heartbeat_counter = 0u64;

        loop {
            // Check if stop signal received
            if stop_rx.try_recv().is_ok() {
                info!("Stop signal received, stopping NINA websocket listener for {}", base_url);
                break;
            }

            info!("Attempting to connect to NINA websocket at: {}", ws_url);
            match connect_async(&ws_url).await {
                Ok((ws_stream, response)) => {
                    info!("Successfully connected to NINA websocket at: {} (status: {})", ws_url, response.status());
                    let (_write, mut read) = ws_stream.split();

                    // Just connect and listen - NINA should send events automatically
                    info!("Connected to NINA websocket, listening for events...");

                    while let Some(message) = read.next().await {
                        heartbeat_counter += 1;
                        if heartbeat_counter % 30 == 0 {
                            info!("NINA websocket listener active for {} - received {} messages", base_url, heartbeat_counter);
                        }

                        match message {
                            Ok(Message::Text(text)) => {
                                println!("WEBSOCKET MESSAGE: {}", text);
                                // Parse the websocket message
                                if let Ok(nina_response) = serde_json::from_str::<NinaResponse<ImagePreparedEvent>>(&text) {
                                    debug!("Successfully parsed message as NinaResponse<ImagePreparedEvent> wrapper");
                                    if nina_response.success && nina_response.r#type == "Socket" {
                                        let event = &nina_response.response;
                                        if event.event == "IMAGE-PREPARED" {
                                            info!("Received IMAGE-PREPARED event from {}: {:?}", base_url, event);
                                            on_image_prepared(event.clone());
                                        } else {
                                            debug!("Ignoring non-IMAGE-PREPARED event: {}", event.event);
                                        }
                                    } else {
                                        debug!("Ignoring unsuccessful or non-socket message: success={}, type={}", nina_response.success, nina_response.r#type);
                                    }
                                } else {
                                    debug!("Failed to parse as NinaResponse<ImagePreparedEvent>, trying direct ImagePreparedEvent parsing");
                                    // Try parsing as direct ImagePreparedEvent (in case the wrapper isn't used for websockets)
                                    if let Ok(event) = serde_json::from_str::<ImagePreparedEvent>(&text) {
                                        if event.event == "IMAGE-PREPARED" {
                                            info!("Received IMAGE-PREPARED event from {} (direct parsing): {:?}", base_url, event);
                                            on_image_prepared(event);
                                        } else {
                                            debug!("Ignoring non-IMAGE-PREPARED event (direct): {}", event.event);
                                        }
                                    } else {
                                        debug!("Failed to parse websocket message as either NinaResponse<ImagePreparedEvent> or ImagePreparedEvent");
                                    }
                                }
                            }
                            Ok(Message::Close(close_frame)) => {
                                info!("NINA websocket connection closed for {}: {:?}", ws_url, close_frame);
                                break;
                            }
                            Ok(Message::Ping(payload)) => {
                                debug!("Received ping from {}, payload length: {}", ws_url, payload.len());
                            }
                            Ok(Message::Pong(payload)) => {
                                debug!("Received pong from {}, payload length: {}", ws_url, payload.len());
                            }
                            Ok(Message::Binary(data)) => {
                                debug!("Received binary message from {}, length: {}", ws_url, data.len());
                            }
                            Ok(Message::Frame(frame)) => {
                                debug!("Received frame message from {}: {:?}", ws_url, frame);
                            }
                            Err(e) => {
                                error!("Error receiving websocket message from {}: {}", ws_url, e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to connect to NINA websocket at {}: {} (detailed error: {:?})", ws_url, e, e);
                    // Log additional connection details
                    if e.to_string().contains("Connection refused") {
                        error!("Connection refused - check if NINA server is running and port {} is accessible", base_url);
                    } else if e.to_string().contains("Connection timed out") {
                        error!("Connection timed out - check network connectivity to {}", base_url);
                    } else if e.to_string().contains("DNS") {
                        error!("DNS resolution failed - check hostname/IP address: {}", base_url);
                    }
                    continue;
                }
            }

            // Wait before reconnecting
            info!("Reconnecting to NINA websocket in 5 seconds...");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
        })
    });

    (handle, stop_tx)
}
