use std::rc::Rc;
use slint::{VecModel};
use chrono::Timelike;
use crate::MainWindow;
use crate::app::utils::parse_hour;

#[derive(Clone)]
pub struct MeteoBlueNightData {
    day: String,
    hour: u8,
    is_evening: bool, // true for evening hours (after sunset), false for morning hours (before sunrise)
    clouds_low_pct: u8,
    clouds_mid_pct: u8,
    clouds_high_pct: u8,
    seeing_arcsec: f32,
    index1: u8,
    index2: u8,
    jetstream_ms: Option<f32>,
    bad_layers_bot_km: Option<f32>,
    bad_layers_top_km: Option<f32>,
    bad_layers_k_per_100m: Option<f32>,
    temp_c: f32,
    humidity_pct: u8,
}

pub async fn fetch_meteoblue_data(lat: f64, lon: f64, clearoutside_forecast: &clearoutside::ClearOutsideForecast) -> Result<Vec<MeteoBlueNightData>, Box<dyn std::error::Error>> {
    use meteoblue::fetch_meteoblue_data;

    // Fetch meteoblue data
    let meteoblue_data = fetch_meteoblue_data(lat, lon).await?;

    // Process data for night hours display using ClearOutside sunrise/sunset
    let night_data = process_meteoblue_night_data(&meteoblue_data, clearoutside_forecast)?;

    Ok(night_data)
}

fn process_meteoblue_night_data(meteoblue_data: &[meteoblue::SeeingData], clearoutside_forecast: &clearoutside::ClearOutsideForecast) -> Result<Vec<MeteoBlueNightData>, Box<dyn std::error::Error>> {
    let mut night_data = Vec::new();

    // Group meteoblue data by day
    use std::collections::HashMap;
    let mut meteoblue_by_day: HashMap<String, Vec<&meteoblue::SeeingData>> = HashMap::new();
    for data_point in meteoblue_data {
        meteoblue_by_day.entry(data_point.day.clone()).or_insert(Vec::new()).push(data_point);
    }

    // Sort meteoblue days
    let mut sorted_meteoblue_days: Vec<_> = meteoblue_by_day.keys().collect();
    sorted_meteoblue_days.sort();

    // Sort clearoutside days
    let mut sorted_clearoutside_days: Vec<_> = clearoutside_forecast.forecast.iter().collect();
    sorted_clearoutside_days.sort_by_key(|(k, _)| *k);

    // Process each night (similar to clearoutside logic)
    for i in 0..sorted_clearoutside_days.len() {
        let (_, day_info) = &sorted_clearoutside_days[i];
        let sunset_hour = parse_hour(&day_info.sun.set)?;

        // Get evening hours from current meteoblue day (after sunset)
        if let Some(day_key) = sorted_meteoblue_days.get(i) {
            if let Some(day_data_points) = meteoblue_by_day.get(*day_key) {
                let evening_hours: Vec<&meteoblue::SeeingData> = day_data_points.iter()
                    .filter(|data| (data.hour as u32) > sunset_hour)
                    .cloned()
                    .collect();

                // Add evening hours to night data
                for data_point in evening_hours {
                    night_data.push(MeteoBlueNightData {
                        day: format!("night-{}", i),
                        hour: data_point.hour,
                        is_evening: true, // Evening hours (after sunset)
                        clouds_low_pct: data_point.clouds_low_pct,
                        clouds_mid_pct: data_point.clouds_mid_pct,
                        clouds_high_pct: data_point.clouds_high_pct,
                        seeing_arcsec: data_point.seeing_arcsec,
                        index1: data_point.index1,
                        index2: data_point.index2,
                        jetstream_ms: data_point.jetstream_ms,
                        bad_layers_bot_km: data_point.bad_layers_bot_km,
                        bad_layers_top_km: data_point.bad_layers_top_km,
                        bad_layers_k_per_100m: data_point.bad_layers_k_per_100m,
                        temp_c: data_point.temp_c,
                        humidity_pct: data_point.humidity_pct,
                    });
                }
            }
        }

        // Get morning hours from next meteoblue day (before sunrise)
        if i + 1 < sorted_clearoutside_days.len() {
            let (_, next_day_info) = &sorted_clearoutside_days[i + 1];
            let sunrise_hour = parse_hour(&next_day_info.sun.rise)?;

            if let Some(next_day_key) = sorted_meteoblue_days.get(i + 1) {
                if let Some(next_day_data_points) = meteoblue_by_day.get(*next_day_key) {
                    let morning_hours: Vec<&meteoblue::SeeingData> = next_day_data_points.iter()
                        .filter(|data| {
                            let hour = data.hour as u32;
                            hour < sunrise_hour && hour <= 11 // Only morning hours 0-11
                        })
                        .cloned()
                        .collect();

                    // Add morning hours to current night
                    for data_point in morning_hours {
                        night_data.push(MeteoBlueNightData {
                            day: format!("night-{}", i), // Associate with current night
                            hour: data_point.hour,
                            is_evening: false, // Morning hours (before sunrise)
                            clouds_low_pct: data_point.clouds_low_pct,
                            clouds_mid_pct: data_point.clouds_mid_pct,
                            clouds_high_pct: data_point.clouds_high_pct,
                            seeing_arcsec: data_point.seeing_arcsec,
                            index1: data_point.index1,
                            index2: data_point.index2,
                            jetstream_ms: data_point.jetstream_ms,
                            bad_layers_bot_km: data_point.bad_layers_bot_km,
                            bad_layers_top_km: data_point.bad_layers_top_km,
                            bad_layers_k_per_100m: data_point.bad_layers_k_per_100m,
                            temp_c: data_point.temp_c,
                            humidity_pct: data_point.humidity_pct,
                        });
                    }
                }
            }
        }
    }

    // Sort all night data by night, then by evening/morning, then by hour (same as clearoutside)
    night_data.sort_by(|a, b| {
        let night_cmp = a.day.cmp(&b.day);
        if night_cmp == std::cmp::Ordering::Equal {
            // Within the same night, evening hours come before morning hours
            match (a.is_evening, b.is_evening) {
                (true, false) => std::cmp::Ordering::Less,   // evening before morning
                (false, true) => std::cmp::Ordering::Greater, // morning after evening
                _ => a.hour.cmp(&b.hour), // same type (both evening or both morning), sort by hour
            }
        } else {
            night_cmp
        }
    });

    Ok(night_data)
}

pub fn set_meteoblue_data(main_window: &MainWindow, night_data: Vec<MeteoBlueNightData>) {

    // Group data by day
    use std::collections::HashMap;
    let mut data_by_day: HashMap<String, Vec<&MeteoBlueNightData>> = HashMap::new();

    for data_point in &night_data {
        data_by_day.entry(data_point.day.clone()).or_insert(Vec::new()).push(data_point);
    }

    // Sort days
    let mut sorted_days: Vec<_> = data_by_day.keys().collect();
    sorted_days.sort();

    // Get current hour for highlighting (convert to local time for the location)
    // Coordinates are in Eastern Time Zone (UTC-4), so convert UTC to local
    let utc_now = chrono::Utc::now();
    let eastern_offset = chrono::FixedOffset::west_opt(4 * 3600).unwrap(); // UTC-4
    let local_time = utc_now.with_timezone(&eastern_offset);
    let current_hour = local_time.hour() as i32;
    let mut current_hour_index = -1;

    // For now, just handle the first night (night-0) - we can expand this later
    if let Some(day_data) = data_by_day.get("night-0") {
        // Extract data arrays in the specified order
        let hours: Vec<i32> = day_data.iter().map(|d| d.hour as i32).collect();
        let clouds_low: Vec<i32> = day_data.iter().map(|d| d.clouds_low_pct as i32).collect();
        let clouds_mid: Vec<i32> = day_data.iter().map(|d| d.clouds_mid_pct as i32).collect();
        let clouds_high: Vec<i32> = day_data.iter().map(|d| d.clouds_high_pct as i32).collect();
        let seeing: Vec<f32> = day_data.iter().map(|d| d.seeing_arcsec).collect();
        let index1: Vec<i32> = day_data.iter().map(|d| d.index1 as i32).collect();
        let index2: Vec<i32> = day_data.iter().map(|d| d.index2 as i32).collect();
        let jetstream: Vec<f32> = day_data.iter().map(|d| d.jetstream_ms.unwrap_or(0.0)).collect();
        let bad_layers_bot: Vec<f32> = day_data.iter().map(|d| d.bad_layers_bot_km.unwrap_or(0.0)).collect();
        let bad_layers_top: Vec<f32> = day_data.iter().map(|d| d.bad_layers_top_km.unwrap_or(0.0)).collect();
        let bad_layers_k: Vec<f32> = day_data.iter().map(|d| d.bad_layers_k_per_100m.unwrap_or(0.0)).collect();
        let temp: Vec<f32> = day_data.iter().map(|d| d.temp_c).collect();
        let humidity: Vec<i32> = day_data.iter().map(|d| d.humidity_pct as i32).collect();

        // Find the current hour index
        for (idx, &hour) in hours.iter().enumerate() {
            if hour == current_hour {
                current_hour_index = idx as i32;
                break;
            }
        }

        // Get the length before moving the vectors
        let hours_count = hours.len();

        // Set the data to UI properties
        main_window.set_night_hours(Rc::new(VecModel::from(hours)).into());
        main_window.set_night_clouds_low(Rc::new(VecModel::from(clouds_low)).into());
        main_window.set_night_clouds_mid(Rc::new(VecModel::from(clouds_mid)).into());
        main_window.set_night_clouds_high(Rc::new(VecModel::from(clouds_high)).into());
        main_window.set_night_seeing(Rc::new(VecModel::from(seeing)).into());
        main_window.set_night_index1(Rc::new(VecModel::from(index1)).into());
        main_window.set_night_index2(Rc::new(VecModel::from(index2)).into());
        main_window.set_night_jetstream(Rc::new(VecModel::from(jetstream)).into());
        main_window.set_night_bad_layers_bot(Rc::new(VecModel::from(bad_layers_bot)).into());
        main_window.set_night_bad_layers_top(Rc::new(VecModel::from(bad_layers_top)).into());
        main_window.set_night_bad_layers_k(Rc::new(VecModel::from(bad_layers_k)).into());
        main_window.set_night_temp(Rc::new(VecModel::from(temp)).into());
        main_window.set_night_humidity(Rc::new(VecModel::from(humidity)).into());
        main_window.set_current_hour_index(current_hour_index);

        println!("Night 0 data set to UI: {} hours, current hour index: {}", hours_count, current_hour_index);
    } else {
        // Clear the data if no night-0 data available
        main_window.set_night_hours(Rc::new(VecModel::from(Vec::<i32>::new())).into());
        main_window.set_night_clouds_low(Rc::new(VecModel::from(Vec::<i32>::new())).into());
        main_window.set_night_clouds_mid(Rc::new(VecModel::from(Vec::<i32>::new())).into());
        main_window.set_night_clouds_high(Rc::new(VecModel::from(Vec::<i32>::new())).into());
        main_window.set_night_seeing(Rc::new(VecModel::from(Vec::<f32>::new())).into());
        main_window.set_night_index1(Rc::new(VecModel::from(Vec::<i32>::new())).into());
        main_window.set_night_index2(Rc::new(VecModel::from(Vec::<i32>::new())).into());
        main_window.set_night_jetstream(Rc::new(VecModel::from(Vec::<f32>::new())).into());
        main_window.set_night_bad_layers_bot(Rc::new(VecModel::from(Vec::<f32>::new())).into());
        main_window.set_night_bad_layers_top(Rc::new(VecModel::from(Vec::<f32>::new())).into());
        main_window.set_night_bad_layers_k(Rc::new(VecModel::from(Vec::<f32>::new())).into());
        main_window.set_night_temp(Rc::new(VecModel::from(Vec::<f32>::new())).into());
        main_window.set_night_humidity(Rc::new(VecModel::from(Vec::<i32>::new())).into());
        main_window.set_current_hour_index(-1);
    }
}
