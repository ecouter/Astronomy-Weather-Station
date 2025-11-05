use std::sync::Mutex;
use once_cell::sync::Lazy;
use chrono::{Timelike, Datelike};
use slint::ComponentHandle;
use crate::MainWindow;
use crate::app::utils::{decode_png_to_slint_image, calculate_env_canada_forecast_time};

pub static ENV_CANADA_CLOUDS_IMAGES: Lazy<Mutex<Vec<Vec<u8>>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static ENV_CANADA_CLOUDS_INDEX: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));
pub static ENV_CANADA_SURFACE_WIND_IMAGES: Lazy<Mutex<Vec<Vec<u8>>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static ENV_CANADA_SURFACE_WIND_INDEX: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));
pub static ENV_CANADA_SEEING_IMAGES: Lazy<Mutex<Vec<Vec<u8>>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static ENV_CANADA_SEEING_INDEX: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));
pub static ENV_CANADA_TEMPERATURE_IMAGES: Lazy<Mutex<Vec<Vec<u8>>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static ENV_CANADA_TEMPERATURE_INDEX: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));
pub static ENV_CANADA_TRANSPARENCY_IMAGES: Lazy<Mutex<Vec<Vec<u8>>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static ENV_CANADA_TRANSPARENCY_INDEX: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));
pub static ENV_CANADA_RELATIVE_HUMIDITY_IMAGES: Lazy<Mutex<Vec<Vec<u8>>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static ENV_CANADA_RELATIVE_HUMIDITY_INDEX: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));
pub static ENV_CANADA_MODEL_RUN_INFO: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::new()));
pub static ENV_CANADA_MODEL_RUN_STR: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::new()));

pub fn setup_environment_canada_callbacks(main_window: &MainWindow) {
    // Clouds navigation
    let main_window_weak5 = main_window.as_weak();
    main_window.on_env_canada_clouds_previous(move || {
        debug!("Environment Canada clouds previous button clicked");
        let window_weak = main_window_weak5.clone();
        slint::invoke_from_event_loop(move || {
            debug!("Processing Environment Canada clouds previous in event loop");
            let window = window_weak.upgrade();
            if let Some(window) = window {
                debug!("Window upgraded successfully");
                if let Ok(images) = ENV_CANADA_CLOUDS_IMAGES.try_lock() {
                    debug!("ENV_CANADA_CLOUDS_IMAGES lock acquired, {} images", images.len());
                    if let Ok(mut index) = ENV_CANADA_CLOUDS_INDEX.try_lock() {
                        debug!("ENV_CANADA_CLOUDS_INDEX lock acquired, current index: {}", *index);
                        if !images.is_empty() {
                            if *index == 0 {
                                *index = images.len() - 1;
                            } else {
                                *index -= 1;
                            }
                            debug!("New index: {}", *index);
                            let current_image_data = &images[*index];
                            let model_run_str = ENV_CANADA_MODEL_RUN_STR.lock().unwrap();
                            let counter_text = calculate_env_canada_forecast_time(&model_run_str, *index, false);
                            match decode_png_to_slint_image(current_image_data) {
                                Ok(slint_image) => {
                                    window.set_env_clouds_image(slint_image);
                                    window.set_env_clouds_counter(counter_text.into());
                                }
                                Err(e) => {
                                    error!("Failed to decode Environment Canada clouds image: {}", e);
                                }
                            }
                        } else {
                            debug!("No Environment Canada clouds images available");
                        }
                    } else {
                        debug!("Failed to acquire ENV_CANADA_CLOUDS_INDEX lock");
                    }
                } else {
                    debug!("Failed to acquire ENV_CANADA_CLOUDS_IMAGES lock");
                }
            } else {
                debug!("Failed to upgrade window weak reference");
            }
        }).unwrap();
    });

    let main_window_weak6 = main_window.as_weak();
    main_window.on_env_canada_clouds_next(move || {
        debug!("Environment Canada clouds next button clicked");
        let window_weak = main_window_weak6.clone();
        slint::invoke_from_event_loop(move || {
            debug!("Processing Environment Canada clouds next in event loop");
            let window = window_weak.upgrade();
            if let Some(window) = window {
                debug!("Window upgraded successfully");
                if let Ok(images) = ENV_CANADA_CLOUDS_IMAGES.try_lock() {
                    debug!("ENV_CANADA_CLOUDS_IMAGES lock acquired, {} images", images.len());
                    if let Ok(mut index) = ENV_CANADA_CLOUDS_INDEX.try_lock() {
                        debug!("ENV_CANADA_CLOUDS_INDEX lock acquired, current index: {}", *index);
                        if !images.is_empty() {
                            *index = (*index + 1) % images.len();
                            debug!("New index: {}", *index);
                            let current_image_data = &images[*index];
                            let model_run_str = ENV_CANADA_MODEL_RUN_STR.lock().unwrap();
                            let counter_text = calculate_env_canada_forecast_time(&model_run_str, *index, false);
                            match decode_png_to_slint_image(current_image_data) {
                                Ok(slint_image) => {
                                    window.set_env_clouds_image(slint_image);
                                    window.set_env_clouds_counter(counter_text.into());
                                }
                                Err(e) => {
                                    error!("Failed to decode Environment Canada clouds image: {}", e);
                                }
                            }
                        } else {
                            debug!("No Environment Canada clouds images available");
                        }
                    } else {
                        debug!("Failed to acquire ENV_CANADA_CLOUDS_INDEX lock");
                    }
                } else {
                    debug!("Failed to acquire ENV_CANADA_CLOUDS_IMAGES lock");
                }
            } else {
                debug!("Failed to upgrade window weak reference");
            }
        }).unwrap();
    });

    // Surface wind navigation
    let main_window_weak7 = main_window.as_weak();
    main_window.on_env_canada_surface_wind_previous(move || {
        debug!("Environment Canada surface wind previous button clicked");
        let window_weak = main_window_weak7.clone();
        slint::invoke_from_event_loop(move || {
            debug!("Processing Environment Canada surface wind previous in event loop");
            let window = window_weak.upgrade();
            if let Some(window) = window {
                debug!("Window upgraded successfully");
                if let Ok(images) = ENV_CANADA_SURFACE_WIND_IMAGES.try_lock() {
                    debug!("ENV_CANADA_SURFACE_WIND_IMAGES lock acquired, {} images", images.len());
                    if let Ok(mut index) = ENV_CANADA_SURFACE_WIND_INDEX.try_lock() {
                        debug!("ENV_CANADA_SURFACE_WIND_INDEX lock acquired, current index: {}", *index);
                        if !images.is_empty() {
                            if *index == 0 {
                                *index = images.len() - 1;
                            } else {
                                *index -= 1;
                            }
                            debug!("New index: {}", *index);
                            let current_image_data = &images[*index];
                            let model_run_str = ENV_CANADA_MODEL_RUN_STR.lock().unwrap();
                            let counter_text = calculate_env_canada_forecast_time(&model_run_str, *index, false);
                            match decode_png_to_slint_image(current_image_data) {
                                Ok(slint_image) => {
                                    window.set_env_surface_wind_image(slint_image);
                                    window.set_env_surface_wind_counter(counter_text.into());
                                }
                                Err(e) => {
                                    error!("Failed to decode Environment Canada surface wind image: {}", e);
                                }
                            }
                        } else {
                            debug!("No Environment Canada surface wind images available");
                        }
                    } else {
                        debug!("Failed to acquire ENV_CANADA_SURFACE_WIND_INDEX lock");
                    }
                } else {
                    debug!("Failed to acquire ENV_CANADA_SURFACE_WIND_IMAGES lock");
                }
            } else {
                debug!("Failed to upgrade window weak reference");
            }
        }).unwrap();
    });

    let main_window_weak8 = main_window.as_weak();
    main_window.on_env_canada_surface_wind_next(move || {
        debug!("Environment Canada surface wind next button clicked");
        let window_weak = main_window_weak8.clone();
        slint::invoke_from_event_loop(move || {
            debug!("Processing Environment Canada surface wind next in event loop");
            let window = window_weak.upgrade();
            if let Some(window) = window {
                debug!("Window upgraded successfully");
                if let Ok(images) = ENV_CANADA_SURFACE_WIND_IMAGES.try_lock() {
                    debug!("ENV_CANADA_SURFACE_WIND_IMAGES lock acquired, {} images", images.len());
                    if let Ok(mut index) = ENV_CANADA_SURFACE_WIND_INDEX.try_lock() {
                        debug!("ENV_CANADA_SURFACE_WIND_INDEX lock acquired, current index: {}", *index);
                        if !images.is_empty() {
                            *index = (*index + 1) % images.len();
                            debug!("New index: {}", *index);
                            let current_image_data = &images[*index];
                            let model_run_str = ENV_CANADA_MODEL_RUN_STR.lock().unwrap();
                            let counter_text = calculate_env_canada_forecast_time(&model_run_str, *index, false);
                            match decode_png_to_slint_image(current_image_data) {
                                Ok(slint_image) => {
                                    window.set_env_surface_wind_image(slint_image);
                                    window.set_env_surface_wind_counter(counter_text.into());
                                }
                                Err(e) => {
                                    error!("Failed to decode Environment Canada surface wind image: {}", e);
                                }
                            }
                        } else {
                            debug!("No Environment Canada surface wind images available");
                        }
                    } else {
                        debug!("Failed to acquire ENV_CANADA_SURFACE_WIND_INDEX lock");
                    }
                } else {
                    debug!("Failed to acquire ENV_CANADA_SURFACE_WIND_IMAGES lock");
                }
            } else {
                debug!("Failed to upgrade window weak reference");
            }
        }).unwrap();
    });

    // Seeing navigation
    let main_window_weak9 = main_window.as_weak();
    main_window.on_env_canada_seeing_previous(move || {
        debug!("Environment Canada seeing previous button clicked");
        let window_weak = main_window_weak9.clone();
        slint::invoke_from_event_loop(move || {
            debug!("Processing Environment Canada seeing previous in event loop");
            let window = window_weak.upgrade();
            if let Some(window) = window {
                debug!("Window upgraded successfully");
                if let Ok(images) = ENV_CANADA_SEEING_IMAGES.try_lock() {
                    debug!("ENV_CANADA_SEEING_IMAGES lock acquired, {} images", images.len());
                    if let Ok(mut index) = ENV_CANADA_SEEING_INDEX.try_lock() {
                        debug!("ENV_CANADA_SEEING_INDEX lock acquired, current index: {}", *index);
                        if !images.is_empty() {
                            if *index == 0 {
                                *index = images.len() - 1;
                            } else {
                                *index -= 1;
                            }
                            debug!("New index: {}", *index);
                            let current_image_data = &images[*index];
                            let model_run_str = ENV_CANADA_MODEL_RUN_STR.lock().unwrap();
                            let counter_text = calculate_env_canada_forecast_time(&model_run_str, *index, true);
                            match decode_png_to_slint_image(current_image_data) {
                                Ok(slint_image) => {
                                    window.set_env_seeing_image(slint_image);
                                    window.set_env_seeing_counter(counter_text.into());
                                }
                                Err(e) => {
                                    error!("Failed to decode Environment Canada seeing image: {}", e);
                                }
                            }
                        } else {
                            debug!("No Environment Canada seeing images available");
                        }
                    } else {
                        debug!("Failed to acquire ENV_CANADA_SEEING_INDEX lock");
                    }
                } else {
                    debug!("Failed to acquire ENV_CANADA_SEEING_IMAGES lock");
                }
            } else {
                debug!("Failed to upgrade window weak reference");
            }
        }).unwrap();
    });

    let main_window_weak10 = main_window.as_weak();
    main_window.on_env_canada_seeing_next(move || {
        debug!("Environment Canada seeing next button clicked");
        let window_weak = main_window_weak10.clone();
        slint::invoke_from_event_loop(move || {
            debug!("Processing Environment Canada seeing next in event loop");
            let window = window_weak.upgrade();
            if let Some(window) = window {
                debug!("Window upgraded successfully");
                if let Ok(images) = ENV_CANADA_SEEING_IMAGES.try_lock() {
                    debug!("ENV_CANADA_SEEING_IMAGES lock acquired, {} images", images.len());
                    if let Ok(mut index) = ENV_CANADA_SEEING_INDEX.try_lock() {
                        debug!("ENV_CANADA_SEEING_INDEX lock acquired, current index: {}", *index);
                        if !images.is_empty() {
                            *index = (*index + 1) % images.len();
                            debug!("New index: {}", *index);
                            let current_image_data = &images[*index];
                            let model_run_str = ENV_CANADA_MODEL_RUN_STR.lock().unwrap();
                            let counter_text = calculate_env_canada_forecast_time(&model_run_str, *index, true);
                            match decode_png_to_slint_image(current_image_data) {
                                Ok(slint_image) => {
                                    window.set_env_seeing_image(slint_image);
                                    window.set_env_seeing_counter(counter_text.into());
                                }
                                Err(e) => {
                                    error!("Failed to decode Environment Canada seeing image: {}", e);
                                }
                            }
                        } else {
                            debug!("No Environment Canada seeing images available");
                        }
                    } else {
                        debug!("Failed to acquire ENV_CANADA_SEEING_INDEX lock");
                    }
                } else {
                    debug!("Failed to acquire ENV_CANADA_SEEING_IMAGES lock");
                }
            } else {
                debug!("Failed to upgrade window weak reference");
            }
        }).unwrap();
    });

    // Temperature navigation
    let main_window_weak11 = main_window.as_weak();
    main_window.on_env_canada_temperature_previous(move || {
        debug!("Environment Canada temperature previous button clicked");
        let window_weak = main_window_weak11.clone();
        slint::invoke_from_event_loop(move || {
            debug!("Processing Environment Canada temperature previous in event loop");
            let window = window_weak.upgrade();
            if let Some(window) = window {
                debug!("Window upgraded successfully");
                if let Ok(images) = ENV_CANADA_TEMPERATURE_IMAGES.try_lock() {
                    debug!("ENV_CANADA_TEMPERATURE_IMAGES lock acquired, {} images", images.len());
                    if let Ok(mut index) = ENV_CANADA_TEMPERATURE_INDEX.try_lock() {
                        debug!("ENV_CANADA_TEMPERATURE_INDEX lock acquired, current index: {}", *index);
                        if !images.is_empty() {
                            if *index == 0 {
                                *index = images.len() - 1;
                            } else {
                                *index -= 1;
                            }
                            debug!("New index: {}", *index);
                            let current_image_data = &images[*index];
                            let model_run_str = ENV_CANADA_MODEL_RUN_STR.lock().unwrap();
                            let counter_text = calculate_env_canada_forecast_time(&model_run_str, *index, false);
                            match decode_png_to_slint_image(current_image_data) {
                                Ok(slint_image) => {
                                    window.set_env_temperature_image(slint_image);
                                    window.set_env_temperature_counter(counter_text.into());
                                }
                                Err(e) => {
                                    error!("Failed to decode Environment Canada temperature image: {}", e);
                                }
                            }
                        } else {
                            debug!("No Environment Canada temperature images available");
                        }
                    } else {
                        debug!("Failed to acquire ENV_CANADA_TEMPERATURE_INDEX lock");
                    }
                } else {
                    debug!("Failed to acquire ENV_CANADA_TEMPERATURE_IMAGES lock");
                }
            } else {
                debug!("Failed to upgrade window weak reference");
            }
        }).unwrap();
    });

    let main_window_weak12 = main_window.as_weak();
    main_window.on_env_canada_temperature_next(move || {
        debug!("Environment Canada temperature next button clicked");
        let window_weak = main_window_weak12.clone();
        slint::invoke_from_event_loop(move || {
            debug!("Processing Environment Canada temperature next in event loop");
            let window = window_weak.upgrade();
            if let Some(window) = window {
                debug!("Window upgraded successfully");
                if let Ok(images) = ENV_CANADA_TEMPERATURE_IMAGES.try_lock() {
                    debug!("ENV_CANADA_TEMPERATURE_IMAGES lock acquired, {} images", images.len());
                    if let Ok(mut index) = ENV_CANADA_TEMPERATURE_INDEX.try_lock() {
                        debug!("ENV_CANADA_TEMPERATURE_INDEX lock acquired, current index: {}", *index);
                        if !images.is_empty() {
                            *index = (*index + 1) % images.len();
                            debug!("New index: {}", *index);
                            let current_image_data = &images[*index];
                            let model_run_str = ENV_CANADA_MODEL_RUN_STR.lock().unwrap();
                            let counter_text = calculate_env_canada_forecast_time(&model_run_str, *index, false);
                            match decode_png_to_slint_image(current_image_data) {
                                Ok(slint_image) => {
                                    window.set_env_temperature_image(slint_image);
                                    window.set_env_temperature_counter(counter_text.into());
                                }
                                Err(e) => {
                                    error!("Failed to decode Environment Canada temperature image: {}", e);
                                }
                            }
                        } else {
                            debug!("No Environment Canada temperature images available");
                        }
                    } else {
                        debug!("Failed to acquire ENV_CANADA_TEMPERATURE_INDEX lock");
                    }
                } else {
                    debug!("Failed to acquire ENV_CANADA_TEMPERATURE_IMAGES lock");
                }
            } else {
                debug!("Failed to upgrade window weak reference");
            }
        }).unwrap();
    });

    // Transparency navigation
    let main_window_weak13 = main_window.as_weak();
    main_window.on_env_canada_transparency_previous(move || {
        debug!("Environment Canada transparency previous button clicked");
        let window_weak = main_window_weak13.clone();
        slint::invoke_from_event_loop(move || {
            debug!("Processing Environment Canada transparency previous in event loop");
            let window = window_weak.upgrade();
            if let Some(window) = window {
                debug!("Window upgraded successfully");
                if let Ok(images) = ENV_CANADA_TRANSPARENCY_IMAGES.try_lock() {
                    debug!("ENV_CANADA_TRANSPARENCY_IMAGES lock acquired, {} images", images.len());
                    if let Ok(mut index) = ENV_CANADA_TRANSPARENCY_INDEX.try_lock() {
                        debug!("ENV_CANADA_TRANSPARENCY_INDEX lock acquired, current index: {}", *index);
                        if !images.is_empty() {
                            if *index == 0 {
                                *index = images.len() - 1;
                            } else {
                                *index -= 1;
                            }
                            debug!("New index: {}", *index);
                            let current_image_data = &images[*index];
                            let model_run_str = ENV_CANADA_MODEL_RUN_STR.lock().unwrap();
                            let counter_text = calculate_env_canada_forecast_time(&model_run_str, *index, false);
                            match decode_png_to_slint_image(current_image_data) {
                                Ok(slint_image) => {
                                    window.set_env_transparency_image(slint_image);
                                    window.set_env_transparency_counter(counter_text.into());
                                }
                                Err(e) => {
                                    error!("Failed to decode Environment Canada transparency image: {}", e);
                                }
                            }
                        } else {
                            debug!("No Environment Canada transparency images available");
                        }
                    } else {
                        debug!("Failed to acquire ENV_CANADA_TRANSPARENCY_INDEX lock");
                    }
                } else {
                    debug!("Failed to acquire ENV_CANADA_TRANSPARENCY_IMAGES lock");
                }
            } else {
                debug!("Failed to upgrade window weak reference");
            }
        }).unwrap();
    });

    let main_window_weak14 = main_window.as_weak();
    main_window.on_env_canada_transparency_next(move || {
        debug!("Environment Canada transparency next button clicked");
        let window_weak = main_window_weak14.clone();
        slint::invoke_from_event_loop(move || {
            debug!("Processing Environment Canada transparency next in event loop");
            let window = window_weak.upgrade();
            if let Some(window) = window {
                debug!("Window upgraded successfully");
                if let Ok(images) = ENV_CANADA_TRANSPARENCY_IMAGES.try_lock() {
                    debug!("ENV_CANADA_TRANSPARENCY_IMAGES lock acquired, {} images", images.len());
                    if let Ok(mut index) = ENV_CANADA_TRANSPARENCY_INDEX.try_lock() {
                        debug!("ENV_CANADA_TRANSPARENCY_INDEX lock acquired, current index: {}", *index);
                        if !images.is_empty() {
                            *index = (*index + 1) % images.len();
                            debug!("New index: {}", *index);
                            let current_image_data = &images[*index];
                            let model_run_str = ENV_CANADA_MODEL_RUN_STR.lock().unwrap();
                            let counter_text = calculate_env_canada_forecast_time(&model_run_str, *index, false);
                            match decode_png_to_slint_image(current_image_data) {
                                Ok(slint_image) => {
                                    window.set_env_transparency_image(slint_image);
                                    window.set_env_transparency_counter(counter_text.into());
                                }
                                Err(e) => {
                                    error!("Failed to decode Environment Canada transparency image: {}", e);
                                }
                            }
                        } else {
                            debug!("No Environment Canada transparency images available");
                        }
                    } else {
                        debug!("Failed to acquire ENV_CANADA_TRANSPARENCY_INDEX lock");
                    }
                } else {
                    debug!("Failed to acquire ENV_CANADA_TRANSPARENCY_IMAGES lock");
                }
            } else {
                debug!("Failed to upgrade window weak reference");
            }
        }).unwrap();
    });

    // Relative humidity navigation
    let main_window_weak15 = main_window.as_weak();
    main_window.on_env_canada_relative_humidity_previous(move || {
        debug!("Environment Canada relative humidity previous button clicked");
        let window_weak = main_window_weak15.clone();
        slint::invoke_from_event_loop(move || {
            debug!("Processing Environment Canada relative humidity previous in event loop");
            let window = window_weak.upgrade();
            if let Some(window) = window {
                debug!("Window upgraded successfully");
                if let Ok(images) = ENV_CANADA_RELATIVE_HUMIDITY_IMAGES.try_lock() {
                    debug!("ENV_CANADA_RELATIVE_HUMIDITY_IMAGES lock acquired, {} images", images.len());
                    if let Ok(mut index) = ENV_CANADA_RELATIVE_HUMIDITY_INDEX.try_lock() {
                        debug!("ENV_CANADA_RELATIVE_HUMIDITY_INDEX lock acquired, current index: {}", *index);
                        if !images.is_empty() {
                            if *index == 0 {
                                *index = images.len() - 1;
                            } else {
                                *index -= 1;
                            }
                            debug!("New index: {}", *index);
                            let current_image_data = &images[*index];
                            let model_run_str = ENV_CANADA_MODEL_RUN_STR.lock().unwrap();
                            let counter_text = calculate_env_canada_forecast_time(&model_run_str, *index, false);
                            match decode_png_to_slint_image(current_image_data) {
                                Ok(slint_image) => {
                                    window.set_env_relative_humidity_image(slint_image);
                                    window.set_env_relative_humidity_counter(counter_text.into());
                                }
                                Err(e) => {
                                    error!("Failed to decode Environment Canada relative humidity image: {}", e);
                                }
                            }
                        } else {
                            debug!("No Environment Canada relative humidity images available");
                        }
                    } else {
                        debug!("Failed to acquire ENV_CANADA_RELATIVE_HUMIDITY_INDEX lock");
                    }
                } else {
                    debug!("Failed to acquire ENV_CANADA_RELATIVE_HUMIDITY_IMAGES lock");
                }
            } else {
                debug!("Failed to upgrade window weak reference");
            }
        }).unwrap();
    });

    let main_window_weak16 = main_window.as_weak();
    main_window.on_env_canada_relative_humidity_next(move || {
        debug!("Environment Canada relative humidity next button clicked");
        let window_weak = main_window_weak16.clone();
        slint::invoke_from_event_loop(move || {
            debug!("Processing Environment Canada relative humidity next in event loop");
            let window = window_weak.upgrade();
            if let Some(window) = window {
                debug!("Window upgraded successfully");
                if let Ok(images) = ENV_CANADA_RELATIVE_HUMIDITY_IMAGES.try_lock() {
                    debug!("ENV_CANADA_RELATIVE_HUMIDITY_IMAGES lock acquired, {} images", images.len());
                    if let Ok(mut index) = ENV_CANADA_RELATIVE_HUMIDITY_INDEX.try_lock() {
                        debug!("ENV_CANADA_RELATIVE_HUMIDITY_INDEX lock acquired, current index: {}", *index);
                        if !images.is_empty() {
                            *index = (*index + 1) % images.len();
                            debug!("New index: {}", *index);
                            let current_image_data = &images[*index];
                            let model_run_str = ENV_CANADA_MODEL_RUN_STR.lock().unwrap();
                            let counter_text = calculate_env_canada_forecast_time(&model_run_str, *index, false);
                            match decode_png_to_slint_image(current_image_data) {
                                Ok(slint_image) => {
                                    window.set_env_relative_humidity_image(slint_image);
                                    window.set_env_relative_humidity_counter(counter_text.into());
                                }
                                Err(e) => {
                                    error!("Failed to decode Environment Canada relative humidity image: {}", e);
                                }
                            }
                        } else {
                            debug!("No Environment Canada relative humidity images available");
                        }
                    } else {
                        debug!("Failed to acquire ENV_CANADA_RELATIVE_HUMIDITY_INDEX lock");
                    }
                } else {
                    debug!("Failed to acquire ENV_CANADA_RELATIVE_HUMIDITY_IMAGES lock");
                }
            } else {
                debug!("Failed to upgrade window weak reference");
            }
        }).unwrap();
    });
}

pub async fn update_environment_canada_images(main_window: &MainWindow) -> Result<(), Box<dyn std::error::Error>> {
    use environment_canada::{EnvironmentCanadaAPI, ForecastType, Region};
    use chrono::{Utc, Timelike, Duration};

    info!("Updating Environment Canada images...");

    // Calculate latest available model run (accounting for cascading availability)
    // Environment Canada has cascading availability: 06 UTC only available when 12 UTC is available,
    // and 12 UTC only available when 18 UTC is run. Use current_hour - 6 to ensure availability.
    // Special case: if before 06 UTC, use yesterday's 18 UTC run since current day's runs aren't available yet.
    let now = Utc::now();
    let model_run_str = if now.hour() < 6 {
        let yesterday = now - chrono::Duration::days(1);
        format!("{:04}{:02}{:02}18", yesterday.year(), yesterday.month() as u32, yesterday.day() as u32)
    } else {
        let current_hour = now.hour();
        let model_runs = [0, 6, 12, 18];
        let latest_run = model_runs.iter()
            .rev()
            .find(|&&run| run <= current_hour.saturating_sub(6))
            .unwrap_or(&18);
        format!("{:04}{:02}{:02}{:02}", now.year(), now.month() as u32, now.day() as u32, *latest_run)
    };

    println!("Using latest available model run: {} (accounting for cascading availability)", model_run_str);

    // Store model run info for display (convert UTC to local time)
    {
        let mut model_run_info = ENV_CANADA_MODEL_RUN_INFO.lock().unwrap();
        let mut model_run_str_store = ENV_CANADA_MODEL_RUN_STR.lock().unwrap();
        *model_run_str_store = model_run_str.clone();
        if let Ok(utc_hour) = &model_run_str[8..10].parse::<u32>() {
            // Convert UTC hour to local system timezone
            let utc_time = chrono::NaiveTime::from_hms_opt(*utc_hour, 0, 0).unwrap();
            let utc_datetime = chrono::NaiveDateTime::new(now.date_naive(), utc_time);
            let utc_datetime_utc = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(utc_datetime, chrono::Utc);
            let local_datetime = utc_datetime_utc.with_timezone(&chrono::Local);
            let local_hour = local_datetime.hour();
            let am_pm = if local_hour >= 12 { "PM" } else { "AM" };
            let display_hour = if local_hour == 0 { 12 } else if local_hour > 12 { local_hour - 12 } else { local_hour };
            *model_run_info = format!("{} {}", display_hour, am_pm);
        } else {
            *model_run_info = format!("{} UTC", &model_run_str[8..10]);
        }
    }

    // Create API instance
    let api = EnvironmentCanadaAPI::new()?;

    // Fetch multiple hours for each forecast type concurrently
    // Clouds, Surface Wind, Temperature, Transparency, Relative Humidity: hours 1-24
    // Seeing: multiples of 3 (3, 6, 9, ..., 72)

    // Helper function to fetch multiple hours for a forecast type
    async fn fetch_forecast_hours(
        api: &EnvironmentCanadaAPI,
        forecast_type: ForecastType,
        model_run_str: &str,
        region: Region,
        hours: Vec<u32>,
    ) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
        let mut tasks = Vec::new();
        for hour in hours {
            let api_clone = EnvironmentCanadaAPI::new()?;
            let model_run_clone = model_run_str.to_string();
            let task = tokio::spawn(async move {
                match api_clone.fetch_forecast(forecast_type, &model_run_clone, region, hour).await {
                    Ok(data) => Some(data),
                    Err(e) => {
                        eprintln!("Failed to fetch {} for hour {}: {}", forecast_type.name(), hour, e);
                        None
                    }
                }
            });
            tasks.push(task);
        }

        let mut results = Vec::new();
        for task in tasks {
            if let Ok(Some(data)) = task.await {
                results.push(data);
            } else {
                results.push(Vec::new()); // Empty data for failed fetches
            }
        }
        Ok(results)
    }

    // Fetch all forecast types concurrently
    let (
        clouds_images,
        surface_wind_images,
        seeing_images,
        temperature_images,
        transparency_images,
        relative_humidity_images,
    ) = tokio::try_join!(
        fetch_forecast_hours(&api, ForecastType::Cloud, &model_run_str, Region::Northeast, (1..=24).collect()),
        fetch_forecast_hours(&api, ForecastType::SurfaceWind, &model_run_str, Region::Northeast, (1..=24).collect()),
        fetch_forecast_hours(&api, ForecastType::Seeing, &model_run_str, Region::Northeast, (1..=24).map(|h| h * 3).collect()),
        fetch_forecast_hours(&api, ForecastType::Temperature, &model_run_str, Region::Northeast, (1..=24).collect()),
        fetch_forecast_hours(&api, ForecastType::Transparency, &model_run_str, Region::Northeast, (1..=24).collect()),
        fetch_forecast_hours(&api, ForecastType::RelativeHumidity, &model_run_str, Region::Northeast, (1..=24).collect())
    )?;

    // Calculate the current UTC time and set indices to the closest forecast time
    let now = chrono::Utc::now();
    let current_utc_hour = now.hour();

    // Parse model run UTC hour
    let model_run_utc_hour = model_run_str[8..10].parse::<u32>().unwrap_or(0);

    // Calculate hours since model run in UTC
    let hours_since_model_run = if current_utc_hour >= model_run_utc_hour {
        current_utc_hour - model_run_utc_hour
    } else {
        (24 - model_run_utc_hour) + current_utc_hour
    };

    // Set indices to closest available forecast (capped at available images)
    let clouds_index = (hours_since_model_run as usize).min(clouds_images.len().saturating_sub(1));
    let surface_wind_index = (hours_since_model_run as usize).min(surface_wind_images.len().saturating_sub(1));
    let seeing_index = ((hours_since_model_run / 3) as usize).min(seeing_images.len().saturating_sub(1));
    let temperature_index = (hours_since_model_run as usize).min(temperature_images.len().saturating_sub(1));
    let transparency_index = (hours_since_model_run as usize).min(transparency_images.len().saturating_sub(1));
    let relative_humidity_index = (hours_since_model_run as usize).min(relative_humidity_images.len().saturating_sub(1));

    // Update global storage
    {
        let mut clouds_store = ENV_CANADA_CLOUDS_IMAGES.lock().unwrap();
        *clouds_store = clouds_images;
        let mut surface_wind_store = ENV_CANADA_SURFACE_WIND_IMAGES.lock().unwrap();
        *surface_wind_store = surface_wind_images;
        let mut seeing_store = ENV_CANADA_SEEING_IMAGES.lock().unwrap();
        *seeing_store = seeing_images;
        let mut temperature_store = ENV_CANADA_TEMPERATURE_IMAGES.lock().unwrap();
        *temperature_store = temperature_images;
        let mut transparency_store = ENV_CANADA_TRANSPARENCY_IMAGES.lock().unwrap();
        *transparency_store = transparency_images;
        let mut relative_humidity_store = ENV_CANADA_RELATIVE_HUMIDITY_IMAGES.lock().unwrap();
        *relative_humidity_store = relative_humidity_images;
    }

    // Set indices
    {
        *ENV_CANADA_CLOUDS_INDEX.lock().unwrap() = clouds_index;
        *ENV_CANADA_SURFACE_WIND_INDEX.lock().unwrap() = surface_wind_index;
        *ENV_CANADA_SEEING_INDEX.lock().unwrap() = seeing_index;
        *ENV_CANADA_TEMPERATURE_INDEX.lock().unwrap() = temperature_index;
        *ENV_CANADA_TRANSPARENCY_INDEX.lock().unwrap() = transparency_index;
        *ENV_CANADA_RELATIVE_HUMIDITY_INDEX.lock().unwrap() = relative_humidity_index;
    }

    // Update UI with first images
    update_env_canada_clouds_display(main_window);
    update_env_canada_surface_wind_display(main_window);
    update_env_canada_seeing_display(main_window);
    update_env_canada_temperature_display(main_window);
    update_env_canada_transparency_display(main_window);
    update_env_canada_relative_humidity_display(main_window);

    // Set model run info for UI
    {
        let model_run_info = ENV_CANADA_MODEL_RUN_INFO.lock().unwrap();
        main_window.set_env_model_run_info(model_run_info.clone().into());
    }

    // Load initial Nina images
    if let Err(e) = crate::app::nina::update_nina_images(&main_window).await {
        eprintln!("Failed to load Nina images: {}", e);
        main_window.set_error_message(format!("Failed to load Nina images: {}", e).into());
    }

    println!("Environment Canada images updated successfully");
    Ok(())
}

pub fn update_env_canada_clouds_display(main_window: &MainWindow) {
    let images = ENV_CANADA_CLOUDS_IMAGES.lock().unwrap();
    let index = ENV_CANADA_CLOUDS_INDEX.lock().unwrap();

    if !images.is_empty() && *index < images.len() && !images[*index].is_empty() {
        let current_image_data = &images[*index];
        let model_run_str = ENV_CANADA_MODEL_RUN_STR.lock().unwrap();
        let counter_text = calculate_env_canada_forecast_time(&model_run_str, *index, false);
        debug!("Displaying Environment Canada clouds image (index: {})", *index);

        match decode_png_to_slint_image(current_image_data) {
            Ok(slint_image) => {
                main_window.set_env_clouds_image(slint_image);
                main_window.set_env_clouds_counter(counter_text.into());
            }
            Err(e) => {
                error!("Failed to decode Environment Canada clouds image: {}", e);
            }
        }
    } else {
        debug!("No Environment Canada clouds images available for display ({} images, index {})", images.len(), *index);
    }
}

pub fn update_env_canada_surface_wind_display(main_window: &MainWindow) {
    let images = ENV_CANADA_SURFACE_WIND_IMAGES.lock().unwrap();
    let index = ENV_CANADA_SURFACE_WIND_INDEX.lock().unwrap();

    if !images.is_empty() && *index < images.len() && !images[*index].is_empty() {
        let current_image_data = &images[*index];
        let model_run_str = ENV_CANADA_MODEL_RUN_STR.lock().unwrap();
        let counter_text = calculate_env_canada_forecast_time(&model_run_str, *index, false);
        debug!("Displaying Environment Canada surface wind image (index: {})", *index);

        match decode_png_to_slint_image(current_image_data) {
            Ok(slint_image) => {
                main_window.set_env_surface_wind_image(slint_image);
                main_window.set_env_surface_wind_counter(counter_text.into());
            }
            Err(e) => {
                error!("Failed to decode Environment Canada surface wind image: {}", e);
            }
        }
    } else {
        debug!("No Environment Canada surface wind images available for display ({} images, index {})", images.len(), *index);
    }
}

pub fn update_env_canada_seeing_display(main_window: &MainWindow) {
    let images = ENV_CANADA_SEEING_IMAGES.lock().unwrap();
    let index = ENV_CANADA_SEEING_INDEX.lock().unwrap();

    if !images.is_empty() && *index < images.len() && !images[*index].is_empty() {
        let current_image_data = &images[*index];
        let model_run_str = ENV_CANADA_MODEL_RUN_STR.lock().unwrap();
        let counter_text = calculate_env_canada_forecast_time(&model_run_str, *index, true);
        debug!("Displaying Environment Canada seeing image (index: {})", *index);

        match decode_png_to_slint_image(current_image_data) {
            Ok(slint_image) => {
                main_window.set_env_seeing_image(slint_image);
                main_window.set_env_seeing_counter(counter_text.into());
            }
            Err(e) => {
                error!("Failed to decode Environment Canada seeing image: {}", e);
            }
        }
    } else {
        debug!("No Environment Canada seeing images available for display ({} images, index {})", images.len(), *index);
    }
}

pub fn update_env_canada_temperature_display(main_window: &MainWindow) {
    let images = ENV_CANADA_TEMPERATURE_IMAGES.lock().unwrap();
    let index = ENV_CANADA_TEMPERATURE_INDEX.lock().unwrap();

    if !images.is_empty() && *index < images.len() && !images[*index].is_empty() {
        let current_image_data = &images[*index];
        let model_run_str = ENV_CANADA_MODEL_RUN_STR.lock().unwrap();
        let counter_text = calculate_env_canada_forecast_time(&model_run_str, *index, false);
        debug!("Displaying Environment Canada temperature image (index: {})", *index);

        match decode_png_to_slint_image(current_image_data) {
            Ok(slint_image) => {
                main_window.set_env_temperature_image(slint_image);
                main_window.set_env_temperature_counter(counter_text.into());
            }
            Err(e) => {
                error!("Failed to decode Environment Canada temperature image: {}", e);
            }
        }
    } else {
        debug!("No Environment Canada temperature images available for display ({} images, index {})", images.len(), *index);
    }
}

pub fn update_env_canada_transparency_display(main_window: &MainWindow) {
    let images = ENV_CANADA_TRANSPARENCY_IMAGES.lock().unwrap();
    let index = ENV_CANADA_TRANSPARENCY_INDEX.lock().unwrap();

    if !images.is_empty() && *index < images.len() && !images[*index].is_empty() {
        let current_image_data = &images[*index];
        let model_run_str = ENV_CANADA_MODEL_RUN_STR.lock().unwrap();
        let counter_text = calculate_env_canada_forecast_time(&model_run_str, *index, false);
        debug!("Displaying Environment Canada transparency image (index: {})", *index);

        match decode_png_to_slint_image(current_image_data) {
            Ok(slint_image) => {
                main_window.set_env_transparency_image(slint_image);
                main_window.set_env_transparency_counter(counter_text.into());
            }
            Err(e) => {
                error!("Failed to decode Environment Canada transparency image: {}", e);
            }
        }
    } else {
        debug!("No Environment Canada transparency images available for display ({} images, index {})", images.len(), *index);
    }
}

pub fn update_env_canada_relative_humidity_display(main_window: &MainWindow) {
    let images = ENV_CANADA_RELATIVE_HUMIDITY_IMAGES.lock().unwrap();
    let index = ENV_CANADA_RELATIVE_HUMIDITY_INDEX.lock().unwrap();

    if !images.is_empty() && *index < images.len() && !images[*index].is_empty() {
        let current_image_data = &images[*index];
        let model_run_str = ENV_CANADA_MODEL_RUN_STR.lock().unwrap();
        let counter_text = calculate_env_canada_forecast_time(&model_run_str, *index, false);
        debug!("Displaying Environment Canada relative humidity image (index: {})", *index);

        match decode_png_to_slint_image(current_image_data) {
            Ok(slint_image) => {
                main_window.set_env_relative_humidity_image(slint_image);
                main_window.set_env_relative_humidity_counter(counter_text.into());
            }
            Err(e) => {
                error!("Failed to decode Environment Canada relative humidity image: {}", e);
            }
        }
    } else {
        debug!("No Environment Canada relative humidity images available for display ({} images, index {})", images.len(), *index);
    }
}
