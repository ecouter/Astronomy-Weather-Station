mod app;

slint::include_modules!();

extern crate pretty_env_logger;
#[macro_use] extern crate log;

use std::time::Duration;
use std::thread;
use std::sync::mpsc;

fn main() -> Result<(), slint::PlatformError> {
    pretty_env_logger::init();

    info!("Starting weather station frontend...");

    let main_window = MainWindow::new()?;

    // Set up callback handlers using the modular functions
    app::cloud_cover::setup_cloud_cover_callbacks(&main_window);
    app::wind::setup_wind_callbacks(&main_window);
    app::environment_canada::setup_environment_canada_callbacks(&main_window);

    // Start the async runtime for image fetching
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Initial image load
    rt.block_on(async {
        if let Err(e) = app::weather::update_weather_images(&main_window).await {
            error!("Failed to load initial images: {}", e);
            main_window.set_error_message(format!("Failed to load images: {}", e).into());
        }
        if let Err(e) = app::cloud_cover::update_cloud_cover_images(&main_window).await {
            error!("Failed to load initial cloud cover images: {}", e);
            main_window.set_error_message(format!("Failed to load cloud cover images: {}", e).into());
        }
        if let Err(e) = app::wind::update_wind_images(&main_window).await {
            error!("Failed to load initial wind images: {}", e);
            main_window.set_error_message(format!("Failed to load wind images: {}", e).into());
        }
        if let Err(e) = app::clearoutside::update_clearoutside_data(&main_window).await {
            error!("Failed to load initial ClearOutside data: {}", e);
            main_window.set_error_message(format!("Failed to load ClearOutside data: {}", e).into());
        }
        match app::map::load_map_image(&main_window).await {
            Ok(map_image) => {
                main_window.set_map_image(map_image);
            }
            Err(e) => {
                error!("Failed to load map image: {}", e);
                main_window.set_error_message(format!("Failed to load map image: {}", e).into());
            }
        }
        match app::cleardarksky::load_cleardarksky_image(&main_window).await {
            Ok(cleardarksky_image) => {
                main_window.set_cleardarksky_image(cleardarksky_image);
            }
            Err(e) => {
                error!("Failed to load ClearDarkSky image: {}", e);
                main_window.set_error_message(format!("Failed to load ClearDarkSky image: {}", e).into());
            }
        }
    });

    main_window.set_loading(false);

    // Load initial Environment Canada images
    {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = app::environment_canada::update_environment_canada_images(&main_window).await {
                error!("Failed to load Environment Canada images: {}", e);
            }
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

    // Handle cloud cover updates directly in the main thread using invoke_from_event_loop
    let main_window_weak = main_window.as_weak();
    let _cloud_update_handle = thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        info!("Cloud cover update thread started");
        while let Ok(signal) = cloud_rx.recv() {
            info!("Received cloud signal: {}", signal);
            let window_weak = main_window_weak.clone();
            match signal {
                "update" => {
                    info!("Processing cloud update signal");
                    // Use invoke_from_event_loop to run async code in the UI thread
                    slint::invoke_from_event_loop(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let window = window_weak.upgrade();
                        if let Some(window) = window {
                            rt.block_on(async {
                                if let Err(e) = app::cloud_cover::update_cloud_cover_images(&window).await {
                                    error!("Failed to update cloud cover images: {}", e);
                                    window.set_error_message(format!("Failed to update cloud cover images: {}", e).into());
                                }
                                // Also update ClearDarkSky chart
                                match app::cleardarksky::load_cleardarksky_image(&window).await {
                                    Ok(cleardarksky_image) => {
                                        window.set_cleardarksky_image(cleardarksky_image);
                                        info!("Updated ClearDarkSky chart");
                                    }
                                    Err(e) => {
                                        error!("Failed to update ClearDarkSky image: {}", e);
                                        window.set_error_message(format!("Failed to update ClearDarkSky image: {}", e).into());
                                    }
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
                                if let Err(e) = app::clearoutside::update_clearoutside_data(&window).await {
                                    error!("Failed to update ClearOutside data: {}", e);
                                    window.set_error_message(format!("Failed to update ClearOutside data: {}", e).into());
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
                "nina_update" => {
                    info!("Processing NINA update signal");
                    // Use invoke_from_event_loop to run async code in the UI thread
                    slint::invoke_from_event_loop(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let window = window_weak.upgrade();
                        if let Some(window) = window {
                            rt.block_on(async {
                                if let Err(e) = app::nina::update_nina_images(&window).await {
                                    error!("Failed to update NINA images: {}", e);
                                    window.set_error_message(format!("Failed to update NINA images: {}", e).into());
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
    let _wind_update_handle = thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        info!("Wind update thread started");
        while let Ok(signal) = wind_rx.recv() {
            info!("Received wind signal: {}", signal);
            let window_weak = main_window_weak2.clone();
            match signal {
                "update" => {
                    info!("Processing wind update signal");
                    // Use invoke_from_event_loop to run async code in the UI thread
                    slint::invoke_from_event_loop(move || {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let window = window_weak.upgrade();
                        if let Some(window) = window {
                            rt.block_on(async {
                                if let Err(e) = app::wind::update_wind_images(&window).await {
                                    error!("Failed to update wind images: {}", e);
                                    window.set_error_message(format!("Failed to update wind images: {}", e).into());
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

    // Keep the main window alive by storing it
    let _main_window_handle = main_window.as_weak();

    // Handle weather updates in the UI thread
    let main_window_weak2 = main_window.as_weak();
    let _weather_update_handle = thread::spawn(move || {
        while let Ok(()) = rx.recv() {
            let window_weak = main_window_weak2.clone();
            // Use invoke_from_event_loop to run async code in the UI thread
            slint::invoke_from_event_loop(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                let window = window_weak.upgrade();
                if let Some(window) = window {
                    rt.block_on(async {
                        if let Err(e) = app::weather::update_weather_images(&window).await {
                            error!("Failed to update images: {}", e);
                            window.set_error_message(format!("Failed to update images: {}", e).into());
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
