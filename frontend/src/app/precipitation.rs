use anyhow::Result;
use chrono::{Duration as ChronoDuration, Timelike, Utc};
use log::{error, info};
use once_cell::sync::Lazy;
use slint::ComponentHandle;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::app::utils::decode_png_to_slint_image;
use crate::app::coordinates;
use crate::MainWindow;

use slint::{Model, VecModel};
use crate::MenuItem;

use geomet::{BoundingBox, GeoMetAPI};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PrecipitationModel {
    Radar = 0,
    Hrdps = 1,
    Rdps = 2,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PrecipitationCategory {
    PrecipitationType = 0,
    PrecipitationProbability = 1,
    PrecipitationAmount = 2,
    Temperature = 3,
    Snow = 4,
    Visibility = 5,
    Intensity = 6,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PrecipitationLayer {
    // Original layers
    PrecipType = 0,
    PrecipProb = 1,
    PrecipAmount = 2,
    FreezingMixed = 3,
    SnowDepth = 4,
    TempPressure = 5,
    RadarRain = 6,
    RadarSnow = 7,
    // New layers
    DominantPrecipType = 8,
    InstantPrecipType = 9,
    PrecipCharacter = 10,
    RainProb = 11,
    SnowProb = 12,
    DrizzleProb = 13,
    FreezingDrizzleProb = 14,
    FreezingRainProb = 15,
    IcePelletsProb = 16,
    LiquidPrecipProb = 17,
    FreezingPrecipProb = 18,
    ThunderstormProb = 19,
    BlowingSnowProb = 20,
    LiquidPrecipCondAmt = 21,
    FreezingPrecipCondAmt = 22,
    IcePelletsCondAmt = 23,
    AirTemp = 24,
    DewPointTemp = 25,
    BlowingSnowPresence = 26,
    IceFogVisibility = 27,
    TotalPrecipIntensityIndex = 28,
}

// Cache keys (model, layer, hour_index)
type ImgKey = (PrecipitationModel, PrecipitationLayer, u32);

static PRECIP_IMAGES: Lazy<Mutex<HashMap<ImgKey, Vec<u8>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static PRECIP_LEGENDS: Lazy<Mutex<HashMap<(PrecipitationModel, PrecipitationLayer), Vec<u8>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static PRECIP_POINT_DATA: Lazy<Mutex<PrecipitationPointData>> =
    Lazy::new(|| Mutex::new(PrecipitationPointData::default()));

static ACTIVE_MODE: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(1)); // 0 = Radar, 1 = Predictions
static ACTIVE_MODEL: Lazy<Mutex<PrecipitationModel>> =
    Lazy::new(|| Mutex::new(PrecipitationModel::Hrdps));
static ACTIVE_LAYER: Lazy<Mutex<PrecipitationLayer>> =
    Lazy::new(|| Mutex::new(PrecipitationLayer::PrecipType));
static ACTIVE_HOUR: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(0));
static ACTIVE_CATEGORY: Lazy<Mutex<PrecipitationCategory>> =
    Lazy::new(|| Mutex::new(PrecipitationCategory::PrecipitationType));
static ACTIVE_SUBLAYER: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(0));

static LAST_RADAR_FETCH: Lazy<Mutex<Option<chrono::DateTime<Utc>>>> =
    Lazy::new(|| Mutex::new(None));
static LAST_MODEL_FETCH: Lazy<Mutex<Option<chrono::DateTime<Utc>>>> =
    Lazy::new(|| Mutex::new(None));

#[derive(Clone, Default)]
pub struct PrecipitationPointData {
    pub prob_rain: i32,
    pub prob_snow: i32,
    pub prob_freezing_rain: i32,
    pub prob_ice_pellets: i32,
    pub prob_any_precip: i32,

    pub qpf_6h: f32,
    pub qpf_12h: f32,
    pub qpf_24h: f32,

    pub snow_depth: f32,
    pub air_temp: f32,
    pub dewpoint: f32,
    pub pressure: f32,

    pub system_summary: String,
}

// Public API called from main.rs
pub fn setup_precipitation_callbacks(main_window: &MainWindow) {
    let w_model = main_window.as_weak();
    main_window.on_precipitation_model_changed(move |index| {
        let w = w_model.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(win) = w.upgrade() {
                handle_model_change(&win, index);
            }
        })
        .ok();
    });

    let w_layer = main_window.as_weak();
    main_window.on_precipitation_layer_changed(move |index| {
        let w = w_layer.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(win) = w.upgrade() {
                handle_layer_change(&win, index);
            }
        })
        .ok();
    });

    let w_time = main_window.as_weak();
    main_window.on_precipitation_time_changed(move |index| {
        let w = w_time.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(win) = w.upgrade() {
                handle_time_change(&win, index);
            }
        })
        .ok();
    });

    let w_category = main_window.as_weak();
    main_window.on_precipitation_category_changed(move |index| {
        let w = w_category.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(win) = w.upgrade() {
                handle_category_change(&win, index);
            }
        })
        .ok();
    });

    let w_sublayer = main_window.as_weak();
    main_window.on_precipitation_sublayer_changed(move |index| {
        let w = w_sublayer.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(win) = w.upgrade() {
                handle_sublayer_change(&win, index);
            }
        })
        .ok();
    });

    let w_predictions_model = main_window.as_weak();
    main_window.on_precipitation_predictions_model_changed(move |index| {
        let w = w_predictions_model.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(win) = w.upgrade() {
                handle_predictions_model_change(&win, index);
            }
        })
        .ok();
    });

    let w_time_slider = main_window.as_weak();
    main_window.on_precipitation_time_slider_changed(move |value| {
        let w = w_time_slider.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(win) = w.upgrade() {
                handle_time_change(&win, value as i32);
            }
        })
        .ok();
    });

    let w_refresh = main_window.as_weak();
    main_window.on_precipitation_refresh_now(move || {
        let w = w_refresh.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(win) = w.upgrade() {
                match coordinates::load_coordinates(&win) {
                    Ok((lat, lon)) => {
                        info!("Precipitation refresh: coordinates loaded successfully: lat={}, lon={}", lat, lon);
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        rt.block_on(async move {
                            if let Err(e) = refresh_precipitation_data(lat, lon, &win).await {
                                error!("Precipitation manual refresh failed: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        error!("Precipitation refresh failed: could not load coordinates: {e}");
                    }
                }
            } else {
                error!("Precipitation refresh failed: window upgrade failed");
            }
        }).unwrap();
    });

    let w_load_all = main_window.as_weak();
    main_window.on_precipitation_load_all_hours(move || {
        let w = w_load_all.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(win) = w.upgrade() {
                handle_load_all_hours(&win);
            }
        })
        .ok();
    });
}

pub async fn fetch_initial_precipitation(lat: f64, lon: f64) -> Result<()> {
    // Initialize active selection to HRDPS PrecipType T+0h as primary default.
    {
        let mut model = ACTIVE_MODEL.lock().unwrap();
        let mut layer = ACTIVE_LAYER.lock().unwrap();
        let mut hour = ACTIVE_HOUR.lock().unwrap();
        *model = PrecipitationModel::Hrdps;
        *layer = PrecipitationLayer::PrecipType;
        *hour = 0;
    }

    // Mark model data timestamp for reference (lazy refresh beyond this remains unchanged).
    {
        let mut last = LAST_MODEL_FETCH.lock().unwrap();
        *last = Some(Utc::now());
    }

    info!("precipitation: initial setup completed without prefetching");
    Ok(())
}

pub async fn refresh_precipitation_data(
    lat: f64,
    lon: f64,
    main_window: &MainWindow,
) -> Result<()> {
    // Clear all caches for manual refresh
    {
        let mut imgs = PRECIP_IMAGES.lock().unwrap();
        imgs.clear();
        let mut legends = PRECIP_LEGENDS.lock().unwrap();
        legends.clear();
        let mut last_radar = LAST_RADAR_FETCH.lock().unwrap();
        *last_radar = None;
        let mut last_model = LAST_MODEL_FETCH.lock().unwrap();
        *last_model = None;
    }
    error!("precipitation: cleared all caches for manual refresh");

    let api = GeoMetAPI::new()?;
    let bbox = precipitation_bbox(lat, lon);
    let now = Utc::now();

    // Radar every 30 minutes
    {
        let mut last = LAST_RADAR_FETCH.lock().unwrap();
        if last
            .map(|t| now.signed_duration_since(t) >= ChronoDuration::minutes(30))
            .unwrap_or(true)
        {
            match fetch_radar_now(&api, &bbox).await {
                Ok((rain_bytes, snow_bytes, radar_time)) => {
                    let mut imgs = PRECIP_IMAGES.lock().unwrap();
                    imgs.insert(
                        (PrecipitationModel::Radar, PrecipitationLayer::RadarRain, 0),
                        rain_bytes,
                    );
                    imgs.insert(
                        (PrecipitationModel::Radar, PrecipitationLayer::RadarSnow, 0),
                        snow_bytes,
                    );
                    *last = Some(radar_time);
                    error!(
                        "precipitation: radar refresh ok at {}",
                        radar_time.to_rfc3339()
                    );
                }
                Err(e) => {
                    error!("precipitation: radar refresh FAILED: {}", e);
                }
            }
        }
    }

    // HRDPS/RDPS periodic marker (lazy loading: only update timestamp, no bulk fetch)
    {
        let mut last = LAST_MODEL_FETCH.lock().unwrap();
        if last
            .map(|t| now.signed_duration_since(t) >= ChronoDuration::minutes(60))
            .unwrap_or(true)
        {
            info!("precipitation: marking model data window as stale (lazy load on demand)");
            *last = Some(now);
        }
    }

    // Push updated current selection into UI
    let w = main_window.as_weak();
    slint::invoke_from_event_loop(move || {
        if let Some(win) = w.upgrade() {
            update_precipitation_display(&win);
        }
    })
    .ok();

    error!("precipitation: manual refresh completed successfully");
    Ok(())
}

// Internal helpers

fn precipitation_bbox(lat: f64, lon: f64) -> BoundingBox {
    // 5-7 degrees around station for context
    BoundingBox::new(lon - 12.7, lon + 12.7, lat - 5.0, lat + 5.0)
}

async fn fetch_radar_now(
    api: &GeoMetAPI,
    bbox: &BoundingBox,
) -> Result<(Vec<u8>, Vec<u8>, chrono::DateTime<Utc>)> {
    // Use current UTC, snapped down to previous 30-min slot
    let now = Utc::now();
    let minute = now.minute();
    let snapped_minute = if minute < 30 { 0 } else { 30 };
    let snapped = now
        .with_minute(snapped_minute)
        .and_then(|t| t.with_second(0))
        .and_then(|t| t.with_nanosecond(0))
        .unwrap_or_else(|| now - ChronoDuration::minutes((minute % 30) as i64));

    let time_str = snapped.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let rain_layer = "RADAR_1KM_RRAI";
    let snow_layer = "RADAR_1KM_RSNO";

    info!(
        "precipitation: fetching radar rain/snow at {} for bbox={:?}",
        time_str, bbox
    );

    // Fetch rain radar (required)
    let rain_bytes = match api
        .get_wms_image_with_style(rain_layer, &time_str, bbox.clone(), 1280, 720, Some("Radar-Rain"))
        .await
    {
        Ok(b) => b,
        Err(e) => {
            error!(
                "precipitation: failed to fetch {} at {}: {}",
                rain_layer, time_str, e
            );
            return Err(e.into());
        }
    };

    // Fetch snow radar (optional)
    let snow_bytes = match api
        .get_wms_image_with_style(snow_layer, &time_str, bbox.clone(), 1280, 720, Some("Radar-Snow"))
        .await
    {
        Ok(b) => b,
        Err(e) => {
            error!(
                "precipitation: failed to fetch {} at {}: {} (continuing with rain-only)",
                snow_layer, time_str, e
            );
            Vec::new()
        }
    };

    Ok((rain_bytes, snow_bytes, snapped))
}

async fn preload_model_core(
    api: &GeoMetAPI,
    model: PrecipitationModel,
    bbox: &BoundingBox,
    layer: PrecipitationLayer,
    hour: u32,
) -> Result<()> {
    let (layer_name, style) = match (model, layer) {
        (PrecipitationModel::Radar, PrecipitationLayer::RadarRain) => ("RADAR_1KM_RRAI", Some("Radar-Rain")),
        (PrecipitationModel::Radar, PrecipitationLayer::RadarSnow) => ("RADAR_1KM_RSNO", Some("Radar-Snow")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::PrecipType) => ("HRDPS-WEonG_2.5km_DominantPrecipType", None),
        (PrecipitationModel::Hrdps, PrecipitationLayer::PrecipProb) => ("HRDPS-WEonG_2.5km_Precip-Prob", None),
        (PrecipitationModel::Hrdps, PrecipitationLayer::PrecipAmount) => ("HRDPS-WEonG_2.5km_PrecipCondAmt", None),
        (PrecipitationModel::Hrdps, PrecipitationLayer::TempPressure) => ("HRDPS-WEonG_2.5km_AirTemp", Some("TEMPERATURE-LINEAR")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::FreezingMixed) => ("HRDPS-WEonG_2.5km_FreezingPrecip-Prob", None),
        (PrecipitationModel::Hrdps, PrecipitationLayer::SnowDepth) => ("HRDPS.CONTINENTAL_SD", Some("SNOWDEPTH-LINEAR")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::DominantPrecipType) => ("HRDPS-WEonG_2.5km_DominantPrecipType", Some("DominantPrecipType_Dis")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::InstantPrecipType) => ("HRDPS-WEonG_2.5km_InstantPrecipType", Some("INSTPRECIPITATIONTYPE")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::PrecipCharacter) => ("HRDPS-WEonG_2.5km_PrecipCharacter", Some("PrecipCharacter_Dis")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::RainProb) => ("HRDPS-WEonG_2.5km_Rain-Prob", Some("Rain-Prob")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::SnowProb) => ("HRDPS-WEonG_2.5km_Snow-Prob", Some("Snow-Prob")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::DrizzleProb) => ("HRDPS-WEonG_2.5km_Drizzle-Prob", Some("Drizzle-Prob")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::FreezingDrizzleProb) => ("HRDPS-WEonG_2.5km_FreezingDrizzle-Prob", Some("FreezingDrizzle-Prob")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::FreezingRainProb) => ("HRDPS-WEonG_2.5km_FreezingRain-Prob", Some("FreezingRain-Prob")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::IcePelletsProb) => ("HRDPS-WEonG_2.5km_IcePellets-Prob", Some("IcePellets-Prob")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::LiquidPrecipProb) => ("HRDPS-WEonG_2.5km_LiquidPrecip-Prob", Some("LiquidPrecip-Prob")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::FreezingPrecipProb) => ("HRDPS-WEonG_2.5km_FreezingPrecip-Prob", Some("FreezingPrecip-Prob")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::ThunderstormProb) => ("HRDPS-WEonG_2.5km_Thunderstorm-Prob", Some("Thunderstorm-Prob")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::BlowingSnowProb) => ("HRDPS-WEonG_2.5km_BlowingSnow-Prob", Some("BlowingSnow-Prob")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::LiquidPrecipCondAmt) => ("HRDPS-WEonG_2.5km_LiquidPrecipCondAmt", Some("LiquidPrecipCondAmt")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::FreezingPrecipCondAmt) => ("HRDPS-WEonG_2.5km_FreezingPrecipCondAmt", Some("FreezingPrecipCondAmt")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::IcePelletsCondAmt) => ("HRDPS-WEonG_2.5km_IcePelletsCondAmt", Some("IcePelletsCondAmt")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::AirTemp) => ("HRDPS-WEonG_2.5km_AirTemp", Some("TEMPERATURE-LINEAR")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::DewPointTemp) => ("HRDPS-WEonG_2.5km_DewPointTemp", Some("DEWPOINT")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::BlowingSnowPresence) => ("HRDPS-WEonG_2.5km_BlowingSnowPresence", Some("BlowingSnowPresence_Dis")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::IceFogVisibility) => ("HRDPS-WEonG_2.5km_IceFogVisibility", Some("IceFogVisibility_Dis")),
        (PrecipitationModel::Hrdps, PrecipitationLayer::TotalPrecipIntensityIndex) => ("HRDPS-WEonG_2.5km_TotalPrecipIntensityIndex", Some("TotalPrecipIntensityIndex_Dis")),
        (PrecipitationModel::Rdps, PrecipitationLayer::PrecipType) => ("RDPS-WEonG_10km_DominantPrecipType", None),
        (PrecipitationModel::Rdps, PrecipitationLayer::PrecipProb) => ("RDPS-WEonG_10km_Precip-Prob", None),
        (PrecipitationModel::Rdps, PrecipitationLayer::PrecipAmount) => ("RDPS-WEonG_10km_PrecipCondAmt", None),
        (PrecipitationModel::Rdps, PrecipitationLayer::TempPressure) => ("RDPS-WEonG_10km_AirTemp", Some("TEMPERATURE-LINEAR")),
        (PrecipitationModel::Rdps, PrecipitationLayer::FreezingMixed) => ("RDPS-WEonG_10km_FreezingPrecip-Prob", None),
        (PrecipitationModel::Rdps, PrecipitationLayer::SnowDepth) => ("RDPS.ETA_SD", Some("SNOWDEPTH-LINEAR")),
        (PrecipitationModel::Rdps, PrecipitationLayer::DominantPrecipType) => ("RDPS-WEonG_10km_DominantPrecipType", Some("DominantPrecipType_Dis")),
        (PrecipitationModel::Rdps, PrecipitationLayer::InstantPrecipType) => ("RDPS-WEonG_10km_InstantPrecipType", Some("INSTPRECIPITATIONTYPE")),
        (PrecipitationModel::Rdps, PrecipitationLayer::PrecipCharacter) => ("RDPS-WEonG_10km_PrecipCharacter", Some("PrecipCharacter_Dis")),
        (PrecipitationModel::Rdps, PrecipitationLayer::RainProb) => ("RDPS-WEonG_10km_Rain-Prob", Some("Rain-Prob")),
        (PrecipitationModel::Rdps, PrecipitationLayer::SnowProb) => ("RDPS-WEonG_10km_Snow-Prob", Some("Snow-Prob")),
        (PrecipitationModel::Rdps, PrecipitationLayer::DrizzleProb) => ("RDPS-WEonG_10km_Drizzle-Prob", Some("Drizzle-Prob")),
        (PrecipitationModel::Rdps, PrecipitationLayer::FreezingDrizzleProb) => ("RDPS-WEonG_10km_FreezingDrizzle-Prob", Some("FreezingDrizzle-Prob")),
        (PrecipitationModel::Rdps, PrecipitationLayer::FreezingRainProb) => ("RDPS-WEonG_10km_FreezingRain-Prob", Some("FreezingRain-Prob")),
        (PrecipitationModel::Rdps, PrecipitationLayer::IcePelletsProb) => ("RDPS-WEonG_10km_IcePellets-Prob", Some("IcePellets-Prob")),
        (PrecipitationModel::Rdps, PrecipitationLayer::LiquidPrecipProb) => ("RDPS-WEonG_10km_LiquidPrecip-Prob", Some("LiquidPrecip-Prob")),
        (PrecipitationModel::Rdps, PrecipitationLayer::FreezingPrecipProb) => ("RDPS-WEonG_10km_FreezingPrecip-Prob", Some("FreezingPrecip-Prob")),
        (PrecipitationModel::Rdps, PrecipitationLayer::ThunderstormProb) => ("RDPS-WEonG_10km_Thunderstorm-Prob", Some("Thunderstorm-Prob")),
        (PrecipitationModel::Rdps, PrecipitationLayer::BlowingSnowProb) => ("RDPS-WEonG_10km_BlowingSnow-Prob", Some("BlowingSnow-Prob")),
        (PrecipitationModel::Rdps, PrecipitationLayer::LiquidPrecipCondAmt) => ("RDPS-WEonG_10km_LiquidPrecipCondAmt", Some("LiquidPrecipCondAmt")),
        (PrecipitationModel::Rdps, PrecipitationLayer::FreezingPrecipCondAmt) => ("RDPS-WEonG_10km_FreezingPrecipCondAmt", Some("FreezingPrecipCondAmt")),
        (PrecipitationModel::Rdps, PrecipitationLayer::IcePelletsCondAmt) => ("RDPS-WEonG_10km_IcePelletsCondAmt", Some("IcePelletsCondAmt")),
        (PrecipitationModel::Rdps, PrecipitationLayer::AirTemp) => ("RDPS-WEonG_10km_AirTemp", Some("TEMPERATURE-LINEAR")),
        (PrecipitationModel::Rdps, PrecipitationLayer::DewPointTemp) => ("RDPS-WEonG_10km_DewPointTemp", Some("DEWPOINT")),
        (PrecipitationModel::Rdps, PrecipitationLayer::BlowingSnowPresence) => ("RDPS-WEonG_10km_BlowingSnowPresence", Some("BlowingSnowPresence_Dis")),
        (PrecipitationModel::Rdps, PrecipitationLayer::IceFogVisibility) => ("RDPS-WEonG_10km_IceFogVisibility", Some("IceFogVisibility_Dis")),
        (PrecipitationModel::Rdps, PrecipitationLayer::TotalPrecipIntensityIndex) => ("RDPS-WEonG_10km_TotalPrecipIntensityIndex", Some("TotalPrecipIntensityIndex_Dis")),
        // For unsupported (model, layer) combinations we do nothing.
        _ => return Ok(()),
    };

    let mut imgs = PRECIP_IMAGES.lock().unwrap();
    let mut legends = PRECIP_LEGENDS.lock().unwrap();

    // Legend (once per (model, layer))
    let legend_key = (model, layer);
    if !legends.contains_key(&legend_key) {
        if let Ok(lg) = api
            .get_legend_graphic(layer_name, style, "image/png", None)
            .await
        {
            info!(
                "precipitation: loaded legend for {:?} {:?} ({} bytes)",
                model,
                layer,
                lg.len()
            );
            legends.insert(legend_key, lg);
        } else {
            error!(
                "precipitation: FAILED to load legend for {:?} {:?}",
                model, layer
            );
        }
    }

    // If we already have this (model,layer,hour), do nothing.
    let key = (model, layer, hour);
    if imgs.contains_key(&key) {
        return Ok(());
    }

    // Build TIME parameter: forecast base run + offset
    let now = Utc::now();
    let base_hour = now
        .with_minute(0)
        .and_then(|t| t.with_second(0))
        .and_then(|t| t.with_nanosecond(0))
        .unwrap_or(now);

    let t = base_hour + ChronoDuration::hours(hour as i64);
    let time_str = t.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    info!(
        "precipitation: fetching {:?} {:?} T+{}h ({}) for bbox={:?}",
        model, layer, hour, time_str, bbox
    );

    let image_result = if let Some(style_name) = style {
        api.get_wms_image_with_style(layer_name, &time_str, bbox.clone(), 1280, 720, Some(style_name)).await
    } else {
        api.get_wms_image(layer_name, &time_str, bbox.clone(), 1280, 720).await
    };

    match image_result {
        Ok(bytes) => {
            info!(
                "precipitation: fetched {:?} {:?} T+{}h ({} bytes)",
                model,
                layer,
                hour,
                bytes.len()
            );
            imgs.insert(key, bytes);
        }
        Err(e) => {
            error!(
                "precipitation: FAILED {:?} {:?} T+{}h ({}): {}",
                model, layer, hour, time_str, e
            );
        }
    }

    Ok(())
}

// Display selection logic

pub fn update_precipitation_display(win: &MainWindow) {
    let model = *ACTIVE_MODEL.lock().unwrap();
    let layer = *ACTIVE_LAYER.lock().unwrap();
    let hour = *ACTIVE_HOUR.lock().unwrap();

    let imgs = PRECIP_IMAGES.lock().unwrap();
    let legends = PRECIP_LEGENDS.lock().unwrap();
    let point = PRECIP_POINT_DATA.lock().unwrap().clone();

    info!(
        "precipitation: update display: model={:?} layer={:?} hour={} available_images={}",
        model,
        layer,
        hour,
        imgs.len()
    );

    // Check if exact selection is available
    let mut chosen_bytes: Option<Vec<u8>> = imgs.get(&(model, layer, hour)).cloned();

    if chosen_bytes.is_none() {
        // Try to fetch the missing image lazily
        lazy_fetch_missing_image(win, model, layer, hour);
        // For now, fall back to available images while fetch happens
        chosen_bytes = find_fallback_image(&imgs, model, layer, hour);
    } else {
        chosen_bytes = Some(chosen_bytes.unwrap());
    }

    // Legend
    let legend_bytes = legends.get(&(model, layer)).cloned();

    if chosen_bytes.is_none() {
        error!(
            "precipitation: no image available for model={:?} layer={:?} hour={}",
            model, layer, hour
        );
    }

    if legend_bytes.is_none() {
        info!(
            "precipitation: no legend available for model={:?} layer={:?}",
            model, layer
        );
    }

    // Base map from shared map.rs, to provide consistent geographic context
    let base_map_img = win.get_map_image();

    // Helper function to validate and decode image bytes
    let decode_image_safe = |bytes: &[u8], image_type: &str| -> Option<slint::Image> {
        // Basic validation: check if it looks like a PNG (starts with PNG magic bytes)
        if bytes.len() < 8 || bytes[0..8] != [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
            error!("precipitation: {} data doesn't appear to be a valid PNG ({} bytes, starts with {:02x?})",
                   image_type, bytes.len(), &bytes.get(0..8).unwrap_or(&[]));
            return None;
        }

        match decode_png_to_slint_image(bytes) {
            Ok(img) => {
                info!("precipitation: decoded {} image OK", image_type);
                Some(img)
            }
            Err(e) => {
                error!("precipitation: failed to decode {} image: {}", image_type, e);
                None
            }
        }
    };

    // Decode active precipitation image using existing util (PNG -> slint::Image)
    let (active_img, has_error) = match chosen_bytes.as_ref() {
        Some(bytes) => match decode_image_safe(bytes, "active") {
            Some(img) => (img, false),
            None => (slint::Image::default(), true),
        },
        None => (slint::Image::default(), false),
    };

    let legend_img = legend_bytes
        .as_ref()
        .and_then(|b| decode_image_safe(b, "legend"))
        .unwrap_or_default();

    // Use the map.rs-provided basemap as background, precipitation as overlay in Slint
    win.set_precipitation_base_map_image(base_map_img);
    win.set_precipitation_active_precip_image(active_img);
    win.set_precipitation_legend_image(legend_img);
    win.set_precipitation_active_image_error(has_error);

    // Labels
    let model_label = match model {
        PrecipitationModel::Radar => "Radar (Now)",
        PrecipitationModel::Hrdps => "HRDPS 0–48h",
        PrecipitationModel::Rdps => "RDPS 0–80h",
    };
    let layer_label = match layer {
        PrecipitationLayer::PrecipType => "Precip Type",
        PrecipitationLayer::PrecipProb => "Precip Prob",
        PrecipitationLayer::PrecipAmount => "Precip Amount",
        PrecipitationLayer::FreezingMixed => "Freezing/Mixed",
        PrecipitationLayer::SnowDepth => "Snow Depth",
        PrecipitationLayer::TempPressure => "Temp & Pressure",
        PrecipitationLayer::RadarRain => "Radar Rain",
        PrecipitationLayer::RadarSnow => "Radar Snow",
        PrecipitationLayer::DominantPrecipType => "Dominant Precip Type",
        PrecipitationLayer::InstantPrecipType => "Instant Precip Type",
        PrecipitationLayer::PrecipCharacter => "Precip Character",
        PrecipitationLayer::RainProb => "Rain Prob",
        PrecipitationLayer::SnowProb => "Snow Prob",
        PrecipitationLayer::DrizzleProb => "Drizzle Prob",
        PrecipitationLayer::FreezingDrizzleProb => "Freezing Drizzle Prob",
        PrecipitationLayer::FreezingRainProb => "Freezing Rain Prob",
        PrecipitationLayer::IcePelletsProb => "Ice Pellets Prob",
        PrecipitationLayer::LiquidPrecipProb => "Liquid Precip Prob",
        PrecipitationLayer::FreezingPrecipProb => "Freezing Precip Prob",
        PrecipitationLayer::ThunderstormProb => "Thunderstorm Prob",
        PrecipitationLayer::BlowingSnowProb => "Blowing Snow Prob",
        PrecipitationLayer::LiquidPrecipCondAmt => "Liquid Precip Amount",
        PrecipitationLayer::FreezingPrecipCondAmt => "Freezing Precip Amount",
        PrecipitationLayer::IcePelletsCondAmt => "Ice Pellets Amount",
        PrecipitationLayer::AirTemp => "Air Temperature",
        PrecipitationLayer::DewPointTemp => "Dew Point Temp",
        PrecipitationLayer::BlowingSnowPresence => "Blowing Snow Presence",
        PrecipitationLayer::IceFogVisibility => "Ice Fog Visibility",
        PrecipitationLayer::TotalPrecipIntensityIndex => "Precip Intensity Index",
    };
    let time_label = if model == PrecipitationModel::Radar {
        "NOW".to_string()
    } else {
        format!("+{}h", hour)
    };

    win.set_precipitation_active_model_label(model_label.into());
    win.set_precipitation_active_layer_label(layer_label.into());
    win.set_precipitation_active_time_label(time_label.into());
    win.set_precipitation_last_update_text("".into()); // can be filled from LAST_* timestamps

    // Set slider properties
    let (slider_min, slider_max) = match model {
        PrecipitationModel::Radar => (0.0, 0.0),
        PrecipitationModel::Hrdps => (0.0, 48.0),
        PrecipitationModel::Rdps => (0.0, 80.0),
    };
    win.set_precipitation_time_slider_value(hour as f32);
    win.set_precipitation_time_slider_min(slider_min);
    win.set_precipitation_time_slider_max(slider_max);

    // Update selected indices
    let mode = *ACTIVE_MODE.lock().unwrap();
    win.set_precipitation_selected_mode_index(mode as i32);
    win.set_precipitation_selected_model_index(if model == PrecipitationModel::Hrdps { 0 } else { 1 });
    win.set_precipitation_selected_layer_index(layer as i32);
    win.set_precipitation_selected_time_index(hour as i32);
    win.set_precipitation_selected_category_index(*ACTIVE_CATEGORY.lock().unwrap() as i32);
    win.set_precipitation_selected_sublayer_index(*ACTIVE_SUBLAYER.lock().unwrap() as i32);
    win.set_precipitation_selected_predictions_model_index(if model == PrecipitationModel::Hrdps { 0 } else { 1 });

    // Metrics
    win.set_precipitation_prob_rain(point.prob_rain);
    win.set_precipitation_prob_snow(point.prob_snow);
    win.set_precipitation_prob_freezing_rain(point.prob_freezing_rain);
    win.set_precipitation_prob_ice_pellets(point.prob_ice_pellets);
    win.set_precipitation_prob_any_precip(point.prob_any_precip);

    win.set_precipitation_qpf_6h(point.qpf_6h);
    win.set_precipitation_qpf_12h(point.qpf_12h);
    win.set_precipitation_qpf_24h(point.qpf_24h);

    win.set_precipitation_snow_depth(point.snow_depth);
    win.set_precipitation_air_temp(point.air_temp);
    win.set_precipitation_dewpoint(point.dewpoint);
    win.set_precipitation_pressure(point.pressure);
    win.set_precipitation_system_summary(point.system_summary.into());

    // Update dropdown items
    let category = *ACTIVE_CATEGORY.lock().unwrap();
    let layer_items = generate_layer_dropdown_items(category);
    let model_items = generate_model_dropdown_items();

    win.set_precipitation_layer_dropdown_items(slint::ModelRc::new(VecModel::from(layer_items)));
    win.set_precipitation_model_dropdown_items(slint::ModelRc::new(VecModel::from(model_items)));

    // Set load all hours button initial state
    win.set_precipitation_load_all_hours_button_text("Load all hours".into());
    win.set_precipitation_load_all_hours_button_enabled(model != PrecipitationModel::Radar);
}

// Lazy fetch missing image
fn lazy_fetch_missing_image(win: &MainWindow, model: PrecipitationModel, layer: PrecipitationLayer, hour: u32) {
    info!("precipitation: lazy fetching missing image {:?} {:?} T+{}h", model, layer, hour);

    let bbox = if let Ok((lat, lon)) = coordinates::load_coordinates(win) {
        precipitation_bbox(lat, lon)
    } else {
        precipitation_bbox(45.0, -75.0)
    };

    let win_weak = win.as_weak();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            if let Ok(api) = GeoMetAPI::new() {
                if let Err(e) = preload_model_core(&api, model, &bbox, layer, hour).await {
                    error!(
                        "precipitation: lazy fetch failed for {:?} {:?} T+{}h: {}",
                        model, layer, hour, e
                    );
                } else {
                    info!("precipitation: lazy fetch completed for {:?} {:?} T+{}h", model, layer, hour);
                }
            } else {
                error!("precipitation: failed to create GeoMetAPI for lazy fetch");
            }

            // Update display after fetch attempt
            slint::invoke_from_event_loop(move || {
                if let Some(win) = win_weak.upgrade() {
                    update_precipitation_display(&win);
                }
            }).ok();
        });
    });
}

// Find fallback image while lazy fetch happens
fn find_fallback_image(imgs: &HashMap<ImgKey, Vec<u8>>, model: PrecipitationModel, layer: PrecipitationLayer, hour: u32) -> Option<Vec<u8>> {
    // Try same model/layer at hour 0
    if let Some(b) = imgs.get(&(model, layer, 0)) {
        return Some(b.clone());
    }

    /*// For non-radar models, try radar rain
    if model != PrecipitationModel::Radar {
        if let Some(b) = imgs.get(&(PrecipitationModel::Radar, PrecipitationLayer::RadarRain, 0)) {
            return Some(b.clone());
        }
    }

    // Last resort: any HRDPS PrecipType
    for h in [0u32, 1, 2, 3, 6, 12, 24, 36, 48] {
        if let Some(b) = imgs.get(&(PrecipitationModel::Hrdps, PrecipitationLayer::PrecipType, h)) {
            return Some(b.clone());
        }
    }*/

    None
}

// Callback handlers

fn handle_model_change(win: &MainWindow, index: i32) {
    let mode = index as u32;
    *ACTIVE_MODE.lock().unwrap() = mode;

    let model = if mode == 0 {
        PrecipitationModel::Radar
    } else {
        let current = *ACTIVE_MODEL.lock().unwrap();
        if matches!(current, PrecipitationModel::Radar) {
            PrecipitationModel::Hrdps // Default to HRDPS when entering predictions mode
        } else {
            current
        }
    };

    *ACTIVE_MODEL.lock().unwrap() = model;

    // Set default hour based on model
    let default_hour = match model {
        PrecipitationModel::Radar => 0,
        PrecipitationModel::Hrdps => 0,
        PrecipitationModel::Rdps => 48, // Default RDPS to T+48h
    };

    *ACTIVE_HOUR.lock().unwrap() = default_hour;

    win.set_precipitation_selected_mode_index(index);
    win.set_precipitation_selected_time_index(default_hour as i32);

    if model == PrecipitationModel::Radar {
        *ACTIVE_LAYER.lock().unwrap() = PrecipitationLayer::RadarRain;
        win.set_precipitation_selected_layer_index(6);
    } else {
        *ACTIVE_LAYER.lock().unwrap() = PrecipitationLayer::PrecipType;
        win.set_precipitation_selected_layer_index(0);
    }

    info!("precipitation: mode changed to {}, model={:?}, hour reset to {}", mode, model, default_hour);

    if model == PrecipitationModel::Radar {
        // For Radar: fetch latest composites immediately and update UI when ready.
        let win_weak = win.as_weak();
        let bbox = if let Ok((lat, lon)) = coordinates::load_coordinates(win) {
            precipitation_bbox(lat, lon)
        } else {
            precipitation_bbox(45.0, -75.0)
        };

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                if let Ok(api) = GeoMetAPI::new() {
                    match fetch_radar_now(&api, &bbox).await {
                        Ok((rain, snow, when)) => {
                            {
                                let mut imgs = PRECIP_IMAGES.lock().unwrap();
                                // Map:
                                // - RADAR_1KM_RRAI -> RadarRain
                                // - RADAR_1KM_RSNO -> RadarSnow
                                imgs.insert(
                                    (PrecipitationModel::Radar, PrecipitationLayer::RadarRain, 0),
                                    rain,
                                );
                                if !snow.is_empty() {
                                    imgs.insert(
                                        (PrecipitationModel::Radar, PrecipitationLayer::RadarSnow, 0),
                                        snow,
                                    );
                                }
                            }
                            {
                                let mut last = LAST_RADAR_FETCH.lock().unwrap();
                                *last = Some(when);
                            }
                            info!(
                                "precipitation: radar images cached for slot {}",
                                when.to_rfc3339()
                            );
                        }
                        Err(e) => {
                            error!("precipitation: failed to fetch radar on model change: {}", e);
                        }
                    }
                } else {
                    error!("precipitation: failed to create GeoMetAPI for radar on model change");
                }

                slint::invoke_from_event_loop(move || {
                    if let Some(w) = win_weak.upgrade() {
                        update_precipitation_display(&w);
                    }
                })
                .ok();
            });
        });
    } else {
        // HRDPS/RDPS: display with existing cache; specific hours are lazy-fetched in handle_time_change.
        update_precipitation_display(win);
    }
}

fn handle_layer_change(win: &MainWindow, index: i32) {
    let layer = match index {
        0 => PrecipitationLayer::PrecipType,
        1 => PrecipitationLayer::PrecipProb,
        2 => PrecipitationLayer::PrecipAmount,
        3 => PrecipitationLayer::FreezingMixed,
        4 => PrecipitationLayer::SnowDepth,
        5 => PrecipitationLayer::TempPressure,
        6 => PrecipitationLayer::RadarRain,
        7 => PrecipitationLayer::RadarSnow,
        _ => PrecipitationLayer::PrecipType,
    };
    *ACTIVE_LAYER.lock().unwrap() = layer;
    // When layer changes, keep current hour but ensure it is valid for this model/layer
    win.set_precipitation_selected_layer_index(index);
    info!("precipitation: layer changed to {:?}", layer);
    update_precipitation_display(win);
}

fn handle_time_change(win: &MainWindow, index: i32) {
    if index < 0 {
        return;
    }

    let model = *ACTIVE_MODEL.lock().unwrap();
    let layer = *ACTIVE_LAYER.lock().unwrap();
    let hour = index as u32;

    // For radar: just switch hour/index (single slot) and update immediately
    if model == PrecipitationModel::Radar {
        *ACTIVE_HOUR.lock().unwrap() = 0;
        win.set_precipitation_selected_time_index(0);
        info!(
            "precipitation: radar time change requested (ignored index={}, fixed to slot 0)",
            index
        );
        update_precipitation_display(win);
        return;
    }

    // For model data: fetch on demand, and only update Slint once fetch completes.
    let bbox = if let Ok((lat, lon)) = coordinates::load_coordinates(win) {
        precipitation_bbox(lat, lon)
    } else {
        precipitation_bbox(45.0, -75.0)
    };

    let win_weak = win.as_weak();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            // Ensure we have an API client
            if let Ok(api) = GeoMetAPI::new() {
                // Only fetch if this (model,layer,hour) is missing
                let need_fetch = {
                    let imgs = PRECIP_IMAGES.lock().unwrap();
                    !imgs.contains_key(&(model, layer, hour))
                };

                if need_fetch {
                    if let Err(e) = preload_model_core(&api, model, &bbox, layer, hour).await {
                        error!(
                            "precipitation: failed to lazy-load {:?} {:?} T+{}h: {}",
                            model, layer, hour, e
                        );
                    }
                }
            } else {
                error!("precipitation: failed to create GeoMetAPI in time-change handler");
            }

            // Now that data is guaranteed attempted, update ACTIVE_HOUR + Slint UI in event loop
            slint::invoke_from_event_loop(move || {
                if let Some(win) = win_weak.upgrade() {
                    *ACTIVE_HOUR.lock().unwrap() = hour;
                    win.set_precipitation_selected_time_index(hour as i32);

                    info!(
                        "precipitation: time changed to T+{}h for model={:?}, layer={:?} (UI updated after fetch)",
                        hour, model, layer
                    );

                    update_precipitation_display(&win);
                }
            })
            .ok();
        });
    });
}

fn handle_category_change(win: &MainWindow, index: i32) {
    let category = match index {
        0 => PrecipitationCategory::PrecipitationType,
        1 => PrecipitationCategory::PrecipitationProbability,
        2 => PrecipitationCategory::PrecipitationAmount,
        3 => PrecipitationCategory::Temperature,
        4 => PrecipitationCategory::Snow,
        5 => PrecipitationCategory::Visibility,
        6 => PrecipitationCategory::Intensity,
        _ => PrecipitationCategory::PrecipitationType,
    };
    *ACTIVE_CATEGORY.lock().unwrap() = category;
    // Reset sublayer to 0
    *ACTIVE_SUBLAYER.lock().unwrap() = 0;
    // Update layer based on category and sublayer
    update_active_layer_from_category_and_sublayer(win);
    win.set_precipitation_selected_category_index(index);
    win.set_precipitation_selected_sublayer_index(0);
    info!("precipitation: category changed to {:?}", category);
    update_precipitation_display(win);
}

fn handle_sublayer_change(win: &MainWindow, index: i32) {
    *ACTIVE_SUBLAYER.lock().unwrap() = index as u32;
    // Update layer based on category and sublayer
    update_active_layer_from_category_and_sublayer(win);
    win.set_precipitation_selected_sublayer_index(index);
    info!("precipitation: sublayer changed to {}", index);
    update_precipitation_display(win);
}

fn handle_predictions_model_change(win: &MainWindow, index: i32) {
    let model = match index {
        0 => PrecipitationModel::Hrdps,
        1 => PrecipitationModel::Rdps,
        _ => PrecipitationModel::Hrdps,
    };
    *ACTIVE_MODEL.lock().unwrap() = model;
    win.set_precipitation_selected_predictions_model_index(index);
    info!("precipitation: predictions model changed to {:?}", model);
    update_precipitation_display(win);
}

fn handle_load_all_hours(win: &MainWindow) {
    let model = *ACTIVE_MODEL.lock().unwrap();
    let layer = *ACTIVE_LAYER.lock().unwrap();

    // Disable button and change text to indicate loading
    win.set_precipitation_load_all_hours_button_text("Loading all hours...".into());
    win.set_precipitation_load_all_hours_button_enabled(false);

    if model == PrecipitationModel::Radar {
        // For radar, only one hour, so just mark as done
        win.set_precipitation_load_all_hours_button_text("all hours fetched".into());
        return;
    }

    let max_hours = match model {
        PrecipitationModel::Hrdps => 48,
        PrecipitationModel::Rdps => 80,
        PrecipitationModel::Radar => 0, // shouldn't reach here
    };

    let bbox = if let Ok((lat, lon)) = coordinates::load_coordinates(win) {
        precipitation_bbox(lat, lon)
    } else {
        precipitation_bbox(45.0, -75.0)
    };

    let win_weak = win.as_weak();

    std::thread::spawn(move || {
        let mut handles = Vec::new();

        for hour in 0..=max_hours {
            let bbox_clone = bbox.clone();
            let handle = std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async move {
                    if let Ok(api) = GeoMetAPI::new() {
                        if let Err(e) = preload_model_core(&api, model, &bbox_clone, layer, hour).await {
                            error!(
                                "precipitation: failed to load {:?} {:?} T+{}h: {}",
                                model, layer, hour, e
                            );
                        } else {
                            info!("precipitation: loaded {:?} {:?} T+{}h", model, layer, hour);
                        }
                    } else {
                        error!("precipitation: failed to create GeoMetAPI for hour {}", hour);
                    }
                });
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            if let Err(e) = handle.join() {
                error!("precipitation: thread join failed: {:?}", e);
            }
        }

        info!("precipitation: all hours loaded for {:?} {:?}", model, layer);

        // Update UI after completion
        slint::invoke_from_event_loop(move || {
            if let Some(win) = win_weak.upgrade() {
                win.set_precipitation_load_all_hours_button_text("all hours fetched".into());
                win.set_precipitation_load_all_hours_button_enabled(false);
            }
        })
        .ok();
    });
}

fn update_active_layer_from_category_and_sublayer(win: &MainWindow) {
    let category = *ACTIVE_CATEGORY.lock().unwrap();
    let sublayer = *ACTIVE_SUBLAYER.lock().unwrap();
    let layer = match (category, sublayer) {
        (PrecipitationCategory::PrecipitationType, 0) => PrecipitationLayer::DominantPrecipType,
        (PrecipitationCategory::PrecipitationType, 1) => PrecipitationLayer::InstantPrecipType,
        (PrecipitationCategory::PrecipitationType, 2) => PrecipitationLayer::PrecipCharacter,
        (PrecipitationCategory::PrecipitationProbability, 0) => PrecipitationLayer::PrecipProb,
        (PrecipitationCategory::PrecipitationProbability, 1) => PrecipitationLayer::RainProb,
        (PrecipitationCategory::PrecipitationProbability, 2) => PrecipitationLayer::SnowProb,
        (PrecipitationCategory::PrecipitationProbability, 3) => PrecipitationLayer::DrizzleProb,
        (PrecipitationCategory::PrecipitationProbability, 4) => PrecipitationLayer::FreezingDrizzleProb,
        (PrecipitationCategory::PrecipitationProbability, 5) => PrecipitationLayer::FreezingRainProb,
        (PrecipitationCategory::PrecipitationProbability, 6) => PrecipitationLayer::IcePelletsProb,
        (PrecipitationCategory::PrecipitationProbability, 7) => PrecipitationLayer::LiquidPrecipProb,
        (PrecipitationCategory::PrecipitationProbability, 8) => PrecipitationLayer::FreezingPrecipProb,
        (PrecipitationCategory::PrecipitationProbability, 9) => PrecipitationLayer::ThunderstormProb,
        (PrecipitationCategory::PrecipitationProbability, 10) => PrecipitationLayer::BlowingSnowProb,
        (PrecipitationCategory::PrecipitationAmount, 0) => PrecipitationLayer::PrecipAmount,
        (PrecipitationCategory::PrecipitationAmount, 1) => PrecipitationLayer::LiquidPrecipCondAmt,
        (PrecipitationCategory::PrecipitationAmount, 2) => PrecipitationLayer::FreezingPrecipCondAmt,
        (PrecipitationCategory::PrecipitationAmount, 3) => PrecipitationLayer::IcePelletsCondAmt,
        (PrecipitationCategory::Temperature, 0) => PrecipitationLayer::AirTemp,
        (PrecipitationCategory::Temperature, 1) => PrecipitationLayer::DewPointTemp,
        (PrecipitationCategory::Snow, 0) => PrecipitationLayer::SnowDepth,
        (PrecipitationCategory::Snow, 1) => PrecipitationLayer::BlowingSnowPresence,
        (PrecipitationCategory::Visibility, 0) => PrecipitationLayer::IceFogVisibility,
        (PrecipitationCategory::Intensity, 0) => PrecipitationLayer::TotalPrecipIntensityIndex,
        _ => PrecipitationLayer::PrecipType,
    };
    *ACTIVE_LAYER.lock().unwrap() = layer;
}

fn generate_layer_dropdown_items(category: PrecipitationCategory) -> Vec<MenuItem> {
    match category {
        PrecipitationCategory::PrecipitationType => vec![
            MenuItem {
                icon: slint::Image::default(),
                text: "Dominant Precip Type".into(),
                trailing_text: "".into(),
                enabled: true,
            },
            MenuItem {
                icon: slint::Image::default(),
                text: "Instant Precip Type".into(),
                trailing_text: "".into(),
                enabled: true,
            },
            MenuItem {
                icon: slint::Image::default(),
                text: "Precip Character".into(),
                trailing_text: "".into(),
                enabled: true,
            },
        ],
        PrecipitationCategory::PrecipitationProbability => vec![
            MenuItem {
                icon: slint::Image::default(),
                text: "Any Precip Prob".into(),
                trailing_text: "".into(),
                enabled: true,
            },
            MenuItem {
                icon: slint::Image::default(),
                text: "Rain Prob".into(),
                trailing_text: "".into(),
                enabled: true,
            },
            MenuItem {
                icon: slint::Image::default(),
                text: "Snow Prob".into(),
                trailing_text: "".into(),
                enabled: true,
            },
            MenuItem {
                icon: slint::Image::default(),
                text: "Drizzle Prob".into(),
                trailing_text: "".into(),
                enabled: true,
            },
            MenuItem {
                icon: slint::Image::default(),
                text: "Freezing Drizzle Prob".into(),
                trailing_text: "".into(),
                enabled: true,
            },
            MenuItem {
                icon: slint::Image::default(),
                text: "Freezing Rain Prob".into(),
                trailing_text: "".into(),
                enabled: true,
            },
            MenuItem {
                icon: slint::Image::default(),
                text: "Ice Pellets Prob".into(),
                trailing_text: "".into(),
                enabled: true,
            },
            MenuItem {
                icon: slint::Image::default(),
                text: "Liquid Precip Prob".into(),
                trailing_text: "".into(),
                enabled: true,
            },
            MenuItem {
                icon: slint::Image::default(),
                text: "Freezing Precip Prob".into(),
                trailing_text: "".into(),
                enabled: true,
            },
            MenuItem {
                icon: slint::Image::default(),
                text: "Thunderstorm Prob".into(),
                trailing_text: "".into(),
                enabled: true,
            },
            MenuItem {
                icon: slint::Image::default(),
                text: "Blowing Snow Prob".into(),
                trailing_text: "".into(),
                enabled: true,
            },
        ],
        PrecipitationCategory::PrecipitationAmount => vec![
            MenuItem {
                icon: slint::Image::default(),
                text: "Total Precip Amount".into(),
                trailing_text: "".into(),
                enabled: true,
            },
            MenuItem {
                icon: slint::Image::default(),
                text: "Liquid Precip Amount".into(),
                trailing_text: "".into(),
                enabled: true,
            },
            MenuItem {
                icon: slint::Image::default(),
                text: "Freezing Precip Amount".into(),
                trailing_text: "".into(),
                enabled: true,
            },
            MenuItem {
                icon: slint::Image::default(),
                text: "Ice Pellets Amount".into(),
                trailing_text: "".into(),
                enabled: true,
            },
        ],
        PrecipitationCategory::Temperature => vec![
            MenuItem {
                icon: slint::Image::default(),
                text: "Air Temperature".into(),
                trailing_text: "".into(),
                enabled: true,
            },
            MenuItem {
                icon: slint::Image::default(),
                text: "Dew Point Temperature".into(),
                trailing_text: "".into(),
                enabled: true,
            },
        ],
        PrecipitationCategory::Snow => vec![
            MenuItem {
                icon: slint::Image::default(),
                text: "Snow Depth".into(),
                trailing_text: "".into(),
                enabled: true,
            },
            MenuItem {
                icon: slint::Image::default(),
                text: "Blowing Snow Presence".into(),
                trailing_text: "".into(),
                enabled: true,
            },
        ],
        PrecipitationCategory::Visibility => vec![
            MenuItem {
                icon: slint::Image::default(),
                text: "Ice Fog Visibility".into(),
                trailing_text: "".into(),
                enabled: true,
            },
        ],
        PrecipitationCategory::Intensity => vec![
            MenuItem {
                icon: slint::Image::default(),
                text: "Precip Intensity Index".into(),
                trailing_text: "".into(),
                enabled: true,
            },
        ],
    }
}

fn generate_model_dropdown_items() -> Vec<MenuItem> {
    vec![
        MenuItem {
            icon: slint::Image::default(),
            text: "HRDPS (2.5km)".into(),
            trailing_text: "".into(),
            enabled: true,
        },
        MenuItem {
            icon: slint::Image::default(),
            text: "RDPS (10km)".into(),
            trailing_text: "".into(),
            enabled: true,
        },
    ]
}
