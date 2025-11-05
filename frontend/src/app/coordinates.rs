use std::sync::Mutex;
use once_cell::sync::Lazy;
use crate::MainWindow;

pub static CACHED_COORDINATES: Lazy<Mutex<Option<(f64, f64)>>> = Lazy::new(|| Mutex::new(None));
pub static COORDINATES_POPUP_SHOWN: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

pub fn load_coordinates(main_window: &MainWindow) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    // Check cache first
    {
        let coords = CACHED_COORDINATES.lock().unwrap();
        if let Some(c) = *coords {
            return Ok(c);
        }
    }

    // Try to load from file
    match std::fs::read_to_string("../coordinates.json") {
        Ok(content) => {
            let coords: serde_json::Value = serde_json::from_str(&content)?;
            let lat: f64 = coords["lat"].as_str().unwrap().parse()?;
            let lon: f64 = coords["lon"].as_str().unwrap().parse()?;
            // Cache the coordinates
            {
                let mut cache = CACHED_COORDINATES.lock().unwrap();
                *cache = Some((lat, lon));
            }
            Ok((lat, lon))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Show popup only once
            {
                let mut shown = COORDINATES_POPUP_SHOWN.lock().unwrap();
                if !*shown {
                    *shown = true;
                    main_window.set_show_coordinates_popup(true);
                    main_window.set_coordinates_popup_message("coordinates.json file not found. Please ensure the file exists in the parent directory.".into());
                }
            }
            Err(e.into())
        }
        Err(e) => Err(e.into())
    }
}
