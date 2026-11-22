mod app;

slint::include_modules!();

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use std::time::Duration;

use slint::ComponentHandle;

use crate::app::refresh::{spawn, start_periodic};

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

    // NINA URL change: read the new URL on the UI thread, then let NINA set up
    // its own background tasks. No runtime is created on the UI thread.
    {
        let weak = main_window.as_weak();
        main_window.on_nina_url_saved(move |index| {
            let window = match weak.upgrade() {
                Some(w) => w,
                None => return,
            };
            let url_index = index as usize;
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

            app::nina::clear_nina_error_state(&window, url_index);
            if let Err(e) = app::nina::handle_nina_url_change(url_index, url, &window) {
                error!("Failed to handle NINA URL change for slot {}: {}", url_index + 1, e);
                app::nina::set_nina_error_state(&window, url_index, "Connection could not be established");
            } else {
                info!("Successfully handled NINA URL change for slot {}", url_index + 1);
            }
        });
    }

    // NINA image update signals: a single long-lived listener. It only waits on
    // the channel; each image refresh runs on the shared runtime.
    {
        let (sender, receiver) = std::sync::mpsc::channel();
        *app::nina::NINA_IMAGE_UPDATE_SENDER.lock().unwrap() = Some(sender);
        *app::nina::NINA_IMAGE_UPDATE_CHANNEL.lock().unwrap() = Some(receiver);

        let weak = main_window.as_weak();
        std::thread::spawn(move || {
            info!("NINA image update handler thread started");
            while let Some(receiver) = app::nina::NINA_IMAGE_UPDATE_CHANNEL.lock().unwrap().as_ref() {
                match receiver.recv() {
                    Ok((slot_index, base_url)) => {
                        info!("Processing NINA image update for slot {} from {}", slot_index + 1, base_url);
                        let w = weak.clone();
                        spawn(async move {
                            if let Err(e) = app::nina::update_single_nina_image(w, slot_index, &base_url).await {
                                error!("Failed to update NINA image for slot {}: {}", slot_index + 1, e);
                            }
                        });
                    }
                    Err(_) => break, // channel closed
                }
            }
            info!("NINA image update handler thread ended");
        });
    }

    main_window.set_loading(false); // show the UI immediately; data loads in the background

    let weak = main_window.as_weak();

    // =====================================================================
    // Initial data loads — fetch on the shared runtime, apply on the UI loop
    // =====================================================================

    {
        let w = weak.clone();
        spawn(async move {
            match app::weather::fetch_weather_images(lat, lon).await {
                Ok(data) => {
                    w.upgrade_in_event_loop(move |win| {
                        app::weather::set_weather_images(&win, data);
                        info!("Initial weather data loaded");
                    })
                    .ok();
                }
                Err(e) => error!("Failed to load initial weather images: {}", e),
            }
        });
    }

    {
        let w = weak.clone();
        spawn(async move {
            match app::cloud_cover::fetch_cloud_cover_images(lat, lon).await {
                Ok(images) => {
                    w.upgrade_in_event_loop(move |win| {
                        app::cloud_cover::set_cloud_cover_images(&win, images);
                        info!("Initial cloud cover data loaded");
                    })
                    .ok();
                }
                Err(e) => error!("Failed to load initial cloud cover images: {}", e),
            }
        });
    }

    {
        let w = weak.clone();
        spawn(async move {
            match app::wind::fetch_wind_images(lat, lon).await {
                Ok((images, legend)) => {
                    w.upgrade_in_event_loop(move |win| {
                        app::wind::set_wind_images(&win, (images, legend));
                        info!("Initial wind data loaded");
                    })
                    .ok();
                }
                Err(e) => error!("Failed to load initial wind images: {}", e),
            }
        });
    }

    {
        let w = weak.clone();
        let w_meteo = weak.clone();
        spawn(async move {
            match app::clearoutside::fetch_clearoutside_data(lat, lon)
                .await
                .map_err(|e| e.to_string())
            {
                Ok(data) => {
                    w.upgrade_in_event_loop(move |win| {
                        app::clearoutside::set_clearoutside_data(&win, data);
                        info!("Initial ClearOutside data loaded");
                    })
                    .ok();

                    use clearoutside::ClearOutsideAPI;
                    let lat_str = format!("{:.2}", lat);
                    let lon_str = format!("{:.2}", lon);
                    if let Ok(api) = ClearOutsideAPI::new(&lat_str, &lon_str, Some("midnight")).await {
                        if let Ok(forecast) = api.pull() {
                            if let Ok(meteoblue_data) =
                                app::meteoblue::fetch_meteoblue_data(lat, lon, &forecast).await
                            {
                                w_meteo
                                    .upgrade_in_event_loop(move |win| {
                                        app::meteoblue::set_meteoblue_data(&win, meteoblue_data);
                                        info!("Initial MeteoBlue data loaded");
                                    })
                                    .ok();
                            }
                        }
                    }
                }
                Err(e) => error!("Failed to load initial ClearOutside data: {}", e),
            }
        });
    }

    {
        let w = weak.clone();
        spawn(async move {
            match app::map::fetch_map_image(lat, lon).await {
                Ok(image) => {
                    w.upgrade_in_event_loop(move |win| {
                        app::map::set_map_image(&win, image);
                        info!("Initial map data loaded");
                    })
                    .ok();
                }
                Err(e) => error!("Failed to load initial map image: {}", e),
            }
        });
    }

    {
        let w = weak.clone();
        spawn(async move {
            match app::cleardarksky::fetch_cleardarksky_image(lat, lon).await {
                Ok(image) => {
                    w.upgrade_in_event_loop(move |win| {
                        app::cleardarksky::set_cleardarksky_image(&win, image);
                        info!("Initial ClearDarkSky image loaded");
                    })
                    .ok();
                }
                Err(e) => error!("Failed to load initial ClearDarkSky image: {}", e),
            }
        });
    }

    {
        let w = weak.clone();
        spawn(async move {
            match app::sounding::fetch_sounding_image(lat, lon).await {
                Ok(image) => {
                    w.upgrade_in_event_loop(move |win| {
                        app::sounding::set_sounding_image(&win, image);
                        info!("Initial sounding data loaded");
                    })
                    .ok();
                }
                Err(e) => error!("Failed to load initial sounding image: {}", e),
            }
        });
    }

    {
        let w = weak.clone();
        spawn(async move {
            if let Err(e) = app::environment_canada::update_environment_canada_images(w).await {
                error!("Failed to load Environment Canada images: {}", e);
            }
        });
    }

    {
        let w = weak.clone();
        spawn(async move {
            if let Err(e) = app::precipitation::fetch_initial_precipitation(lat, lon).await {
                error!("Failed to fetch initial precipitation data: {}", e);
            }
            w.upgrade_in_event_loop(|win| app::precipitation::update_precipitation_display(&win))
                .ok();
        });
    }

    // =====================================================================
    // Periodic refresh jobs
    // =====================================================================

    // Weather every 10 minutes
    {
        let weak = weak.clone();
        start_periodic(Duration::from_secs(600), move || {
            let w = weak.clone();
            let w_err = weak.clone();
            async move {
                match app::weather::fetch_weather_images(lat, lon).await {
                    Ok(data) => {
                        w.upgrade_in_event_loop(move |win| app::weather::set_weather_images(&win, data))
                            .ok();
                    }
                    Err(e) => {
                        error!("Failed to update weather images: {}", e);
                        w_err
                            .upgrade_in_event_loop(|win| {
                                win.set_error_message("Failed to update weather images".into());
                            })
                            .ok();
                    }
                }
            }
        });
    }

    // Cloud cover + ClearDarkSky chart hourly
    {
        let weak = weak.clone();
        start_periodic(Duration::from_secs(3600), move || {
            let w_cloud = weak.clone();
            let w_cloud_err = weak.clone();
            let w_cds = weak.clone();
            async move {
                match app::cloud_cover::fetch_cloud_cover_images(lat, lon).await {
                    Ok(images) => {
                        w_cloud
                            .upgrade_in_event_loop(move |win| {
                                app::cloud_cover::set_cloud_cover_images(&win, images);
                                info!("Updated cloud cover images");
                            })
                            .ok();
                    }
                    Err(e) => {
                        error!("Failed to update cloud cover images: {}", e);
                        w_cloud_err
                            .upgrade_in_event_loop(|win| {
                                win.set_error_message("Failed to update cloud cover images".into());
                            })
                            .ok();
                    }
                }
                match app::cleardarksky::fetch_cleardarksky_image(lat, lon).await {
                    Ok(image) => {
                        w_cds
                            .upgrade_in_event_loop(|win| {
                                app::cleardarksky::set_cleardarksky_image(&win, image);
                                info!("Updated ClearDarkSky chart");
                            })
                            .ok();
                    }
                    Err(e) => error!("Failed to update ClearDarkSky image: {}", e),
                }
            }
        });
    }

    // ClearOutside + MeteoBlue hourly
    {
        let weak = weak.clone();
        start_periodic(Duration::from_secs(3600), move || {
            let w_clear = weak.clone();
            let w_clear_err = weak.clone();
            let w_meteo = weak.clone();
            async move {
                match app::clearoutside::fetch_clearoutside_data(lat, lon)
                    .await
                    .map_err(|e| e.to_string())
                {
                    Ok(data) => {
                        w_clear
                            .upgrade_in_event_loop(move |win| {
                                app::clearoutside::set_clearoutside_data(&win, data);
                            })
                            .ok();

                        use clearoutside::ClearOutsideAPI;
                        let lat_str = format!("{:.2}", lat);
                        let lon_str = format!("{:.2}", lon);
                        if let Ok(api) = ClearOutsideAPI::new(&lat_str, &lon_str, Some("midnight")).await {
                            if let Ok(forecast) = api.pull() {
                                if let Ok(meteoblue_data) =
                                    app::meteoblue::fetch_meteoblue_data(lat, lon, &forecast).await
                                {
                                    w_meteo
                                        .upgrade_in_event_loop(move |win| {
                                            app::meteoblue::set_meteoblue_data(&win, meteoblue_data);
                                            info!("Updated MeteoBlue data");
                                        })
                                        .ok();
                                } else {
                                    error!("Failed to update MeteoBlue data");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to update ClearOutside data: {}", e);
                        w_clear_err
                            .upgrade_in_event_loop(|win| {
                                win.set_error_message("Failed to update ClearOutside data".into());
                            })
                            .ok();
                    }
                }
            }
        });
    }

    // Wind hourly
    {
        let weak = weak.clone();
        start_periodic(Duration::from_secs(3600), move || {
            let w = weak.clone();
            let w_err = weak.clone();
            async move {
                match app::wind::fetch_wind_images(lat, lon).await {
                    Ok((images, legend)) => {
                        w.upgrade_in_event_loop(move |win| {
                            app::wind::set_wind_images(&win, (images, legend));
                            info!("Updated wind images");
                        })
                        .ok();
                    }
                    Err(e) => {
                        error!("Failed to update wind images: {}", e);
                        w_err
                            .upgrade_in_event_loop(|win| {
                                win.set_error_message("Failed to update wind images".into());
                            })
                            .ok();
                    }
                }
            }
        });
    }

    // Environment Canada hourly
    {
        let weak = weak.clone();
        start_periodic(Duration::from_secs(3600), move || {
            let w = weak.clone();
            async move {
                if let Err(e) = app::environment_canada::update_environment_canada_images(w).await {
                    error!("Failed to update Environment Canada images: {}", e);
                }
            }
        });
    }

    // Precipitation every 30 minutes
    {
        let weak = weak.clone();
        start_periodic(Duration::from_secs(1800), move || {
            let w = weak.clone();
            async move {
                if let Err(e) =
                    app::precipitation::refresh_precipitation_data(lat, lon, w).await
                {
                    error!("Failed to refresh precipitation data: {}", e);
                }
            }
        });
    }

    info!("Weather station frontend started successfully");

    // Run the main window - this blocks until the window is closed
    main_window.run()
}
