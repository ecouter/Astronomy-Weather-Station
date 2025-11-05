use std::rc::Rc;
use slint::{VecModel};
use crate::MainWindow;
use crate::app::coordinates::load_coordinates;
use crate::app::utils::parse_hour;

#[derive(Clone)]
pub struct NightCondition {
    day: String,
    hour: u32,
    condition: String,
    total_clouds: u32,
    is_evening: bool, // true for evening hours (after sunset), false for morning hours (before sunrise)
}

pub async fn update_clearoutside_data(main_window: &MainWindow) -> Result<(), Box<dyn std::error::Error>> {
    use clearoutside::ClearOutsideAPI;

    // Load coordinates
    let (lat, lon) = load_coordinates(main_window)?;

    // Format coordinates for ClearOutside (lat.lon with 2 decimals)
    let lat_str = format!("{:.2}", lat);
    let lon_str = format!("{:.2}", lon);

    // Create API instance
    let api = ClearOutsideAPI::new(&lat_str, &lon_str, Some("midnight")).await?;

    // Fetch and parse data
    let forecast = api.pull()?;

    // Process data for night hours display
    let night_conditions = process_clearoutside_data(&forecast)?;
    update_clearoutside_display(main_window, night_conditions);

    // Also update meteoblue data using the same coordinates
    if let Err(e) = crate::app::meteoblue::update_meteoblue_data(main_window, &forecast).await {
        eprintln!("Failed to update meteoblue data: {}", e);
        main_window.set_error_message(format!("Failed to update meteoblue data: {}", e).into());
    }

    Ok(())
}

fn process_clearoutside_data(forecast: &clearoutside::ClearOutsideForecast) -> Result<Vec<NightCondition>, Box<dyn std::error::Error>> {
    let mut night_conditions = Vec::new();

    // Sort days by key (day-0, day-1, etc.)
    let mut sorted_days: Vec<_> = forecast.forecast.iter().collect();
    sorted_days.sort_by_key(|(k, _)| *k);

    for (i, (day_key, day_info)) in sorted_days.iter().enumerate() {
        // Parse sunset time for current day
        let sunset_hour = parse_hour(&day_info.sun.set)?;

        // Get hours after sunset from current day
        let evening_hours: Vec<_> = day_info.hours.iter()
            .filter(|(hour_str, _)| {
                if let Ok(hour) = hour_str.parse::<u32>() {
                    hour > sunset_hour
                } else {
                    false
                }
            })
            .collect();

        // Add evening hours from current day
        for (hour_str, hourly_data) in &evening_hours {
            night_conditions.push(NightCondition {
                day: format!("night-{}", i), // Use night-X instead of day-X
                hour: hour_str.parse().unwrap_or(0),
                condition: hourly_data.conditions.clone(),
                total_clouds: hourly_data.total_clouds.parse().unwrap_or(0),
                is_evening: true, // Evening hours (after sunset)
            });
        }

        // Get hours before sunrise from next day (if available)
        if i + 1 < sorted_days.len() {
            let next_day_info = &sorted_days[i + 1].1;
            let next_sunrise_hour = parse_hour(&next_day_info.sun.rise)?;

            let morning_hours: Vec<_> = next_day_info.hours.iter()
                .filter(|(hour_str, _)| {
                    if let Ok(hour) = hour_str.parse::<u32>() {
                        hour < next_sunrise_hour && hour <= 11 // Only morning hours 0-11
                    } else {
                        false
                    }
                })
                .collect();

            // Add morning hours from next day, but associate with current night
            for (hour_str, hourly_data) in &morning_hours {
                night_conditions.push(NightCondition {
                    day: format!("night-{}", i), // Associate with current night
                    hour: hour_str.parse().unwrap_or(0),
                    condition: hourly_data.conditions.clone(),
                    total_clouds: hourly_data.total_clouds.parse().unwrap_or(0),
                    is_evening: false, // Morning hours (before sunrise)
                });
            }
        }
    }

    // Sort all night conditions by night, then by evening/morning, then by hour
    night_conditions.sort_by(|a, b| {
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

    Ok(night_conditions)
}

fn update_clearoutside_display(main_window: &MainWindow, conditions: Vec<NightCondition>) {

    // Group conditions by day
    use std::collections::HashMap;
    let mut conditions_by_day: HashMap<String, Vec<&NightCondition>> = HashMap::new();

    for condition in &conditions {
        conditions_by_day.entry(condition.day.clone()).or_insert(Vec::new()).push(condition);
    }

    // Sort days
    let mut sorted_days: Vec<_> = conditions_by_day.keys().collect();
    sorted_days.sort();

    // Set data for each day
    for (day_idx, day_key) in sorted_days.iter().enumerate() {
        if let Some(day_conditions) = conditions_by_day.get(*day_key) {
            // Conditions are already sorted by the global sorting logic
            let conditions_vec: Vec<slint::SharedString> = day_conditions.iter()
                .map(|c| c.condition.clone().into())
                .collect();
            let clouds_vec: Vec<i32> = day_conditions.iter()
                .map(|c| c.total_clouds as i32)
                .collect();
            let hours_vec: Vec<i32> = day_conditions.iter()
                .map(|c| c.hour as i32)
                .collect();

            match day_idx {
                0 => {
                    main_window.set_day0_conditions(Rc::new(VecModel::from(conditions_vec)).into());
                    main_window.set_day0_clouds(Rc::new(VecModel::from(clouds_vec)).into());
                    main_window.set_day0_hours(Rc::new(VecModel::from(hours_vec)).into());
                }
                1 => {
                    main_window.set_day1_conditions(Rc::new(VecModel::from(conditions_vec)).into());
                    main_window.set_day1_clouds(Rc::new(VecModel::from(clouds_vec)).into());
                    main_window.set_day1_hours(Rc::new(VecModel::from(hours_vec)).into());
                }
                2 => {
                    main_window.set_day2_conditions(Rc::new(VecModel::from(conditions_vec)).into());
                    main_window.set_day2_clouds(Rc::new(VecModel::from(clouds_vec)).into());
                    main_window.set_day2_hours(Rc::new(VecModel::from(hours_vec)).into());
                }
                3 => {
                    main_window.set_day3_conditions(Rc::new(VecModel::from(conditions_vec)).into());
                    main_window.set_day3_clouds(Rc::new(VecModel::from(clouds_vec)).into());
                    main_window.set_day3_hours(Rc::new(VecModel::from(hours_vec)).into());
                }
                4 => {
                    main_window.set_day4_conditions(Rc::new(VecModel::from(conditions_vec)).into());
                    main_window.set_day4_clouds(Rc::new(VecModel::from(clouds_vec)).into());
                    main_window.set_day4_hours(Rc::new(VecModel::from(hours_vec)).into());
                }
                5 => {
                    main_window.set_day5_conditions(Rc::new(VecModel::from(conditions_vec)).into());
                    main_window.set_day5_clouds(Rc::new(VecModel::from(clouds_vec)).into());
                    main_window.set_day5_hours(Rc::new(VecModel::from(hours_vec)).into());
                }
                6 => {
                    main_window.set_day6_conditions(Rc::new(VecModel::from(conditions_vec)).into());
                    main_window.set_day6_clouds(Rc::new(VecModel::from(clouds_vec)).into());
                    main_window.set_day6_hours(Rc::new(VecModel::from(hours_vec)).into());
                }
                _ => {}
            }
        }
    }
}
