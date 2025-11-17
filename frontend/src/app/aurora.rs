use std::sync::Mutex;
use once_cell::sync::Lazy;
use slint::ComponentHandle;
use crate::MainWindow;
use crate::app::utils::decode_png_to_slint_image;
use tokio::time::{timeout, Duration};

pub static AURORA_FORECAST_IMAGE: Lazy<Mutex<Vec<u8>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static ACE_SOLAR_WIND_IMAGE: Lazy<Mutex<Vec<u8>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static DSCOVR_SOLAR_WIND_IMAGE: Lazy<Mutex<Vec<u8>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static SPACE_WEATHER_OVERVIEW_IMAGE: Lazy<Mutex<Vec<u8>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static ACE_EPAM_IMAGE: Lazy<Mutex<Vec<u8>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static CANADIAN_MAGNETIC_IMAGE: Lazy<Mutex<Vec<u8>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static ALERTS_TIMELINE_IMAGE: Lazy<Mutex<Vec<u8>>> = Lazy::new(|| Mutex::new(Vec::new()));
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

                // Clone window for async use
                let window_clone = window.clone_strong();

                // Start async refresh
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async move {
                    if let Err(e) = update_aurora_images(&window_clone).await {
                        error!("Aurora refresh failed: {}", e);
                    }
                });

                // Start countdown timer
                start_refresh_countdown(&window);
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

pub async fn update_aurora_images(main_window: &MainWindow) -> Result<(), Box<dyn std::error::Error>> {
    use aurora::{fetch_aurora_forecast, fetch_ace_real_time_solar_wind, fetch_dscovr_solar_wind,
                     fetch_space_weather_overview, fetch_ace_epam, fetch_canadian_magnetic,
                     fetch_alerts_timeline, fetch_all_aurora_images};

    info!("Updating Aurora images...");

    // Fetch simple images concurrently with individual timeouts
    let mut results = Vec::new();
    let mut failed_requests = Vec::new();

    // Aurora Forecast
    match timeout(Duration::from_secs(15), fetch_aurora_forecast()).await {
        Ok(Ok(result)) => {
            results.push(result);
            info!("Successfully loaded: Aurora Forecast");
        }
        Ok(Err(e)) => {
            error!("Failed to load Aurora Forecast: {}", e);
            failed_requests.push(format!("Aurora Forecast (error: {})", e));
            results.push(Vec::new());
        }
        Err(_) => {
            error!("Timeout loading Aurora Forecast after 15 seconds");
            failed_requests.push("Aurora Forecast (timeout)".to_string());
            results.push(Vec::new());
        }
    }

    // ACE Solar Wind
    match timeout(Duration::from_secs(15), fetch_ace_real_time_solar_wind()).await {
        Ok(Ok(result)) => {
            results.push(result);
            info!("Successfully loaded: ACE Solar Wind");
        }
        Ok(Err(e)) => {
            error!("Failed to load ACE Solar Wind: {}", e);
            failed_requests.push(format!("ACE Solar Wind (error: {})", e));
            results.push(Vec::new());
        }
        Err(_) => {
            error!("Timeout loading ACE Solar Wind after 15 seconds");
            failed_requests.push("ACE Solar Wind (timeout)".to_string());
            results.push(Vec::new());
        }
    }

    // DSCOVR Solar Wind
    match timeout(Duration::from_secs(15), fetch_dscovr_solar_wind()).await {
        Ok(Ok(result)) => {
            results.push(result);
            info!("Successfully loaded: DSCOVR Solar Wind");
        }
        Ok(Err(e)) => {
            error!("Failed to load DSCOVR Solar Wind: {}", e);
            failed_requests.push(format!("DSCOVR Solar Wind (error: {})", e));
            results.push(Vec::new());
        }
        Err(_) => {
            error!("Timeout loading DSCOVR Solar Wind after 15 seconds");
            failed_requests.push("DSCOVR Solar Wind (timeout)".to_string());
            results.push(Vec::new());
        }
    }

    // Space Weather Overview
    match timeout(Duration::from_secs(15), fetch_space_weather_overview()).await {
        Ok(Ok(result)) => {
            results.push(result);
            info!("Successfully loaded: Space Weather Overview");
        }
        Ok(Err(e)) => {
            error!("Failed to load Space Weather Overview: {}", e);
            failed_requests.push(format!("Space Weather Overview (error: {})", e));
            results.push(Vec::new());
        }
        Err(_) => {
            error!("Timeout loading Space Weather Overview after 15 seconds");
            failed_requests.push("Space Weather Overview (timeout)".to_string());
            results.push(Vec::new());
        }
    }

    // ACE EPAM
    match timeout(Duration::from_secs(15), fetch_ace_epam()).await {
        Ok(Ok(result)) => {
            results.push(result);
            info!("Successfully loaded: ACE EPAM");
        }
        Ok(Err(e)) => {
            error!("Failed to load ACE EPAM: {}", e);
            failed_requests.push(format!("ACE EPAM (error: {})", e));
            results.push(Vec::new());
        }
        Err(_) => {
            error!("Timeout loading ACE EPAM after 15 seconds");
            failed_requests.push("ACE EPAM (timeout)".to_string());
            results.push(Vec::new());
        }
    }

    // Canadian Magnetic
    match timeout(Duration::from_secs(15), fetch_canadian_magnetic()).await {
        Ok(Ok(result)) => {
            results.push(result);
            info!("Successfully loaded: Canadian Magnetic");
        }
        Ok(Err(e)) => {
            error!("Failed to load Canadian Magnetic: {}", e);
            failed_requests.push(format!("Canadian Magnetic (error: {})", e));
            results.push(Vec::new());
        }
        Err(_) => {
            error!("Timeout loading Canadian Magnetic after 15 seconds");
            failed_requests.push("Canadian Magnetic (timeout)".to_string());
            results.push(Vec::new());
        }
    }

    // Alerts Timeline
    match timeout(Duration::from_secs(15), fetch_alerts_timeline()).await {
        Ok(Ok(result)) => {
            results.push(result);
            info!("Successfully loaded: Alerts Timeline");
        }
        Ok(Err(e)) => {
            error!("Failed to load Alerts Timeline: {}", e);
            failed_requests.push(format!("Alerts Timeline (error: {})", e));
            results.push(Vec::new());
        }
        Err(_) => {
            error!("Timeout loading Alerts Timeline after 15 seconds");
            failed_requests.push("Alerts Timeline (timeout)".to_string());
            results.push(Vec::new());
        }
    }

    // Handle all-sky images separately since it returns a struct
    info!("Starting to fetch all-sky images...");
    let all_sky_images_result = match timeout(Duration::from_secs(15), fetch_all_aurora_images()).await {
        Ok(Ok(result)) => {
            info!("Successfully loaded: All Sky Images");
            result
        }
        Ok(Err(e)) => {
            error!("Failed to load All Sky Images: {}", e);
            failed_requests.push(format!("All Sky Images (error: {})", e));
            // Return empty struct
            aurora::AuroraAllSkyImages {
                kjell_henriksen_observatory_norway: Vec::new(),
                hankasalmi_finland: Vec::new(),
                yellowknife_canada: Vec::new(),
                athabasca_canada: Vec::new(),
                glacier_national_park_usa: Vec::new(),
                hansville_usa: Vec::new(),
                isle_royale_national_park_usa: Vec::new(),
                heiligenblut_austria: Vec::new(),
                calgary_canada: Vec::new(),
                hobart_australia: Vec::new(),
            }
        }
        Err(_) => {
            error!("Timeout loading All Sky Images after 15 seconds");
            failed_requests.push("All Sky Images (timeout)".to_string());
            // Return empty struct
            aurora::AuroraAllSkyImages {
                kjell_henriksen_observatory_norway: Vec::new(),
                hankasalmi_finland: Vec::new(),
                yellowknife_canada: Vec::new(),
                athabasca_canada: Vec::new(),
                glacier_national_park_usa: Vec::new(),
                hansville_usa: Vec::new(),
                isle_royale_national_park_usa: Vec::new(),
                heiligenblut_austria: Vec::new(),
                calgary_canada: Vec::new(),
                hobart_australia: Vec::new(),
            }
        }
    };
    info!("All-sky images fetch completed, now extracting data...");

    // Extract results in correct order
    let aurora_forecast = results[0].clone();
    let ace_solar_wind = results[1].clone();
    let dscovr_solar_wind = results[2].clone();
    let space_weather_overview = results[3].clone();
    let ace_epam = results[4].clone();
    let canadian_magnetic = results[5].clone();
    let alerts_timeline = results[6].clone();

    // Report any failures
    if !failed_requests.is_empty() {
        let failure_summary = failed_requests.join(", ");
        warn!("Some aurora requests failed or timed out: {}", failure_summary);
    }

    // Store images in global storage
    info!("Storing images in global storage...");
    {
        *AURORA_FORECAST_IMAGE.lock().unwrap() = aurora_forecast;
        *ACE_SOLAR_WIND_IMAGE.lock().unwrap() = ace_solar_wind;
        *DSCOVR_SOLAR_WIND_IMAGE.lock().unwrap() = dscovr_solar_wind;
        *SPACE_WEATHER_OVERVIEW_IMAGE.lock().unwrap() = space_weather_overview;
        *ACE_EPAM_IMAGE.lock().unwrap() = ace_epam;
        *CANADIAN_MAGNETIC_IMAGE.lock().unwrap() = canadian_magnetic;
        *ALERTS_TIMELINE_IMAGE.lock().unwrap() = alerts_timeline;

        // Extract all-sky images from the struct
        info!("Extracting all-sky images from struct...");
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
        info!("All-sky vec created with {} images", all_sky_vec.len());
        *ALL_SKY_IMAGES.lock().unwrap() = all_sky_vec;
    }
    info!("Images stored, now updating UI displays...");

    // Update UI with images
    info!("Starting UI updates...");
    update_aurora_forecast_display(main_window);
    info!("Updated aurora forecast display");
    update_ace_solar_wind_display(main_window);
    info!("Updated ACE solar wind display");
    update_dscovr_solar_wind_display(main_window);
    info!("Updated DSCOVR solar wind display");
    update_space_weather_overview_display(main_window);
    info!("Updated space weather overview display");
    update_ace_epam_display(main_window);
    info!("Updated ACE EPAM display");
    update_canadian_magnetic_display(main_window);
    info!("Updated Canadian magnetic display");
    update_alerts_timeline_display(main_window);
    info!("Updated alerts timeline display");
    info!("Starting all-sky display update...");
    update_all_sky_display(main_window);
    info!("All UI updates completed");

    info!("Aurora images updated successfully");
    Ok(())
}

pub fn update_aurora_forecast_display(main_window: &MainWindow) {
    let image_data = AURORA_FORECAST_IMAGE.lock().unwrap();
    if !image_data.is_empty() {
        match decode_png_to_slint_image(&image_data) {
            Ok(slint_image) => {
                main_window.set_aurora_forecast_image(slint_image);
                main_window.set_aurora_forecast_error(false);
            }
            Err(e) => {
                error!("Failed to decode Aurora forecast image: {}", e);
                main_window.set_aurora_forecast_error(true);
            }
        }
    } else {
        main_window.set_aurora_forecast_error(true);
    }
}

pub fn update_ace_solar_wind_display(main_window: &MainWindow) {
    let image_data = ACE_SOLAR_WIND_IMAGE.lock().unwrap();
    if !image_data.is_empty() {
        match decode_png_to_slint_image(&image_data) {
            Ok(slint_image) => {
                main_window.set_aurora_ace_solar_wind_image(slint_image);
                main_window.set_aurora_ace_solar_wind_error(false);
            }
            Err(e) => {
                error!("Failed to decode ACE solar wind image: {}", e);
                main_window.set_aurora_ace_solar_wind_error(true);
            }
        }
    } else {
        main_window.set_aurora_ace_solar_wind_error(true);
    }
}

pub fn update_dscovr_solar_wind_display(main_window: &MainWindow) {
    let image_data = DSCOVR_SOLAR_WIND_IMAGE.lock().unwrap();
    if !image_data.is_empty() {
        match decode_png_to_slint_image(&image_data) {
            Ok(slint_image) => {
                main_window.set_aurora_dscovr_solar_wind_image(slint_image);
                main_window.set_aurora_dscovr_solar_wind_error(false);
            }
            Err(e) => {
                error!("Failed to decode DSCOVR solar wind image: {}", e);
                main_window.set_aurora_dscovr_solar_wind_error(true);
            }
        }
    } else {
        main_window.set_aurora_dscovr_solar_wind_error(true);
    }
}

pub fn update_space_weather_overview_display(main_window: &MainWindow) {
    let image_data = SPACE_WEATHER_OVERVIEW_IMAGE.lock().unwrap();
    if !image_data.is_empty() {
        match decode_png_to_slint_image(&image_data) {
            Ok(slint_image) => {
                main_window.set_aurora_space_weather_overview_image(slint_image);
                main_window.set_aurora_space_weather_overview_error(false);
            }
            Err(e) => {
                error!("Failed to decode space weather overview image: {}", e);
                main_window.set_aurora_space_weather_overview_error(true);
            }
        }
    } else {
        main_window.set_aurora_space_weather_overview_error(true);
    }
}

pub fn update_ace_epam_display(main_window: &MainWindow) {
    let image_data = ACE_EPAM_IMAGE.lock().unwrap();
    if !image_data.is_empty() {
        match decode_png_to_slint_image(&image_data) {
            Ok(slint_image) => {
                main_window.set_aurora_ace_epam_image(slint_image);
                main_window.set_aurora_ace_epam_error(false);
            }
            Err(e) => {
                error!("Failed to decode ACE EPAM image: {}", e);
                main_window.set_aurora_ace_epam_error(true);
            }
        }
    } else {
        main_window.set_aurora_ace_epam_error(true);
    }
}

pub fn update_canadian_magnetic_display(main_window: &MainWindow) {
    let image_data = CANADIAN_MAGNETIC_IMAGE.lock().unwrap();
    if !image_data.is_empty() {
        match decode_png_to_slint_image(&image_data) {
            Ok(slint_image) => {
                main_window.set_aurora_canadian_magnetic_image(slint_image);
                main_window.set_aurora_canadian_magnetic_error(false);
            }
            Err(e) => {
                error!("Failed to decode Canadian magnetic image: {}", e);
                main_window.set_aurora_canadian_magnetic_error(true);
            }
        }
    } else {
        main_window.set_aurora_canadian_magnetic_error(true);
    }
}

pub fn update_alerts_timeline_display(main_window: &MainWindow) {
    let image_data = ALERTS_TIMELINE_IMAGE.lock().unwrap();
    if !image_data.is_empty() {
        match decode_png_to_slint_image(&image_data) {
            Ok(slint_image) => {
                main_window.set_aurora_alerts_timeline_image(slint_image);
                main_window.set_aurora_alerts_timeline_error(false);
            }
            Err(e) => {
                error!("Failed to decode alerts timeline image: {}", e);
                main_window.set_aurora_alerts_timeline_error(true);
            }
        }
    } else {
        main_window.set_aurora_alerts_timeline_error(true);
    }
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

    // Drop the locks before calling update_all_sky_images_display to avoid deadlock
    drop(images);
    drop(index);

    // Also update the all-sky images array for the grid view
    info!("Calling update_all_sky_images_display...");
    update_all_sky_images_display(main_window);
    info!("update_all_sky_display completed");
}

pub fn update_all_sky_images_display(main_window: &MainWindow) {
    info!("Starting update_all_sky_images_display...");
    let images = ALL_SKY_IMAGES.lock().unwrap();
    info!("Locked ALL_SKY_IMAGES for array display, {} images", images.len());

    if !images.is_empty() {
        let mut slint_images = Vec::new();
        let mut error_flags = Vec::new();
        info!("Starting to process {} all-sky images for array...", images.len());

        for (i, image_data) in images.iter().enumerate() {
            info!("Processing all-sky image {} of {}, data len: {}", i + 1, images.len(), image_data.len());
            if !image_data.is_empty() {
                info!("Image {} is not empty, starting decode_png_to_slint_image...", i);
                match decode_png_to_slint_image(image_data) {
                    Ok(slint_image) => {
                        info!("Successfully decoded all-sky image {} for array", i);
                        slint_images.push(slint_image);
                        error_flags.push(false);
                    }
                    Err(e) => {
                        error!("Failed to decode Aurora all-sky image {} for array: {}", i, e);
                        // Add empty image to maintain array length
                        slint_images.push(slint::Image::default());
                        error_flags.push(true);
                    }
                }
            } else {
                info!("Image {} is empty, adding default image", i);
                // Add empty image for missing data
                slint_images.push(slint::Image::default());
                error_flags.push(true);
            }
        }

        info!("All {} images processed, now setting UI arrays...", slint_images.len());
        main_window.set_aurora_all_sky_images(slint_images.as_slice().into());
        main_window.set_aurora_all_sky_errors(error_flags.as_slice().into());
        info!("Updated Aurora all-sky images array with {} images", slint_images.len());
    } else {
        info!("No Aurora all-sky images available for array display");
    }
    info!("update_all_sky_images_display completed");
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
