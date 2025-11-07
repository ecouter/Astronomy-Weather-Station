use std::sync::Mutex;
use once_cell::sync::Lazy;
use slint::ComponentHandle;
use crate::MainWindow;
use crate::app::coordinates;
use crate::app::utils::decode_png_to_slint_image;

pub static SOUNDING_LOADING: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

pub fn setup_sounding_callbacks(main_window: &MainWindow) {
    let main_window_weak = main_window.as_weak();
    main_window.on_show_sounding_page(move || {
        let window_weak = main_window_weak.clone();
        // Check if already loading to prevent multiple calls
        {
            let loading = SOUNDING_LOADING.lock().unwrap();
            if *loading {
                info!("Sounding already loading, skipping duplicate call");
                return;
            }
        }

        // Spawn a complete separate thread for the entire operation to avoid UI freezing
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let window = window_weak.upgrade();
                if let Some(window) = window {
                    info!("Sounding page opened, loading sounding image...");
                    if let Err(e) = load_sounding_image(&window).await {
                        error!("Failed to load sounding image: {}", e);
                    }
                }
            });
        });
    });
}

pub async fn load_sounding_image(main_window: &MainWindow) -> Result<(), Box<dyn std::error::Error>> {
    info!("Loading sounding image...");

    // Set loading state
    {
        let mut loading = SOUNDING_LOADING.lock().unwrap();
        *loading = true;
    }
    main_window.set_loading(true);

    // Get coordinates
    let (lat, lon) = match coordinates::load_coordinates(main_window) {
        Ok(coords) => coords,
        Err(e) => {
            error!("Failed to load coordinates: {}", e);
            main_window.set_error_message(format!("Failed to load coordinates: {}", e).into());
            main_window.set_loading(false);
            {
                let mut loading = SOUNDING_LOADING.lock().unwrap();
                *loading = false;
            }
            return Err(e);
        }
    };

    // Call SHARPpy API asynchronously with explicit output file path
    let output_path = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("sounding_gfs.png")
        .to_string_lossy()
        .to_string();

    let result = sharppy::generate_gfs_sounding_async(
        lat,
        lon,
        Some(output_path),
        Some(format!("GFS Sounding - {:.2}N, {:.2}E", lat, lon))
    ).await?;

    // The result is the file path to the PNG
    let png_path = result.trim();

    // Read the PNG file
    let png_data = match std::fs::read(png_path) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to read PNG file {}: {}", png_path, e);
            main_window.set_error_message(format!("Failed to read sounding image: {}", e).into());
            main_window.set_loading(false);
            {
                let mut loading = SOUNDING_LOADING.lock().unwrap();
                *loading = false;
            }
            return Err(e.into());
        }
    };

    // Convert to Slint image
    match decode_png_to_slint_image(&png_data) {
        Ok(slint_image) => {
            main_window.set_sounding_image(slint_image);
            info!("Successfully loaded sounding image");
        }
        Err(e) => {
            error!("Failed to decode PNG: {}", e);
            main_window.set_error_message(format!("Failed to decode sounding image: {}", e).into());
            main_window.set_loading(false);
            {
                let mut loading = SOUNDING_LOADING.lock().unwrap();
                *loading = false;
            }
            return Err(e.into());
        }
    }

    // Clear loading state
    main_window.set_loading(false);
    {
        let mut loading = SOUNDING_LOADING.lock().unwrap();
        *loading = false;
    }

    Ok(())
}
