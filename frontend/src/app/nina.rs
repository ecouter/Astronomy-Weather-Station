use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::task::JoinHandle;
use slint::ComponentHandle;
use crate::MainWindow;
use crate::app::utils::decode_png_to_slint_image;

// Store websocket handles for each NINA slot (0-5)
pub static NINA_WEBSOCKET_HANDLES: Lazy<Mutex<Vec<Option<Arc<Mutex<Option<JoinHandle<()>>>>>>>> = Lazy::new(|| {
    Mutex::new(vec![None; 6])
});

/// Fetch and update a single NINA image for the given slot
async fn update_single_nina_image(main_window: &MainWindow, slot_index: usize, base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    use nina::{fetch_prepared_image, PreparedImageParams};

    let image_params = PreparedImageParams {
        resize: Some(true),
        quality: Some(80),
        size: Some("400x225".to_string()),
        scale: Some(1.0),
        factor: Some(1.0),
        black_clipping: Some(0.0),
        unlinked: Some(false),
        debayer: Some(true),
        bayer_pattern: Some("RGGB".to_string()),
        auto_prepare: Some(true),
    };

    match fetch_prepared_image(base_url, &image_params).await {
        Ok(image_data) => {
            match decode_png_to_slint_image(&image_data) {
                Ok(slint_image) => {
                    match slot_index {
                        0 => main_window.set_nina_image1(slint_image),
                        1 => main_window.set_nina_image2(slint_image),
                        2 => main_window.set_nina_image3(slint_image),
                        3 => main_window.set_nina_image4(slint_image),
                        4 => main_window.set_nina_image5(slint_image),
                        5 => main_window.set_nina_image6(slint_image),
                        _ => {}
                    }
                    info!("Updated NINA image for slot {}", slot_index + 1);
                }
                Err(e) => {
                    error!("Failed to decode NINA image for slot {}: {}", slot_index + 1, e);
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch NINA image from {} for slot {}: {}", base_url, slot_index + 1, e);
        }
    }

    Ok(())
}

/// Start websocket listener for a NINA slot
pub fn start_nina_websocket(slot_index: usize, base_url: String, main_window: &MainWindow) {
    // Stop existing websocket for this slot if any
    stop_nina_websocket(slot_index);

    let main_window_weak = main_window.as_weak();
    let base_url_clone = base_url.clone();

    // Create the callback that will be called when an image-save event is received
    let on_image_save = move |_event: nina::ImageSaveEvent| {
        let main_window_weak = main_window_weak.clone();
        let base_url = base_url_clone.clone();

        // Spawn a task to handle the image update in the UI thread
        slint::invoke_from_event_loop(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let window = main_window_weak.upgrade();
            if let Some(window) = window {
                rt.block_on(async {
                    if let Err(e) = update_single_nina_image(&window, slot_index, &base_url).await {
                        error!("Failed to update NINA image for slot {}: {}", slot_index + 1, e);
                    }
                });
            }
        }).unwrap();
    };

    // Spawn the websocket listener
    let handle = nina::spawn_nina_websocket_listener(base_url, on_image_save);

    // Store the handle
    {
        let mut handles = NINA_WEBSOCKET_HANDLES.lock().unwrap();
        handles[slot_index] = Some(Arc::new(Mutex::new(Some(handle))));
    }

    info!("Started websocket listener for NINA slot {}", slot_index + 1);
}

/// Stop websocket listener for a NINA slot
pub fn stop_nina_websocket(slot_index: usize) {
    info!("Stopping websocket listener for NINA slot {}", slot_index + 1);
    let mut handles = NINA_WEBSOCKET_HANDLES.lock().unwrap();
    if let Some(handle_arc) = handles[slot_index].take() {
        let mut handle_opt = handle_arc.lock().unwrap();
        if let Some(handle) = handle_opt.take() {
            handle.abort();
            info!("Stopped websocket listener for NINA slot {}", slot_index + 1);
        }
    }
}

/// Handle URL change for a NINA slot
pub fn handle_nina_url_change(slot_index: usize, new_url: String, main_window: &MainWindow) {
    if new_url.trim().is_empty() {
        // Empty URL - stop websocket
        stop_nina_websocket(slot_index);
        // Clear the image
        let empty_image = slint::Image::default();
        match slot_index {
            0 => main_window.set_nina_image1(empty_image),
            1 => main_window.set_nina_image2(empty_image),
            2 => main_window.set_nina_image3(empty_image),
            3 => main_window.set_nina_image4(empty_image),
            4 => main_window.set_nina_image5(empty_image),
            5 => main_window.set_nina_image6(empty_image),
            _ => {}
        }
    } else {
        // Valid URL - start websocket
        start_nina_websocket(slot_index, new_url, main_window);
    }
}
