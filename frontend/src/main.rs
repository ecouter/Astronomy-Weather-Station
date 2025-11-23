mod app;

slint::include_modules!();



extern crate pretty_env_logger;
#[macro_use] extern crate log;

use std::time::Duration;
use std::thread;
use std::sync::mpsc;

#[derive(Clone)]
enum InitialDataMessage {
    Weather(app::weather::WeatherData),
    CloudCover(Vec<Vec<u8>>),
    Wind((Vec<Vec<u8>>, Vec<u8>)),
    ClearOutside(app::clearoutside::ClearOutsideData),
    MeteoBlue(Vec<app::meteoblue::MeteoBlueNightData>),
    Map(Vec<u8>),
    ClearDarkSky(Vec<u8>),
    Sounding(Vec<u8>),
    Aurora,
}

fn main() -> Result<(), slint::PlatformError> {
    pretty_env_logger::init();

    info!("Starting weather station frontend...");

    let main_window = MainWindow::new()?;

    // Load coordinates
    let (lat, lon) = match app::coordinates::load_coordinates(&main_window) {
        Ok(coords) => coords,
        Err(e) => {
            error!("Failed to load coordinates: {}", e);
            main_window.set_error_message(format!("Failed to load coordinates: {}", e).into());
            return Err(slint::PlatformError::Other("Failed to load coordinates".into()));
        }
    };

    // Set up callback handlers using the modular functions
    app::cloud_cover::setup_cloud_cover_callbacks(&main_window);
    app::wind::setup_wind_callbacks(&main_window);
    app::environment_canada::setup_environment_canada_callbacks(&main_window);
    app::sounding::setup_sounding_callbacks(&main_window);
    app::precipitation::setup_precipitation_callbacks(&main_window);
    app::aurora::setup_aurora_callbacks(&main_window);

    // Set up NINA URL change callback handler
    let main_window_weak_nina = main_window.as_weak();
    main_window.on_nina_url_saved(move |index| {
        let window_weak = main_window_weak_nina.clone();
        let url_index = index as usize;

        // Use invoke_from_event_loop to run async code in the UI thread
        slint::invoke_from_event_loop(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let window = window_weak.upgrade();
            if let Some(window) = window {
                let url = match url_index {
                    0 => window.get_nina_url1().to_string(),
                    1 => window.get_nina_url2().to_string(),
                    2 => window.get_nina_url3().to_string(),
                    3 => window.get_nina_url4().to_string(),
                    4 => window.get_nina_url5().to_string(),
                    5 => window.get_nina_url6().to_string(),
                    _ => return,
                };
                info!("NINA URL change callback: slot {} -> '{}'", url_index + 1, url);

                // Clear any existing error state first
                app::nina::clear_nina_error_state(&window, url_index);

                rt.block_on(async {
                    if let Err(e) = app::nina::handle_nina_url_change(url_index, url, &window).await {
                        error!("Failed to handle NINA URL change for slot {}: {}", url_index + 1, e);
                        app::nina::set_nina_error_state(&window, url_index, "Connection could not be established");
                    } else {
                        info!("Successfully handled NINA URL change for slot {}", url_index + 1);
                    }
                });
            }
        }).unwrap();
    });

    main_window.set_loading(false); // Show UI immediately, loading happens asynchronously

    // Channel for initial data loading
    let (initial_tx, initial_rx) = mpsc::channel();

    // Spawn initial data loading tasks asynchronously
    {
        let initial_tx_clone = initial_tx.clone();
        let lat_clone = lat;
        let lon_clone = lon;
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if let Ok(data) = app::weather::fetch_weather_images(lat_clone, lon_clone).await {
                    let _ = initial_tx_clone.send(InitialDataMessage::Weather(data));
                }
            });
        });
    }

    {
        let initial_tx_clone = initial_tx.clone();
        let lat_clone = lat;
        let lon_clone = lon;
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if let Ok(images) = app::cloud_cover::fetch_cloud_cover_images(lat_clone, lon_clone).await {
                    let _ = initial_tx_clone.send(InitialDataMessage::CloudCover(images));
                }
            });
        });
    }

    {
        let initial_tx_clone = initial_tx.clone();
        let lat_clone = lat;
        let lon_clone = lon;
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if let Ok((images, legend)) = app::wind::fetch_wind_images(lat_clone, lon_clone).await {
                    let _ = initial_tx_clone.send(InitialDataMessage::Wind((images, legend)));
                }
            });
        });
    }

    {
        let initial_tx_clone = initial_tx.clone();
        let lat_clone = lat;
        let lon_clone = lon;
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if let Ok(data) = app::clearoutside::fetch_clearoutside_data(lat_clone, lon_clone).await {
                    let _ = initial_tx_clone.send(InitialDataMessage::ClearOutside(data.clone()));
                    // Now fetch meteoblue data using the clearoutside forecast
                    // We need to create a clearoutside::ClearOutsideForecast from our data
                    // For simplicity, let's fetch the raw clearoutside forecast
                    use clearoutside::ClearOutsideAPI;
                    let lat_str = format!("{:.2}", lat_clone);
                    let lon_str = format!("{:.2}", lon_clone);
                    if let Ok(api) = ClearOutsideAPI::new(&lat_str, &lon_str, Some("midnight")).await {
                        if let Ok(forecast) = api.pull() {
                            if let Ok(meteoblue_data) = app::meteoblue::fetch_meteoblue_data(lat_clone, lon_clone, &forecast).await {
                                let _ = initial_tx_clone.send(InitialDataMessage::MeteoBlue(meteoblue_data));
                            }
                        }
                    }
                }
            });
        });
    }

    {
        let initial_tx_clone = initial_tx.clone();
        let lat_clone = lat;
        let lon_clone = lon;
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if let Ok(image) = app::map::fetch_map_image(lat_clone, lon_clone).await {
                    let _ = initial_tx_clone.send(InitialDataMessage::Map(image));
                }
            });
        });
    }

    {
        let initial_tx_clone = initial_tx.clone();
        let lat_clone = lat;
        let lon_clone = lon;
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if let Ok(image) = app::cleardarksky::fetch_cleardarksky_image(lat_clone, lon_clone).await {
                    let _ = initial_tx_clone.send(InitialDataMessage::ClearDarkSky(image));
                }
            });
        });
    }

    {
        let initial_tx_clone = initial_tx.clone();
        let lat_clone = lat;
        let lon_clone = lon;
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if let Ok(image) = app::sounding::fetch_sounding_image(lat_clone, lon_clone).await {
                    let _ = initial_tx_clone.send(InitialDataMessage::Sounding(image));
                }
            });
        });
    }

    // Load initial Environment Canada images
    {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = app::environment_canada::update_environment_canada_images(&main_window).await {
                error!("Failed to load Environment Canada images: {}", e);
            }
        });
    }

    // Initial precipitation data load (non-blocking for UI)
    {
        let main_window_weak = main_window.as_weak();
        let lat_p = lat;
        let lon_p = lon;
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                if let Err(e) =
                    app::precipitation::fetch_initial_precipitation(lat_p, lon_p).await
                {
                    error!("Failed to fetch initial precipitation data: {}", e);
                }
                if let Some(window) = main_window_weak.upgrade() {
                    // Update the precipitation view with whatever was loaded
                    slint::invoke_from_event_loop(move || {
                        if let Some(w) = main_window_weak.upgrade() {
                            app::precipitation::update_precipitation_display(&w);
                        }
                    })
                    .ok();
                }
            });
        });
    }

    // Channel for communication between background thread and UI thread
    let (tx, rx) = mpsc::channel();
    // Channel for cloud cover updates (not cycling)
    let (cloud_tx, cloud_rx) = mpsc::channel();
    // Channel for wind updates (not cycling)
    let (wind_tx, wind_rx) = mpsc::channel();

    // Spawn background thread for periodic weather updates
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut interval = tokio::time::interval(Duration::from_secs(600)); // 10 minutes
            interval.tick().await; // Skip immediate first trigger
            loop {
                interval.tick().await;
                // Signal the UI thread to update images
                if tx.send(()).is_err() {
                    // UI thread has shut down
                    break;
                }
            }
        });
    });

    // Spawn background thread for cloud cover updates (hourly)
    let cloud_tx_clone = cloud_tx.clone();
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // 1 hour
            interval.tick().await; // Skip immediate first trigger
            loop {
                interval.tick().await;
                // Signal the UI thread to update cloud cover images
                if cloud_tx_clone.send("update").is_err() {
                    // UI thread has shut down
                    break;
                }
            }
        });
    });

    // Spawn background thread for ClearOutside data updates (hourly)
    let clearoutside_tx = cloud_tx.clone();
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // 1 hour
            interval.tick().await; // Skip immediate first trigger
            loop {
                interval.tick().await;
                // Signal the UI thread to update ClearOutside data
                if clearoutside_tx.send("clearoutside_update").is_err() {
                    // UI thread has shut down
                    break;
                }
            }
        });
    });

    // Spawn background thread for wind updates (hourly)
    let wind_tx_clone = wind_tx.clone();
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // 1 hour
            interval.tick().await; // Skip immediate first trigger
            loop {
                interval.tick().await;
                // Signal the UI thread to update wind images
                if wind_tx_clone.send("update").is_err() {
                    // UI thread has shut down
                    break;
                }
            }
        });
    });
// Spawn background thread for Environment Canada updates (hourly)
let env_canada_tx = cloud_tx.clone(); // Reuse channel for simplicity
thread::spawn(move || {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut interval = tokio::time::interval(Duration::from_secs(3600)); // 1 hour
        interval.tick().await; // Skip immediate first trigger
        loop {
            interval.tick().await;
            // Signal the UI thread to update Environment Canada images
            if env_canada_tx.send("env_canada_update").is_err() {
                // UI thread has shut down
                break;
            }
        }
    });
});

// Spawn background thread for precipitation updates
{
    let main_window_weak = main_window.as_weak();
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Radar every 30 minutes, models every 60 minutes (handled inside refresh)
            let mut interval = tokio::time::interval(Duration::from_secs(60 * 30));
            interval.tick().await; // skip immediate
            loop {
                interval.tick().await;
                if let Some(window) = main_window_weak.upgrade() {
                    // Reload coordinates in case they changed
                    if let Ok((lat, lon)) = app::coordinates::load_coordinates(&window) {
                        if let Err(e) = app::precipitation::refresh_precipitation_data(lat, lon, &window).await {
                            error!("Failed to refresh precipitation data: {}", e);
                        }
                    } else {
                        error!("Unable to reload coordinates for precipitation refresh");
                    }
                } else {
                    break; // UI closed
                }
            }
        });
    });
}



    // Handle cloud cover updates directly in the main thread using invoke_from_event_loop
    let main_window_weak = main_window.as_weak();
    let lat3 = lat;
    let lon3 = lon;
    let _cloud_update_handle = thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        info!("Cloud cover update thread started");
        while let Ok(signal) = cloud_rx.recv() {
            info!("Received cloud signal: {}", signal);
            let window_weak = main_window_weak.clone();
            let lat = lat3;
            let lon = lon3;
            match signal {
                "update" => {
                    info!("Processing cloud update signal");
                    // Use invoke_from_event_loop to run async code in the UI thread
                    slint::invoke_from_event_loop(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let window = window_weak.upgrade();
                        if let Some(window) = window {
                            rt.block_on(async {
                                if let Ok(images) = app::cloud_cover::fetch_cloud_cover_images(lat, lon).await {
                                    app::cloud_cover::set_cloud_cover_images(&window, images);
                                } else {
                                    error!("Failed to update cloud cover images");
                                    window.set_error_message("Failed to update cloud cover images".into());
                                }
                                // Also update ClearDarkSky chart
                                if let Ok(image) = app::cleardarksky::fetch_cleardarksky_image(lat, lon).await {
                                    app::cleardarksky::set_cleardarksky_image(&window, image);
                                    info!("Updated ClearDarkSky chart");
                                } else {
                                    error!("Failed to update ClearDarkSky image");
                                    window.set_error_message("Failed to update ClearDarkSky image".into());
                                }
                            });
                        }
                    }).unwrap();
                }
                "clearoutside_update" => {
                    info!("Processing ClearOutside update signal");
                    // Use invoke_from_event_loop to run async code in the UI thread
                    slint::invoke_from_event_loop(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let window = window_weak.upgrade();
                        if let Some(window) = window {
                            rt.block_on(async {
                                if let Ok(data) = app::clearoutside::fetch_clearoutside_data(lat, lon).await {
                                    app::clearoutside::set_clearoutside_data(&window, data.clone());
                                    // Also update MeteoBlue data using the clearoutside forecast
                                    use clearoutside::ClearOutsideAPI;
                                    let lat_str = format!("{:.2}", lat);
                                    let lon_str = format!("{:.2}", lon);
                                    if let Ok(api) = ClearOutsideAPI::new(&lat_str, &lon_str, Some("midnight")).await {
                                        if let Ok(forecast) = api.pull() {
                                            if let Ok(meteoblue_data) = app::meteoblue::fetch_meteoblue_data(lat, lon, &forecast).await {
                                                app::meteoblue::set_meteoblue_data(&window, meteoblue_data);
                                                info!("Updated MeteoBlue data");
                                            } else {
                                                error!("Failed to update MeteoBlue data");
                                            }
                                        }
                                    }
                                } else {
                                    error!("Failed to update ClearOutside data");
                                    window.set_error_message("Failed to update ClearOutside data".into());
                                }
                            });
                        }
                    }).unwrap();
                }
                "env_canada_update" => {
                    info!("Processing Environment Canada update signal");
                    // Use invoke_from_event_loop to run async code in the UI thread
                    slint::invoke_from_event_loop(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let window = window_weak.upgrade();
                        if let Some(window) = window {
                            rt.block_on(async {
                                if let Err(e) = app::environment_canada::update_environment_canada_images(&window).await {
                                    error!("Failed to update Environment Canada images: {}", e);
                                }
                            });
                        }
                    }).unwrap();
                }

                "cycle" => {
                    info!("Processing cloud cycle signal");
                    // Use invoke_from_event_loop to update UI in the UI thread
                    slint::invoke_from_event_loop(move || {
                        let window = window_weak.upgrade();
                        if let Some(window) = window {
                            app::cloud_cover::update_cloud_cover_display(&window);
                        }
                    }).unwrap();
                }
                _ => {
                    info!("Unknown cloud signal: {}", signal);
                }
            }
        }
        info!("Cloud cover update thread ended");
    });

    // Handle wind updates directly in the main thread using invoke_from_event_loop
    let main_window_weak2 = main_window.as_weak();
    let lat4 = lat;
    let lon4 = lon;
    let _wind_update_handle = thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        info!("Wind update thread started");
        while let Ok(signal) = wind_rx.recv() {
            info!("Received wind signal: {}", signal);
            let window_weak = main_window_weak2.clone();
            let lat = lat4;
            let lon = lon4;
            match signal {
                "update" => {
                    info!("Processing wind update signal");
                    // Use invoke_from_event_loop to run async code in the UI thread
                    slint::invoke_from_event_loop(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let window = window_weak.upgrade();
                        if let Some(window) = window {
                            rt.block_on(async {
                                if let Ok((images, legend)) = app::wind::fetch_wind_images(lat, lon).await {
                                    app::wind::set_wind_images(&window, (images, legend));
                                } else {
                                    error!("Failed to update wind images");
                                    window.set_error_message("Failed to update wind images".into());
                                }
                            });
                        }
                    }).unwrap();
                }
                "cycle" => {
                    info!("Processing wind cycle signal");
                    // Use invoke_from_event_loop to update UI in the UI thread
                    slint::invoke_from_event_loop(move || {
                        let window = window_weak.upgrade();
                        if let Some(window) = window {
                            app::wind::update_wind_display(&window);
                        }
                    }).unwrap();
                }
                _ => {
                    info!("Unknown wind signal: {}", signal);
                }
            }
        }
        info!("Wind update thread ended");
    });

    // Handle initial data loading in the UI thread
    let main_window_weak_initial = main_window.as_weak();
    let _initial_data_handle = thread::spawn(move || {
        info!("Initial data loading thread started");
        while let Ok(message) = initial_rx.recv() {
            let window_weak = main_window_weak_initial.clone();
            slint::invoke_from_event_loop(move || {
                let window = window_weak.upgrade();
                if let Some(window) = window {
                    match message {
                        InitialDataMessage::Weather(data) => {
                            app::weather::set_weather_images(&window, data);
                            info!("Initial weather data loaded");
                        }
                        InitialDataMessage::CloudCover(images) => {
                            app::cloud_cover::set_cloud_cover_images(&window, images);
                            info!("Initial cloud cover data loaded");
                        }
                        InitialDataMessage::Wind((images, legend)) => {
                            app::wind::set_wind_images(&window, (images, legend));
                            info!("Initial wind data loaded");
                        }
                        InitialDataMessage::ClearOutside(data) => {
                            app::clearoutside::set_clearoutside_data(&window, data);
                            info!("Initial ClearOutside data loaded");
                        }
                        InitialDataMessage::MeteoBlue(data) => {
                            app::meteoblue::set_meteoblue_data(&window, data);
                            info!("Initial MeteoBlue data loaded");
                        }
                        InitialDataMessage::Map(image) => {
                            app::map::set_map_image(&window, image);
                            info!("Initial map data loaded");
                        }
                        InitialDataMessage::ClearDarkSky(image) => {
                            app::cleardarksky::set_cleardarksky_image(&window, image);
                            info!("Initial ClearDarkSky data loaded");
                        }
                        InitialDataMessage::Sounding(image) => {
                            app::sounding::set_sounding_image(&window, image);
                            info!("Initial sounding data loaded");
                        }
                        InitialDataMessage::Aurora => {
                            info!("Initial Aurora data loaded");
                        }
                    }
                }
            }).unwrap();
        }
        info!("Initial data loading thread ended");
    });

    // Keep the main window alive by storing it
    let _main_window_handle = main_window.as_weak();

    // Handle weather updates in the UI thread
    let main_window_weak2 = main_window.as_weak();
    let lat2 = lat;
    let lon2 = lon;
    let _weather_update_handle = thread::spawn(move || {
        while let Ok(()) = rx.recv() {
            let window_weak = main_window_weak2.clone();
            let lat = lat2;
            let lon = lon2;
            // Use invoke_from_event_loop to run async code in the UI thread
            slint::invoke_from_event_loop(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let window = window_weak.upgrade();
                if let Some(window) = window {
                    rt.block_on(async {
                        if let Ok(data) = app::weather::fetch_weather_images(lat, lon).await {
                            app::weather::set_weather_images(&window, data);
                        } else {
                            error!("Failed to update weather images");
                            window.set_error_message("Failed to update weather images".into());
                        }
                    });
                }
            }).unwrap();
        }
    });

    info!("Weather station frontend started successfully");

    // Run the main window - this blocks until the window is closed
    let result = main_window.run();

    info!("Main window closed, shutting down threads...");
    result
}
