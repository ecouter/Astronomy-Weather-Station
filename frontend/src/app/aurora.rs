use std::sync::Mutex;
use std::error::Error;
use once_cell::sync::Lazy;
use slint::ComponentHandle;
use crate::MainWindow;
use crate::app::refresh::spawn;
use crate::app::utils::{decode_png_to_slint_image, resize_png_if_needed};
use crate::app::coordinates::load_coordinates;
use chrono::{Duration, Utc};

pub fn setup_aurora_callbacks(main_window: &MainWindow) {
    // Refresh callback
    let main_window_weak_refresh = main_window.as_weak();
    main_window.on_aurora_refresh(move || {
        let window_weak = main_window_weak_refresh.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(window) = window_weak.upgrade() {
                // Disable button and set text
                window.set_aurora_refresh_button_enabled(false);
                window.set_aurora_refresh_button_text("Refreshing...".into());

                // Start countdown timer immediately
                start_refresh_countdown(&window);

                // Load coordinates in the main thread where window is available
                let (lat, lon) = if let Ok(coords) = load_coordinates(&window) {
                    info!("Aurora refresh: coordinates loaded successfully: lat={}, lon={}", coords.0, coords.1);
                    coords
                } else {
                    error!("Aurora refresh: could not load coordinates, using defaults");
                    (45.0, -75.0)
                };

                // Clone weak for async use in background thread
                let weak_clone = window.as_weak();

                // Fetch on the shared runtime; the UI is never blocked.
                spawn(async move {
                    if let Err(e) = update_aurora_images(weak_clone, lat, lon).await {
                        error!("Aurora refresh failed: {}", e);
                    }
                });
            }
        }).unwrap();
    });
}

async fn fetch_and_update_single_image(
    main_window_weak: slint::Weak<MainWindow>,
    fetch_result: Result<Vec<u8>, anyhow::Error>,
    setter: fn(&MainWindow, Option<slint::Image>, bool),
    name: &str,
) {
    let name = name.to_string(); // Clone for move
    match fetch_result {
        Ok(image_data) => {
            info!("Successfully fetched: {}", name);

            // Resize image if needed
            let resized_data = match resize_png_if_needed(image_data, 1920) {
                Ok(resized) => {
                    info!("Successfully resized {} image", name);
                    resized
                },
                Err(e) => {
                    error!("Failed to resize {} image: {}, using original", name, e);
                    Vec::new() // Use empty to indicate error
                }
            };

            slint::invoke_from_event_loop(move || {
                if let Some(window) = main_window_weak.upgrade() {
                    if resized_data.is_empty() {
                        setter(&window, None, true);
                    } else {
                        match decode_png_to_slint_image(&resized_data) {
                            Ok(slint_image) => {
                                setter(&window, Some(slint_image), false);
                            }
                            Err(e) => {
                                error!("Failed to decode {} image: {}", name, e);
                                setter(&window, None, true);
                            }
                        }
                    }
                }
            }).unwrap();
        }
        Err(e) => {
            error!("Failed to fetch {}: {}", name, e);
            let name_copy = name.clone();
            slint::invoke_from_event_loop(move || {
                if let Some(window) = main_window_weak.upgrade() {
                    setter(&window, None, true);
                }
            }).unwrap();
        }
    }
}

pub async fn update_aurora_images(main_window_weak: slint::Weak<MainWindow>, lat: f64, lon: f64) -> Result<(), Box<dyn std::error::Error>> {
    use aurora::{fetch_aurora_forecast, fetch_ace_real_time_solar_wind, fetch_dscovr_solar_wind,
                     fetch_space_weather_overview, fetch_ace_epam, fetch_canadian_magnetic,
                     fetch_alerts_timeline, fetch_tonights_aurora_forecast,
                     fetch_tomorrow_aurora_forecast, fetch_wsa_enlil, fetch_nasa_viirs};

    info!("Starting Aurora images update with coordinates: lat={}, lon={}", lat, lon);

    // Create clones for each async block to avoid borrow checker issues
    let weak1 = main_window_weak.clone();
    let weak2 = main_window_weak.clone();
    let weak3 = main_window_weak.clone();
    let weak4 = main_window_weak.clone();
    let weak5 = main_window_weak.clone();
    let weak6 = main_window_weak.clone();
    let weak7 = main_window_weak.clone();
    let weak8 = main_window_weak.clone();
    let weak9 = main_window_weak.clone();
    let weak10 = main_window_weak.clone();
    let weak11 = main_window_weak.clone();
    let weak12 = main_window_weak.clone();

    // Run all fetches concurrently using tokio::join! instead of spawn
    tokio::join!(
        async {
            let result = fetch_aurora_forecast().await;
            fetch_and_update_single_image(weak1, result, |mw, img, err| {
                if let Some(img) = img {
                    mw.set_aurora_forecast_image(img);
                }
                mw.set_aurora_forecast_error(err);
            }, "Aurora Forecast").await;
        },

        async {
            let result = fetch_ace_real_time_solar_wind().await;
            fetch_and_update_single_image(weak2, result, |mw, img, err| {
                if let Some(img) = img {
                    mw.set_aurora_ace_solar_wind_image(img);
                }
                mw.set_aurora_ace_solar_wind_error(err);
            }, "ACE Solar Wind").await;
        },

        async {
            let result = fetch_dscovr_solar_wind().await;
            fetch_and_update_single_image(weak3, result, |mw, img, err| {
                if let Some(img) = img {
                    mw.set_aurora_dscovr_solar_wind_image(img);
                }
                mw.set_aurora_dscovr_solar_wind_error(err);
            }, "DSCOVR Solar Wind").await;
        },

        async {
            let result = fetch_space_weather_overview().await;
            fetch_and_update_single_image(weak4, result, |mw, img, err| {
                if let Some(img) = img {
                    mw.set_aurora_space_weather_overview_image(img);
                }
                mw.set_aurora_space_weather_overview_error(err);
            }, "Space Weather Overview").await;
        },

        async {
            let result = fetch_ace_epam().await;
            fetch_and_update_single_image(weak5, result, |mw, img, err| {
                if let Some(img) = img {
                    mw.set_aurora_ace_epam_image(img);
                }
                mw.set_aurora_ace_epam_error(err);
            }, "ACE EPAM").await;
        },

        async {
            let result = fetch_canadian_magnetic().await;
            fetch_and_update_single_image(weak6, result, |mw, img, err| {
                if let Some(img) = img {
                    mw.set_aurora_canadian_magnetic_image(img);
                }
                mw.set_aurora_canadian_magnetic_error(err);
            }, "Canadian Magnetic").await;
        },

        async {
            let result = fetch_alerts_timeline().await;
            fetch_and_update_single_image(weak7, result, |mw, img, err| {
                if let Some(img) = img {
                    mw.set_aurora_alerts_timeline_image(img);
                }
                mw.set_aurora_alerts_timeline_error(err);
            }, "Alerts Timeline").await;
        },

        async {
            let result = fetch_tonights_aurora_forecast().await;
            fetch_and_update_single_image(weak9, result, |mw, img, err| {
                if let Some(img) = img {
                    mw.set_aurora_tonights_forecast_image(img);
                }
                mw.set_aurora_tonights_forecast_error(err);
            }, "Tonight's Aurora Forecast").await;
        },

        async {
            let result = fetch_tomorrow_aurora_forecast().await;
            fetch_and_update_single_image(weak10, result, |mw, img, err| {
                if let Some(img) = img {
                    mw.set_aurora_tomorrow_forecast_image(img);
                }
                mw.set_aurora_tomorrow_forecast_error(err);
            }, "Tomorrow's Aurora Forecast").await;
        },

        async {
            let result = fetch_wsa_enlil().await;
            fetch_and_update_single_image(weak11, result, |mw, img, err| {
                if let Some(img) = img {
                    mw.set_aurora_wsa_enlil_image(img);
                }
                mw.set_aurora_wsa_enlil_error(err);
            }, "WSA-ENLIL Prediction").await;
        },

        async {
            // Try today's date first, fallback to yesterday if it fails
            let today = Utc::now();
            let today_str = today.format("%Y-%m-%d").to_string();
            let yesterday = today - Duration::days(1);
            let yesterday_str = yesterday.format("%Y-%m-%d").to_string();

            let result = match fetch_nasa_viirs(lat, lon, today_str).await {
                Ok(data) => {
                    info!("Successfully fetched NASA VIIRS data for today");
                    Ok(data)
                },
                Err(e) => {
                    warn!("Failed to fetch NASA VIIRS data for today ({}), trying yesterday", e);
                    match fetch_nasa_viirs(lat, lon, yesterday_str).await {
                        Ok(data) => {
                            info!("Successfully fetched NASA VIIRS data for yesterday (fallback)");
                            Ok(data)
                        },
                        Err(e2) => {
                            error!("Failed to fetch NASA VIIRS data for both today and yesterday: today={}, yesterday={}", e, e2);
                            Err(e2)
                        }
                    }
                }
            };

            fetch_and_update_single_image(weak12, result, |mw, img, err| {
                if let Some(img) = img {
                    mw.set_aurora_nasa_viirs_image(img);
                }
                mw.set_aurora_nasa_viirs_error(err);
            }, "NASA VIIRS Nighttime").await;
        },
    );

    info!("All Aurora images updated successfully");
    Ok(())
}

fn start_refresh_countdown(main_window: &MainWindow) {
    let window_weak = main_window.as_weak();

    spawn(async move {
        let mut remaining_seconds = 300; // 5 minutes

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            remaining_seconds -= 1;

            if remaining_seconds <= 0 {
                window_weak
                    .upgrade_in_event_loop(|window| {
                        window.set_aurora_refresh_button_enabled(true);
                        window.set_aurora_refresh_button_text("Refresh".into());
                    })
                    .ok();
                break;
            }

            let minutes = remaining_seconds / 60;
            let seconds = remaining_seconds % 60;
            let countdown_text = format!("Refresh in {:02}:{:02}", minutes, seconds);
            let w = window_weak.clone();
            w.upgrade_in_event_loop(move |window| {
                window.set_aurora_refresh_button_text(countdown_text.into());
            })
            .ok();
        }
    });
}
