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
    pub scale: serde_json::Value,
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
    let url = format!("{}/v2/api/equipment/guider/graph", base_url.trim_end_matches('/'));
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

/// Generate a guiding graph PNG for the given data
pub fn generate_guiding_graph_png(graph_data: &GuideStepsHistory, slot_index: usize) -> Result<Vec<u8>, anyhow::Error> {
    use plotters::prelude::*;

    info!("Generating guiding graph for slot {} with {} guide steps", slot_index + 1, graph_data.guide_steps.len());

    // Get only the last HistorySize points, similar to Vue.js: steps.slice(-size)
    let history_size = graph_data.history_size as usize;
    let steps_to_show: Vec<&GuideStep> = if graph_data.guide_steps.len() > history_size {
        graph_data.guide_steps.iter().rev().take(history_size).rev().collect()
    } else {
        graph_data.guide_steps.iter().collect()
    };

    info!("Using {} steps for slot {}", steps_to_show.len(), slot_index + 1);

    // Prepare data arrays like Vue.js Chart.js datasets
    let mut ra_distances = Vec::new();
    let mut dec_distances = Vec::new();
    let mut ra_durations = Vec::new();
    let mut dec_durations = Vec::new();
    let mut dither_points = Vec::new();

    let mut max_duration = 0.0f64;

    for step in &steps_to_show {
        let ra = step.ra_duration as f64;
        let dec = step.dec_duration as f64;

        ra_distances.push(step.ra_distance_raw_display as f64);
        dec_distances.push(step.dec_distance_raw_display as f64);
        ra_durations.push(ra);
        dec_durations.push(dec);

        // Track max duration for scaling like Vue.js
        max_duration = max_duration.max(ra.abs()).max(dec.abs());

        // Dither points (when Dither is not "NaN") - like Vue.js
        if step.dither != "NaN" {
            dither_points.push((step.id as f64, 0.0)); // At zero line
        }
    }

    // Fixed Y-axis scaling from -4 to 4 like Vue.js
    let min_y = -4.0;
    let max_y = 4.0;

    // Dynamic scaling for duration bars - fallback to 100ms like Vue.js
    let max_abs = max_duration.max(100.0);

    // Create bitmap buffer for plotters (RGB format: 3 bytes per pixel)
    let width = 800;
    let height = 180;
    let mut bitmap_buffer = vec![0u8; (width * height * 3) as usize];

    {
        let backend = plotters::backend::BitMapBackend::with_buffer(&mut bitmap_buffer, (width, height));
        let root = backend.into_drawing_area();

        // Fill with gray background
        root.fill(&RGBColor(39, 43, 46))?;

        // Create chart builder without margins (like original)
        let mut chart = ChartBuilder::on(&root)
            .x_label_area_size(30)
            .y_label_area_size(60)
            .build_cartesian_2d(0.0..history_size as f64, min_y..max_y)?;

        // Configure mesh with grid lines and labels
        chart.configure_mesh()
            .x_labels(0) // No x-axis labels since it's time-based
            .y_labels(5)
            .y_label_formatter(&|y| format!("{:.0}", y))
            .draw()?;

        // Prepare RA distance data points for LineSeries
        let ra_points: Vec<(f64, f64)> = ra_distances.iter().enumerate()
            .map(|(i, &y)| (i as f64, y.clamp(min_y, max_y)))
            .collect();

        // Draw RA distance line using LineSeries
        chart.draw_series(LineSeries::new(
            ra_points,
            RGBColor(70, 130, 180).stroke_width(2), // Exact Vue.js blue
        ))?;

        // Prepare Dec distance data points for LineSeries
        let dec_points: Vec<(f64, f64)> = dec_distances.iter().enumerate()
            .map(|(i, &y)| (i as f64, y.clamp(min_y, max_y)))
            .collect();

        // Draw Dec distance line using LineSeries
        chart.draw_series(LineSeries::new(
            dec_points,
            RGBColor(220, 20, 60).stroke_width(2), // Exact Vue.js red
        ))?;

        // Draw RA duration bars manually (ChartBuilder doesn't have built-in bars)
        for (i, &duration) in ra_durations.iter().enumerate() {
            if duration != 0.0 {
                let x = i as f64;
                let bar_color = RGBColor(70, 130, 180).mix(0.4); // Steel blue with 0.4 alpha

                // Bars always start from 0 and extend in the direction of the duration
                let y_start = 0.0;
                let y_end = (duration / max_abs) * (max_y - min_y) * 0.6;

                // Convert chart coordinates to screen coordinates for manual drawing
                let screen_x = chart.backend_coord(&(x, 0.0)).0;
                let screen_y_start = chart.backend_coord(&(x, y_start)).1;
                let screen_y_end = chart.backend_coord(&(x, y_end)).1;

                // Draw thicker bar (3 pixels wide) - handle both positive and negative directions
                let (y_min, y_max) = if screen_y_start < screen_y_end {
                    (screen_y_start, screen_y_end)
                } else {
                    (screen_y_end, screen_y_start)
                };

                for dx in 0..3 {
                    for y in y_min..=y_max {
                        root.draw(&plotters::element::Pixel::new((screen_x as i32 + dx, y as i32), bar_color))?;
                    }
                }
            }
        }

        // Draw Dec duration bars manually
        for (i, &duration) in dec_durations.iter().enumerate() {
            if duration != 0.0 {
                let x = i as f64;
                let bar_color = RGBColor(220, 20, 60).mix(0.4); // Crimson with 0.4 alpha

                // Bars always start from 0 and extend in the direction of the duration
                let y_start = 0.0;
                let y_end = (duration / max_abs) * (max_y - min_y) * 0.6;

                // Convert chart coordinates to screen coordinates for manual drawing
                let screen_x = chart.backend_coord(&(x + 0.1, 0.0)).0; // Slight offset from RA bars
                let screen_y_start = chart.backend_coord(&(x, y_start)).1;
                let screen_y_end = chart.backend_coord(&(x, y_end)).1;

                // Draw thicker bar (3 pixels wide) - handle both positive and negative directions
                let (y_min, y_max) = if screen_y_start < screen_y_end {
                    (screen_y_start, screen_y_end)
                } else {
                    (screen_y_end, screen_y_start)
                };

                for dx in 0..3 {
                    for y in y_min..=y_max {
                        root.draw(&plotters::element::Pixel::new((screen_x as i32 + dx, y as i32), bar_color))?;
                    }
                }
            }
        }

        // Draw dither points using PointSeries with triangle markers
        if !dither_points.is_empty() {
            chart.draw_series(PointSeries::of_element(
                dither_points,
                6, // pointRadius: 6 in Vue.js
                RGBColor(255, 165, 0), // Orange - exact Vue.js color
                &|c, s, st| {
                    return plotters::element::TriangleMarker::new(c, s, st.filled());
                },
            ))?;
        }

        // Ensure the drawing is completed
        root.present()?;
    }

    // Convert bitmap buffer to image::RgbImage and encode to PNG
    let img = image::RgbImage::from_raw(width, height, bitmap_buffer)
        .ok_or_else(|| anyhow::anyhow!("Failed to create image from bitmap buffer"))?;

    // Crop the image (60px from left, 30px from bottom -> 740x150)
    let cropped = image::DynamicImage::ImageRgb8(img).crop(60, 0, 740, 150);

    // Encode cropped image to PNG bytes
    let mut png_buffer = Vec::new();
    cropped.write_to(&mut std::io::Cursor::new(&mut png_buffer), image::ImageFormat::Png)?;

    Ok(png_buffer)
}
