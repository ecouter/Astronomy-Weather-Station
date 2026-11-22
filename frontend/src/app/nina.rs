use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::sync::Arc;
use slint::ComponentHandle;
use crate::MainWindow;
use crate::app::refresh::RUNTIME;
use crate::app::utils::decode_png_to_slint_image;

// Store websocket stop senders for each NINA slot (0-5)
pub static NINA_WEBSOCKET_SENDERS: Lazy<Mutex<Vec<Option<Arc<Mutex<Option<std::sync::mpsc::Sender<()>>>>>>>> = Lazy::new(|| {
    Mutex::new(vec![None; 6])
});

// Store guiding graph task handles for each NINA slot (0-5)
pub static NINA_GUIDING_THREADS: Lazy<Mutex<Vec<Option<tokio::task::JoinHandle<()>>>>> = Lazy::new(|| {
    let mut vec = Vec::with_capacity(6);
    for _ in 0..6 {
        vec.push(None);
    }
    Mutex::new(vec)
});

/// Fetch and update a single NINA image for the given slot
pub async fn update_single_nina_image(
    main_window: slint::Weak<MainWindow>,
    slot_index: usize,
    base_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use nina::{fetch_prepared_image, PreparedImageParams};

    info!("Starting image update for NINA slot {} from URL: {}", slot_index + 1, base_url);

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

    debug!("Using image parameters: {:?}", image_params);

    match fetch_prepared_image(base_url, &image_params).await {
        Ok(image_data) => {
            info!("Successfully fetched {} bytes of image data for slot {}", image_data.len(), slot_index + 1);
            main_window
                .upgrade_in_event_loop(move |win| {
                    match decode_png_to_slint_image(&image_data) {
                        Ok(slint_image) => {
                            debug!("Successfully decoded PNG to Slint image for slot {}", slot_index + 1);
                            clear_nina_error_state(&win, slot_index);
                            match slot_index {
                                0 => win.set_nina_image1(slint_image),
                                1 => win.set_nina_image2(slint_image),
                                2 => win.set_nina_image3(slint_image),
                                3 => win.set_nina_image4(slint_image),
                                4 => win.set_nina_image5(slint_image),
                                5 => win.set_nina_image6(slint_image),
                                _ => warn!("Invalid slot index {} for NINA image update", slot_index),
                            }
                            info!("Updated NINA image for slot {}", slot_index + 1);
                        }
                        Err(e) => {
                            error!("Failed to decode NINA image for slot {}: {}", slot_index + 1, e);
                            set_nina_error_state(&win, slot_index, "Failed to decode image data");
                        }
                    }
                })
                .ok();
            Ok(())
        }
        Err(e) => {
            error!("Failed to fetch NINA image from {} for slot {}: {}", base_url, slot_index + 1, e);
            main_window
                .upgrade_in_event_loop(move |win| {
                    set_nina_error_state(&win, slot_index, "Failed to fetch image");
                })
                .ok();
            Err(Box::new(NinaError(format!("Failed to fetch image: {}", e))))
        }
    }
}

/// Fetch and update a single NINA guiding graph for the given slot
async fn update_single_nina_guiding_graph(
    main_window: slint::Weak<MainWindow>,
    slot_index: usize,
    base_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use nina::{fetch_guiding_graph, generate_guiding_graph_png, fetch_guider_info};

    info!("Starting guiding graph update for NINA slot {} from URL: {}", slot_index + 1, base_url);

    let guider_state = match fetch_guider_info(base_url).await {
        Ok(guider_info) => {
            info!("Successfully fetched guider info for slot {}: state='{}'", slot_index + 1, guider_info.state);
            Some(guider_info.state)
        }
        Err(e) => {
            error!("Failed to fetch guider info from {} for slot {}: {}", base_url, slot_index + 1, e);
            None
        }
    };

    if let Some(state) = guider_state {
        let w = main_window.clone();
        w.upgrade_in_event_loop(move |win| {
            match slot_index {
                0 => win.set_nina_guider_state1(state.into()),
                1 => win.set_nina_guider_state2(state.into()),
                2 => win.set_nina_guider_state3(state.into()),
                3 => win.set_nina_guider_state4(state.into()),
                4 => win.set_nina_guider_state5(state.into()),
                5 => win.set_nina_guider_state6(state.into()),
                _ => warn!("Invalid slot index {} for guider state update", slot_index),
            }
        })
        .ok();
    }

    match fetch_guiding_graph(base_url).await {
        Ok(graph_data) => {
            info!("Successfully fetched guiding graph data for slot {}", slot_index + 1);
            match generate_guiding_graph_png(&graph_data, slot_index) {
                Ok(png_data) => {
                    info!("Successfully generated {} bytes of guiding graph PNG for slot {}", png_data.len(), slot_index + 1);
                    let w = main_window.clone();
                    w.upgrade_in_event_loop(move |win| {
                        match decode_png_to_slint_image(&png_data) {
                            Ok(slint_image) => {
                                debug!("Successfully decoded guiding graph PNG to Slint image for slot {}", slot_index + 1);
                                match slot_index {
                                    0 => win.set_nina_guiding_image1(slint_image),
                                    1 => win.set_nina_guiding_image2(slint_image),
                                    2 => win.set_nina_guiding_image3(slint_image),
                                    3 => win.set_nina_guiding_image4(slint_image),
                                    4 => win.set_nina_guiding_image5(slint_image),
                                    5 => win.set_nina_guiding_image6(slint_image),
                                    _ => warn!("Invalid slot index {} for NINA guiding graph update", slot_index),
                                }
                                info!("Updated NINA guiding graph for slot {}", slot_index + 1);
                            }
                            Err(e) => {
                                error!("Failed to decode NINA guiding graph for slot {}: {}", slot_index + 1, e);
                            }
                        }
                    })
                    .ok();
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to generate guiding graph PNG for slot {}: {}", slot_index + 1, e);
                    Err(Box::new(NinaError(format!("Failed to generate guiding graph: {}", e))))
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch guiding graph data from {} for slot {}: {}", base_url, slot_index + 1, e);
            Err(Box::new(NinaError(format!("Failed to fetch guiding graph: {}", e))))
        }
    }
}

use std::fmt;

/// Custom error type for NINA operations
#[derive(Debug)]
pub struct NinaError(pub String);

impl fmt::Display for NinaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for NinaError {}

/// Start websocket listener for a NINA slot
pub fn start_nina_websocket(slot_index: usize, base_url: String, main_window: &MainWindow) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting websocket listener for NINA slot {} with URL: {}", slot_index + 1, base_url);

    // Stop existing websocket for this slot if any
    stop_nina_websocket(slot_index);

    let main_window_weak = main_window.as_weak();
    let base_url_clone = base_url.clone();

    // Create the callback that will be called when an image-prepared event is received.
    // The websocket listener runs on its own thread; hand the actual work to the
    // shared runtime and keep all UI updates on the Slint event loop.
    let on_image_prepared = move |event: nina::ImagePreparedEvent| {
        info!("Received image prepared event for slot {}: event={}",
              slot_index + 1,
              event.event);
        let w = main_window_weak.clone();
        let url = base_url_clone.clone();
        RUNTIME.spawn(async move {
            if let Err(e) = update_single_nina_image(w, slot_index, &url).await {
                error!("Failed to update NINA image for slot {}: {}", slot_index + 1, e);
            }
        });
    };

    // Spawn the websocket listener
    match nina::spawn_nina_websocket_listener(base_url, on_image_prepared) {
        Ok((_handle, sender)) => {
            // Store the sender
            {
                let mut senders = NINA_WEBSOCKET_SENDERS.lock().unwrap();
                senders[slot_index] = Some(Arc::new(Mutex::new(Some(sender))));
                info!("Stored websocket stop sender for NINA slot {}", slot_index + 1);
            }

            info!("Started websocket listener for NINA slot {}", slot_index + 1);
            Ok(())
        }
        Err(e) => {
            error!("Failed to start NINA websocket listener for slot {}: {}", slot_index + 1, e);
            Err(Box::new(NinaError(format!("Failed to establish websocket connection: {}", e))))
        }
    }
}

/// Stop websocket listener for a NINA slot
pub fn stop_nina_websocket(slot_index: usize) {
    info!("Stopping websocket listener for NINA slot {}", slot_index + 1);
    let mut senders = NINA_WEBSOCKET_SENDERS.lock().unwrap();
    if let Some(sender_arc) = senders[slot_index].take() {
        let mut sender_opt = sender_arc.lock().unwrap();
        if let Some(sender) = sender_opt.take() {
            let _ = sender.send(());
            info!("Sent stop signal to websocket listener for NINA slot {}", slot_index + 1);
        }
    }
}

/// Start guiding graph polling task for a NINA slot
pub fn start_nina_guiding_thread(slot_index: usize, base_url: String, main_window: &MainWindow) {
    info!("Starting guiding graph polling task for NINA slot {} with URL: {}", slot_index + 1, base_url);

    stop_nina_guiding_thread(slot_index);

    let main_window_weak = main_window.as_weak();
    let base_url_clone = base_url.clone();

    let handle = RUNTIME.spawn(async move {
        loop {
            let w = main_window_weak.clone();
            let url = base_url_clone.clone();
            if let Err(e) = update_single_nina_guiding_graph(w, slot_index, &url).await {
                error!("Failed to update NINA guiding graph for slot {}: {}", slot_index + 1, e);
            }
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    {
        let mut threads = NINA_GUIDING_THREADS.lock().unwrap();
        threads[slot_index] = Some(handle);
    }

    info!("Started guiding graph task for NINA slot {}", slot_index + 1);
}

/// Stop guiding graph polling task for a NINA slot
pub fn stop_nina_guiding_thread(slot_index: usize) {
    info!("Stopping guiding graph task for NINA slot {}", slot_index + 1);
    let mut threads = NINA_GUIDING_THREADS.lock().unwrap();
    if let Some(handle) = threads[slot_index].take() {
        handle.abort();
    }
}

/// Handle URL change for a NINA slot
pub fn handle_nina_url_change(slot_index: usize, new_url: String, main_window: &MainWindow) -> Result<(), Box<dyn std::error::Error>> {
    info!("Handling URL change for NINA slot {}: '{}'", slot_index + 1, new_url);

    if new_url.trim().is_empty() {
        info!("Empty URL provided for slot {}, stopping websocket and guiding thread, clearing images", slot_index + 1);
        // Empty URL - stop websocket and guiding thread
        stop_nina_websocket(slot_index);
        stop_nina_guiding_thread(slot_index);

        // Clear error state for empty URL
        clear_nina_error_state(main_window, slot_index);

        // Clear the images
        let empty_image = slint::Image::default();
        match slot_index {
            0 => {
                main_window.set_nina_image1(empty_image.clone());
                main_window.set_nina_guiding_image1(empty_image);
                info!("Cleared images for NINA slot 1");
            }
            1 => {
                main_window.set_nina_image2(empty_image.clone());
                main_window.set_nina_guiding_image2(empty_image);
                info!("Cleared images for NINA slot 2");
            }
            2 => {
                main_window.set_nina_image3(empty_image.clone());
                main_window.set_nina_guiding_image3(empty_image);
                info!("Cleared images for NINA slot 3");
            }
            3 => {
                main_window.set_nina_image4(empty_image.clone());
                main_window.set_nina_guiding_image4(empty_image);
                info!("Cleared images for NINA slot 4");
            }
            4 => {
                main_window.set_nina_image5(empty_image.clone());
                main_window.set_nina_guiding_image5(empty_image);
                info!("Cleared images for NINA slot 5");
            }
            5 => {
                main_window.set_nina_image6(empty_image.clone());
                main_window.set_nina_guiding_image6(empty_image);
                info!("Cleared images for NINA slot 6");
            }
            _ => {
                warn!("Invalid slot index {} for clearing NINA images", slot_index);
            }
        }
        return Ok(());
    } else {
        info!("Valid URL provided for slot {}, starting websocket and guiding thread", slot_index + 1);

        // Try to start websocket connection - this now checks for initial connection
        match start_nina_websocket(slot_index, new_url.clone(), main_window) {
            Ok(_) => {
                info!("Successfully started websocket for NINA slot {}", slot_index + 1);
                // Clear any error state
                clear_nina_error_state(main_window, slot_index);
                // Start guiding thread (this doesn't have connection errors to check)
                start_nina_guiding_thread(slot_index, new_url, main_window);
                Ok(())
            }
            Err(e) => {
                error!("Failed to start websocket for NINA slot {}: {}", slot_index + 1, e);
                // Set error state
                set_nina_error_state(main_window, slot_index, "Connection could not be established");
                // Still try to start guiding thread (in case images work even if websocket doesn't)
                start_nina_guiding_thread(slot_index, new_url, main_window);
                return Err(e);
            }
        }
    }
}

/// Clear NINA error state for a specific slot
pub fn clear_nina_error_state(main_window: &MainWindow, slot_index: usize) {
    match slot_index {
        0 => main_window.set_nina_error1("".into()),
        1 => main_window.set_nina_error2("".into()),
        2 => main_window.set_nina_error3("".into()),
        3 => main_window.set_nina_error4("".into()),
        4 => main_window.set_nina_error5("".into()),
        5 => main_window.set_nina_error6("".into()),
        _ => warn!("Invalid slot index {} for clearing NINA error state", slot_index),
    }
}

/// Set NINA error state for a specific slot
pub fn set_nina_error_state(main_window: &MainWindow, slot_index: usize, error_message: &str) {
    match slot_index {
        0 => main_window.set_nina_error1(error_message.into()),
        1 => main_window.set_nina_error2(error_message.into()),
        2 => main_window.set_nina_error3(error_message.into()),
        3 => main_window.set_nina_error4(error_message.into()),
        4 => main_window.set_nina_error5(error_message.into()),
        5 => main_window.set_nina_error6(error_message.into()),
        _ => warn!("Invalid slot index {} for setting NINA error state", slot_index),
    }
}
