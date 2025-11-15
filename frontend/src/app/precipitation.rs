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

use geomet::{BoundingBox, GeoMetAPI};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PrecipitationModel {
    Radar = 0,
    Hrdps = 1,
    Rdps = 2,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PrecipitationLayer {
    PrecipType = 0,
    PrecipProb = 1,
    PrecipAmount = 2,
    FreezingMixed = 3,
    SnowDepth = 4,
    TempPressure = 5,
    RadarRain = 6,
    RadarSnow = 7,
}

// Cache keys (model, layer, hour_index)
type ImgKey = (PrecipitationModel, PrecipitationLayer, u32);

static PRECIP_IMAGES: Lazy<Mutex<HashMap<ImgKey, Vec<u8>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static PRECIP_LEGENDS: Lazy<Mutex<HashMap<(PrecipitationModel, PrecipitationLayer), Vec<u8>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static PRECIP_POINT_DATA: Lazy<Mutex<PrecipitationPointData>> =
    Lazy::new(|| Mutex::new(PrecipitationPointData::default()));

static ACTIVE_MODEL: Lazy<Mutex<PrecipitationModel>> =
    Lazy::new(|| Mutex::new(PrecipitationModel::Hrdps));
static ACTIVE_LAYER: Lazy<Mutex<PrecipitationLayer>> =
    Lazy::new(|| Mutex::new(PrecipitationLayer::PrecipType));
static ACTIVE_HOUR: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(0));

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

    let w_refresh = main_window.as_weak();
    main_window.on_precipitation_refresh_now(move || {
        let w = w_refresh.clone();
        std::thread::spawn(move || {
            if let Some(win) = w.upgrade() {
                if let Ok((lat, lon)) = coordinates::load_coordinates(&win) {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async move {
                        if let Err(e) = refresh_precipitation_data(lat, lon, &win).await {
                            error!("Precipitation manual refresh failed: {e}");
                        }
                    });
                }
            }
        });
    });

    info!("Precipitation callbacks set up");
}

pub async fn fetch_initial_precipitation(lat: f64, lon: f64) -> Result<()> {
    // On first entry, eagerly fetch all available layers at T+0h for all models:
    // - Radar: PrecipType, SnowDepth
    // - HRDPS: all 6 layers (PrecipType, PrecipProb, PrecipAmount, FreezingMixed, SnowDepth, TempPressure)
    // - RDPS: all 6 layers (PrecipType, PrecipProb, PrecipAmount, FreezingMixed, SnowDepth, TempPressure)
    // (Further hours remain lazy-loaded via handle_time_change.)
    let bbox = precipitation_bbox(lat, lon);
    info!("precipitation: initial load, prefetching all layers at T+0h for bbox={:?}", bbox);

    let api = GeoMetAPI::new()?;

    // 1) Radar layers at T+0
    match fetch_radar_now(&api, &bbox).await {
        Ok((rain, snow, when)) => {
            let mut imgs = PRECIP_IMAGES.lock().unwrap();
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
            {
                let mut last = LAST_RADAR_FETCH.lock().unwrap();
                *last = Some(when);
            }
            info!(
                "precipitation: initial radar cached for slot {}",
                when.to_rfc3339()
            );
        }
        Err(e) => {
            error!("precipitation: initial radar fetch FAILED: {}", e);
        }
    }

    // 2) All HRDPS layers at T+0
    let hrdps_layers = &[
        PrecipitationLayer::PrecipType,
        PrecipitationLayer::PrecipProb,
        PrecipitationLayer::PrecipAmount,
        PrecipitationLayer::FreezingMixed,
        PrecipitationLayer::SnowDepth,
        PrecipitationLayer::TempPressure,
    ];

    for layer in hrdps_layers {
        if let Err(e) = preload_model_core(&api, PrecipitationModel::Hrdps, &bbox, *layer, 0).await {
            error!(
                "precipitation: initial HRDPS {:?} T+0h fetch FAILED: {}",
                layer, e
            );
        }
    }

    // 3) All RDPS layers at T+0
    let rdps_layers = &[
        PrecipitationLayer::PrecipType,
        PrecipitationLayer::PrecipProb,
        PrecipitationLayer::PrecipAmount,
        PrecipitationLayer::FreezingMixed,
        PrecipitationLayer::SnowDepth,
        PrecipitationLayer::TempPressure,
    ];

    for layer in rdps_layers {
        if let Err(e) = preload_model_core(&api, PrecipitationModel::Rdps, &bbox, *layer, 0).await {
            error!(
                "precipitation: initial RDPS {:?} T+0h fetch FAILED: {}",
                layer, e
            );
        }
    }

    // Initialize active selection to HRDPS PrecipType T+0h as primary default.
    {
        let mut model = ACTIVE_MODEL.lock().unwrap();
        let mut layer = ACTIVE_LAYER.lock().unwrap();
        let mut hour = ACTIVE_HOUR.lock().unwrap();
        *model = PrecipitationModel::Hrdps;
        *layer = PrecipitationLayer::PrecipType;
        *hour = 0;
    }

    // Prefetch RDPS PrecipType at T+48h (default for RDPS)
    if let Err(e) = preload_model_core(&api, PrecipitationModel::Rdps, &bbox, PrecipitationLayer::PrecipType, 48).await {
        error!("precipitation: initial RDPS PrecipType T+48h fetch FAILED: {}", e);
    }

    // Mark model data timestamp for reference (lazy refresh beyond this remains unchanged).
    {
        let mut last = LAST_MODEL_FETCH.lock().unwrap();
        *last = Some(Utc::now());
    }

    info!("precipitation: initial all layers at T+0h and RDPS T+48h prefetched");
    Ok(())
}

pub async fn refresh_precipitation_data(
    lat: f64,
    lon: f64,
    main_window: &MainWindow,
) -> Result<()> {
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
                    info!(
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
    let layer_name = match (model, layer) {
        (PrecipitationModel::Radar, PrecipitationLayer::RadarRain) => "RADAR_1KM_RRAI",
        (PrecipitationModel::Radar, PrecipitationLayer::RadarSnow) => "RADAR_1KM_RSNO",
        (PrecipitationModel::Hrdps, PrecipitationLayer::PrecipType) => {
            "HRDPS-WEonG_2.5km_DominantPrecipType"
        }
        (PrecipitationModel::Hrdps, PrecipitationLayer::PrecipProb) => {
            "HRDPS-WEonG_2.5km_Precip-Prob"
        }
        (PrecipitationModel::Hrdps, PrecipitationLayer::PrecipAmount) => {
            "HRDPS-WEonG_2.5km_PrecipCondAmt"
        }
        (PrecipitationModel::Hrdps, PrecipitationLayer::TempPressure) => {
            "HRDPS-WEonG_2.5km_AirTemp"
        }
        (PrecipitationModel::Hrdps, PrecipitationLayer::FreezingMixed) => {
            "HRDPS-WEonG_2.5km_FreezingPrecip-Prob"
        }
        (PrecipitationModel::Hrdps, PrecipitationLayer::SnowDepth) => "HRDPS.CONTINENTAL_SD",
        (PrecipitationModel::Rdps, PrecipitationLayer::SnowDepth) => "RDPS.ETA_SD",
        (PrecipitationModel::Rdps, PrecipitationLayer::PrecipType) => {
            "RDPS-WEonG_10km_DominantPrecipType"
        }
        (PrecipitationModel::Rdps, PrecipitationLayer::PrecipProb) => {
            "RDPS-WEonG_10km_Precip-Prob"
        }
        (PrecipitationModel::Rdps, PrecipitationLayer::PrecipAmount) => {
            "RDPS-WEonG_10km_PrecipCondAmt"
        }
        (PrecipitationModel::Rdps, PrecipitationLayer::TempPressure) => {
            "RDPS-WEonG_10km_AirTemp"
        }
        (PrecipitationModel::Rdps, PrecipitationLayer::FreezingMixed) => {
            "RDPS-WEonG_10km_FreezingPrecip-Prob"
        }
        // For unsupported (model, layer) combinations we do nothing.
        _ => return Ok(()),
    };

    let mut imgs = PRECIP_IMAGES.lock().unwrap();
    let mut legends = PRECIP_LEGENDS.lock().unwrap();

    // Legend (once per (model, layer))
    let legend_key = (model, layer);
    if !legends.contains_key(&legend_key) {
        if let Ok(lg) = api
            .get_legend_graphic(layer_name, None, "image/png", None)
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

    // Use appropriate style for radar layers
    let style = match model {
        PrecipitationModel::Radar => {
            match layer {
                PrecipitationLayer::RadarRain => Some("Radar-Rain"),
                PrecipitationLayer::RadarSnow => Some("Radar-Snow"),
                _ => None,
            }
        }
        _ => None,
    };

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

    // Fallback order: exact selection -> same model/layer hour 0 -> radar -> any HRDPS
    let mut chosen_bytes: Option<Vec<u8>> = imgs.get(&(model, layer, hour)).cloned();

    if chosen_bytes.is_none() {
        if let Some(b) = imgs.get(&(model, layer, 0)).cloned() {
            chosen_bytes = Some(b);
        }
    }

    if chosen_bytes.is_none() && model != PrecipitationModel::Radar {
        if let Some(b) = imgs
            .get(&(PrecipitationModel::Radar, PrecipitationLayer::RadarRain, 0))
            .cloned()
        {
            chosen_bytes = Some(b);
        }
    }

    if chosen_bytes.is_none() {
        // last resort: any HRDPS PrecipType
        for h in [0u32, 1, 2, 3, 6, 12, 24, 36, 48] {
            if let Some(b) = imgs
                .get(&(PrecipitationModel::Hrdps, PrecipitationLayer::PrecipType, h))
                .cloned()
            {
                chosen_bytes = Some(b);
                break;
            }
        }
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
    let active_img = chosen_bytes
        .as_ref()
        .and_then(|b| decode_image_safe(b, "active"))
        .unwrap_or_default();

    let legend_img = legend_bytes
        .as_ref()
        .and_then(|b| decode_image_safe(b, "legend"))
        .unwrap_or_default();

    // Use the map.rs-provided basemap as background, precipitation as overlay in Slint
    win.set_precipitation_base_map_image(base_map_img);
    win.set_precipitation_active_precip_image(active_img);
    win.set_precipitation_legend_image(legend_img);

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
    };
    let time_label = if model == PrecipitationModel::Radar {
        "Now".to_string()
    } else {
        format!("T+{}h", hour)
    };

    win.set_precipitation_active_model_label(model_label.into());
    win.set_precipitation_active_layer_label(layer_label.into());
    win.set_precipitation_active_time_label(time_label.into());
    win.set_precipitation_last_update_text("".into()); // can be filled from LAST_* timestamps

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
}

// Callback handlers

fn handle_model_change(win: &MainWindow, index: i32) {
    let model = match index {
        0 => PrecipitationModel::Radar,
        1 => PrecipitationModel::Hrdps,
        2 => PrecipitationModel::Rdps,
        _ => PrecipitationModel::Hrdps,
    };

    *ACTIVE_MODEL.lock().unwrap() = model;

    // Set default hour based on model
    let default_hour = match model {
        PrecipitationModel::Radar => 0,
        PrecipitationModel::Hrdps => 0,
        PrecipitationModel::Rdps => 48, // Default RDPS to T+48h
    };

    *ACTIVE_HOUR.lock().unwrap() = default_hour;

    win.set_precipitation_selected_model_index(index);
    win.set_precipitation_selected_time_index(default_hour as i32);

    if model == PrecipitationModel::Radar {
        *ACTIVE_LAYER.lock().unwrap() = PrecipitationLayer::RadarRain;
        win.set_precipitation_selected_layer_index(6);
    } else {
        *ACTIVE_LAYER.lock().unwrap() = PrecipitationLayer::PrecipType;
        win.set_precipitation_selected_layer_index(0);
    }

    info!("precipitation: model changed to {:?}, hour reset to 0", model);

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
