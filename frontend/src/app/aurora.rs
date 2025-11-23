use std::sync::Mutex;
use std::error::Error;
use once_cell::sync::Lazy;
use slint::ComponentHandle;
use crate::MainWindow;
use crate::app::utils::{decode_png_to_slint_image, resize_png_if_needed};
use tokio::time::Duration;

pub static ALL_SKY_IMAGES: Lazy<Mutex<Vec<Vec<u8>>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static ALL_SKY_INDEX: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));

// Location names for all-sky images in the same order as the API
const ALL_SKY_LOCATIONS: &[&str] = &[
    "Kjell Henriksen Observatory, Norway",
    "Hankasalmi, Finland",
    "Yellowknife, Canada",
    "Athabasca, Canada",
    "Glacier National Park, USA",
    "Hansville, USA",
    "Isle Royale National Park, USA",
    "Heiligenblut, Austria",
    "Calgary, Canada",
    "Hobart, Australia",
];

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

                // Clone weak for async use in background thread
                let weak_clone = window.as_weak();

                // Spawn background thread for async refresh to avoid freezing UI
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async move {
                        if let Err(e) = update_aurora_images(weak_clone).await {
                            error!("Aurora refresh failed: {}", e);
                        }
                    });
                });
            }
        }).unwrap();
    });

    // All-sky navigation
    let main_window_weak = main_window.as_weak();
    main_window.on_aurora_all_sky_previous(move || {
        debug!("Aurora all-sky previous button clicked");
        let window_weak = main_window_weak.clone();
        slint::invoke_from_event_loop(move || {
            debug!("Processing Aurora all-sky previous in event loop");
            let window = window_weak.upgrade();
            if let Some(window) = window {
                debug!("Window upgraded successfully");
                if let Ok(images) = ALL_SKY_IMAGES.try_lock() {
                    debug!("ALL_SKY_IMAGES lock acquired, {} images", images.len());
                    if let Ok(mut index) = ALL_SKY_INDEX.try_lock() {
                        debug!("ALL_SKY_INDEX lock acquired, current index: {}", *index);
                        if !images.is_empty() {
                            if *index == 0 {
                                *index = images.len() - 1;
                            } else {
                                *index -= 1;
                            }
                            debug!("New index: {}", *index);
                            let current_image_data = &images[*index];
                            let location_name = ALL_SKY_LOCATIONS.get(*index).unwrap_or(&"Unknown Location");
                            match decode_png_to_slint_image(current_image_data) {
                                Ok(slint_image) => {
                                    window.set_aurora_all_sky_image(slint_image);
                                    window.set_aurora_all_sky_location_name(location_name.to_string().into());
                                }
                                Err(e) => {
                                    error!("Failed to decode Aurora all-sky image: {}", e);
                                }
                            }
                        } else {
                            debug!("No Aurora all-sky images available");
                        }
                    } else {
                        debug!("Failed to acquire ALL_SKY_INDEX lock");
                    }
                } else {
                    debug!("Failed to acquire ALL_SKY_IMAGES lock");
                }
            } else {
                debug!("Failed to upgrade window weak reference");
            }
        }).unwrap();
    });

    let main_window_weak2 = main_window.as_weak();
    main_window.on_aurora_all_sky_next(move || {
        debug!("Aurora all-sky next button clicked");
        let window_weak = main_window_weak2.clone();
        slint::invoke_from_event_loop(move || {
            debug!("Processing Aurora all-sky next in event loop");
            let window = window_weak.upgrade();
            if let Some(window) = window {
                debug!("Window upgraded successfully");
                if let Ok(images) = ALL_SKY_IMAGES.try_lock() {
                    debug!("ALL_SKY_IMAGES lock acquired, {} images", images.len());
                    if let Ok(mut index) = ALL_SKY_INDEX.try_lock() {
                        debug!("ALL_SKY_INDEX lock acquired, current index: {}", *index);
                        if !images.is_empty() {
                            *index = (*index + 1) % images.len();
                            debug!("New index: {}", *index);
                            let current_image_data = &images[*index];
                            let location_name = ALL_SKY_LOCATIONS.get(*index).unwrap_or(&"Unknown Location");
                            match decode_png_to_slint_image(current_image_data) {
                                Ok(slint_image) => {
                                    window.set_aurora_all_sky_image(slint_image);
                                    window.set_aurora_all_sky_location_name(location_name.to_string().into());
                                }
                                Err(e) => {
                                    error!("Failed to decode Aurora all-sky image: {}", e);
                                }
                            }
                        } else {
                            debug!("No Aurora all-sky images available");
                        }
                    } else {
                        debug!("Failed to acquire ALL_SKY_INDEX lock");
                    }
                } else {
                    debug!("Failed to acquire ALL_SKY_IMAGES lock");
                }
            } else {
                debug!("Failed to upgrade window weak reference");
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

pub async fn update_aurora_images(main_window_weak: slint::Weak<MainWindow>) -> Result<(), Box<dyn std::error::Error>> {
    use aurora::{fetch_aurora_forecast, fetch_ace_real_time_solar_wind, fetch_dscovr_solar_wind,
                     fetch_space_weather_overview, fetch_ace_epam, fetch_canadian_magnetic,
                     fetch_alerts_timeline, fetch_all_aurora_images};

    info!("Starting Aurora images update...");

    // Create clones for each async block to avoid borrow checker issues
    let weak1 = main_window_weak.clone();
    let weak2 = main_window_weak.clone();
    let weak3 = main_window_weak.clone();
    let weak4 = main_window_weak.clone();
    let weak5 = main_window_weak.clone();
    let weak6 = main_window_weak.clone();
    let weak7 = main_window_weak.clone();
    let weak8 = main_window_weak.clone();

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

        // All-sky images (fetched as one unit)
        async {
            match fetch_all_aurora_images().await {
                Ok(all_sky_images_result) => {
                    info!("Successfully fetched: All Sky Images");

                    let all_sky_vec = vec![
                        all_sky_images_result.kjell_henriksen_observatory_norway,
                        all_sky_images_result.hankasalmi_finland,
                        all_sky_images_result.yellowknife_canada,
                        all_sky_images_result.athabasca_canada,
                        all_sky_images_result.glacier_national_park_usa,
                        all_sky_images_result.hansville_usa,
                        all_sky_images_result.isle_royale_national_park_usa,
                        all_sky_images_result.heiligenblut_austria,
                        all_sky_images_result.calgary_canada,
                        all_sky_images_result.hobart_australia,
                    ];

                    // Resize all-sky images if needed
                    let mut resized_all_sky = Vec::new();
                    for image_data in all_sky_vec {
                        let resized = if image_data.is_empty() {
                            Vec::new()
                        } else {
                            match resize_png_if_needed(image_data, 1920) {
                                Ok(r) => {
                                    info!("Successfully resized all-sky image");
                                    r
                                }
                                Err(e) => {
                                    error!("Failed to resize all-sky image: {}", e);
                                    Vec::new()
                                }
                            }
                        };
                        resized_all_sky.push(resized);
                    }

                    // Store resized in global
                    {
                        *ALL_SKY_IMAGES.lock().unwrap() = resized_all_sky.clone();
                    }

                    // Update UI in event loop - decode inside the closure
                    slint::invoke_from_event_loop(move || {
                        if let Some(window) = main_window_weak.upgrade() {
                            let mut slint_images = Vec::new();
                            let mut error_flags = Vec::new();
                            for image_data in &resized_all_sky {
                                if !image_data.is_empty() {
                                    match decode_png_to_slint_image(image_data) {
                                        Ok(slint_image) => {
                                            slint_images.push(slint_image);
                                            error_flags.push(false);
                                        }
                                        Err(e) => {
                                            error!("Failed to decode all-sky image: {}", e);
                                            slint_images.push(slint::Image::default());
                                            error_flags.push(true);
                                        }
                                    }
                                } else {
                                    slint_images.push(slint::Image::default());
                                    error_flags.push(true);
                                }
                            }

                            // Set the arrays
                            window.set_aurora_all_sky_images(slint_images.as_slice().into());
                            window.set_aurora_all_sky_errors(error_flags.as_slice().into());
                            // Update current display
                            update_all_sky_display(&window);
                        }
                    }).unwrap();
                }
                Err(e) => {
                    error!("Failed to fetch All Sky Images: {}", e);
                    slint::invoke_from_event_loop(move || {
                        if let Some(window) = main_window_weak.upgrade() {
                            *ALL_SKY_IMAGES.lock().unwrap() = vec![Vec::new(); 10];
                            let default_images: Vec<slint::Image> = vec![slint::Image::default(); 10];
                            let error_flags: Vec<bool> = vec![true; 10];
                            window.set_aurora_all_sky_images(default_images.as_slice().into());
                            window.set_aurora_all_sky_errors(error_flags.as_slice().into());
                            update_all_sky_display(&window);
                        }
                    }).unwrap();
                }
            };
        },
    );

    info!("All Aurora images updated successfully");
    Ok(())
}

pub fn update_all_sky_display(main_window: &MainWindow) {
    info!("Starting update_all_sky_display...");
    let images = ALL_SKY_IMAGES.lock().unwrap();
    let index = ALL_SKY_INDEX.lock().unwrap();
    info!("Locked ALL_SKY_IMAGES and ALL_SKY_INDEX, images len: {}, index: {}", images.len(), *index);

    if !images.is_empty() && *index < images.len() && !images[*index].is_empty() {
        let current_image_data = &images[*index];
        let location_name = ALL_SKY_LOCATIONS.get(*index).unwrap_or(&"Unknown Location");
        info!("Displaying Aurora all-sky image (index: {}, location: {}, data len: {})", *index, location_name, current_image_data.len());

        info!("Starting decode_png_to_slint_image for current image...");
        match decode_png_to_slint_image(current_image_data) {
            Ok(slint_image) => {
                info!("Successfully decoded current all-sky image, setting UI...");
                main_window.set_aurora_all_sky_image(slint_image);
                main_window.set_aurora_all_sky_location_name(location_name.to_string().into());
                info!("Set current all-sky image and location name");
            }
            Err(e) => {
                error!("Failed to decode Aurora all-sky image: {}", e);
            }
        }
    } else {
        info!("No Aurora all-sky images available for display ({} images, index {})", images.len(), *index);
    }
    info!("update_all_sky_display completed");
}

fn start_refresh_countdown(main_window: &MainWindow) {
    let window_weak = main_window.as_weak();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            let mut remaining_seconds = 300; // 5 minutes

            loop {
                interval.tick().await;
                remaining_seconds -= 1;

                if remaining_seconds <= 0 {
                    // Countdown finished, re-enable button
                    slint::invoke_from_event_loop(move || {
                        if let Some(window) = window_weak.upgrade() {
                            window.set_aurora_refresh_button_enabled(true);
                            window.set_aurora_refresh_button_text("Refresh".into());
                        }
                    }).ok();
                    break;
                }

                // Update button text with countdown
                let minutes = remaining_seconds / 60;
                let seconds = remaining_seconds % 60;
                let countdown_text = format!("Refresh in {:02}:{:02}", minutes, seconds);

                let window_weak_clone = window_weak.clone();
                slint::invoke_from_event_loop(move || {
                    if let Some(window) = window_weak_clone.upgrade() {
                        window.set_aurora_refresh_button_text(countdown_text.into());
                    }
                }).ok();
            }
        });
    });
}
