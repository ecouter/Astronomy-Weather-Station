use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::sync::Arc;
use slint::ComponentHandle;
use crate::MainWindow;
use crate::app::utils::decode_png_to_slint_image;

// Store websocket stop senders for each NINA slot (0-5)
pub static NINA_WEBSOCKET_SENDERS: Lazy<Mutex<Vec<Option<Arc<Mutex<Option<std::sync::mpsc::Sender<()>>>>>>>> = Lazy::new(|| {
    Mutex::new(vec![None; 6])
});

// Store guiding graph thread handles and stop senders for each NINA slot (0-5)
pub static NINA_GUIDING_THREADS: Lazy<Mutex<Vec<Option<(std::thread::JoinHandle<()>, std::sync::mpsc::Sender<()>)>>>> = Lazy::new(|| {
    let mut vec = Vec::with_capacity(6);
    for _ in 0..6 {
        vec.push(None);
    }
    Mutex::new(vec)
});

/// Fetch and update a single NINA image for the given slot
async fn update_single_nina_image(main_window: &MainWindow, slot_index: usize, base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
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
            match decode_png_to_slint_image(&image_data) {
                Ok(slint_image) => {
                    debug!("Successfully decoded PNG to Slint image for slot {}", slot_index + 1);
                    match slot_index {
                        0 => {
                            main_window.set_nina_image1(slint_image);
                            info!("Updated NINA image for slot 1");
                        }
                        1 => {
                            main_window.set_nina_image2(slint_image);
                            info!("Updated NINA image for slot 2");
                        }
                        2 => {
                            main_window.set_nina_image3(slint_image);
                            info!("Updated NINA image for slot 3");
                        }
                        3 => {
                            main_window.set_nina_image4(slint_image);
                            info!("Updated NINA image for slot 4");
                        }
                        4 => {
                            main_window.set_nina_image5(slint_image);
                            info!("Updated NINA image for slot 5");
                        }
                        5 => {
                            main_window.set_nina_image6(slint_image);
                            info!("Updated NINA image for slot 6");
                        }
                        _ => {
                            warn!("Invalid slot index {} for NINA image update", slot_index);
                        }
                    }
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

/// Fetch and update a single NINA guiding graph for the given slot
async fn update_single_nina_guiding_graph(main_window: &MainWindow, slot_index: usize, base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    use nina::{fetch_guiding_graph, generate_guiding_graph_png};

    info!("Starting guiding graph update for NINA slot {} from URL: {}", slot_index + 1, base_url);

    match fetch_guiding_graph(base_url).await {
        Ok(graph_data) => {
            info!("Successfully fetched guiding graph data for slot {}", slot_index + 1);
            match generate_guiding_graph_png(&graph_data, slot_index) {
                Ok(png_data) => {
                    info!("Successfully generated {} bytes of guiding graph PNG for slot {}", png_data.len(), slot_index + 1);
                    match decode_png_to_slint_image(&png_data) {
                        Ok(slint_image) => {
                            debug!("Successfully decoded guiding graph PNG to Slint image for slot {}", slot_index + 1);
                            match slot_index {
                                0 => {
                                    main_window.set_nina_guiding_image1(slint_image);
                                    info!("Updated NINA guiding graph for slot 1");
                                }
                                1 => {
                                    main_window.set_nina_guiding_image2(slint_image);
                                    info!("Updated NINA guiding graph for slot 2");
                                }
                                2 => {
                                    main_window.set_nina_guiding_image3(slint_image);
                                    info!("Updated NINA guiding graph for slot 3");
                                }
                                3 => {
                                    main_window.set_nina_guiding_image4(slint_image);
                                    info!("Updated NINA guiding graph for slot 4");
                                }
                                4 => {
                                    main_window.set_nina_guiding_image5(slint_image);
                                    info!("Updated NINA guiding graph for slot 5");
                                }
                                5 => {
                                    main_window.set_nina_guiding_image6(slint_image);
                                    info!("Updated NINA guiding graph for slot 6");
                                }
                                _ => {
                                    warn!("Invalid slot index {} for NINA guiding graph update", slot_index);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to decode NINA guiding graph for slot {}: {}", slot_index + 1, e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to generate guiding graph PNG for slot {}: {}", slot_index + 1, e);
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch guiding graph data from {} for slot {}: {}", base_url, slot_index + 1, e);
        }
    }

    Ok(())
}

/// Start websocket listener for a NINA slot
pub fn start_nina_websocket(slot_index: usize, base_url: String, main_window: &MainWindow) {
    info!("Starting websocket listener for NINA slot {} with URL: {}", slot_index + 1, base_url);

    // Stop existing websocket for this slot if any
    stop_nina_websocket(slot_index);

    let main_window_weak = main_window.as_weak();
    let base_url_clone = base_url.clone();

    // Create the callback that will be called when an image-prepared event is received
    let on_image_prepared = move |event: nina::ImagePreparedEvent| {
        info!("Received image prepared event for slot {}: event={}",
              slot_index + 1,
              event.event);

        let main_window_weak = main_window_weak.clone();
        let base_url = base_url_clone.clone();

        // Spawn a task to handle the image update in the UI thread
        match slint::invoke_from_event_loop(move || {
            debug!("Invoking UI update for NINA slot {} in event loop", slot_index + 1);
            let rt = tokio::runtime::Runtime::new().unwrap();
            let window = main_window_weak.upgrade();
            if let Some(window) = window {
                rt.block_on(async {
                    if let Err(e) = update_single_nina_image(&window, slot_index, &base_url).await {
                        error!("Failed to update NINA image for slot {}: {}", slot_index + 1, e);
                    }
                });
            } else {
                warn!("Failed to upgrade main window weak reference for slot {}", slot_index + 1);
            }
        }) {
            Ok(_) => debug!("Successfully invoked UI update for slot {}", slot_index + 1),
            Err(e) => error!("Failed to invoke UI update for slot {}: {}", slot_index + 1, e),
        }
    };

    // Spawn the websocket listener
    let (_handle, sender) = nina::spawn_nina_websocket_listener(base_url, on_image_prepared);

    // Store the sender
    {
        let mut senders = NINA_WEBSOCKET_SENDERS.lock().unwrap();
        senders[slot_index] = Some(Arc::new(Mutex::new(Some(sender))));
        info!("Stored websocket stop sender for NINA slot {}", slot_index + 1);
    }

    info!("Started websocket listener for NINA slot {}", slot_index + 1);
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

/// Start guiding graph polling thread for a NINA slot
pub fn start_nina_guiding_thread(slot_index: usize, base_url: String, main_window: &MainWindow) {
    info!("Starting guiding graph polling thread for NINA slot {} with URL: {}", slot_index + 1, base_url);

    // Stop existing guiding thread for this slot if any
    stop_nina_guiding_thread(slot_index);

    let main_window_weak = main_window.as_weak();
    let base_url_clone = base_url.clone();
    let (stop_tx, stop_rx) = std::sync::mpsc::channel();

    let handle = std::thread::spawn(move || {
        info!("Guiding graph polling thread started for slot {}", slot_index + 1);
        let rt = tokio::runtime::Runtime::new().unwrap();

        loop {
            // Check if stop signal received
            if stop_rx.try_recv().is_ok() {
                info!("Stop signal received, stopping guiding graph polling thread for slot {}", slot_index + 1);
                break;
            }

            // Update guiding graph
            let main_window_weak = main_window_weak.clone();
            let base_url = base_url_clone.clone();

            match slint::invoke_from_event_loop(move || {
                debug!("Invoking guiding graph update for NINA slot {} in event loop", slot_index + 1);
                let rt = tokio::runtime::Runtime::new().unwrap();
                let window = main_window_weak.upgrade();
                if let Some(window) = window {
                    rt.block_on(async {
                        if let Err(e) = update_single_nina_guiding_graph(&window, slot_index, &base_url).await {
                            error!("Failed to update NINA guiding graph for slot {}: {}", slot_index + 1, e);
                        }
                    });
                } else {
                    warn!("Failed to upgrade main window weak reference for guiding graph slot {}", slot_index + 1);
                }
            }) {
                Ok(_) => debug!("Successfully invoked guiding graph update for slot {}", slot_index + 1),
                Err(e) => error!("Failed to invoke guiding graph update for slot {}: {}", slot_index + 1, e),
            }

            // Wait 10 seconds before next update
            std::thread::sleep(std::time::Duration::from_secs(10));
        }

        info!("Guiding graph polling thread ended for slot {}", slot_index + 1);
    });

    // Store the handle and sender
    {
        let mut threads = NINA_GUIDING_THREADS.lock().unwrap();
        threads[slot_index] = Some((handle, stop_tx));
        info!("Stored guiding graph thread for NINA slot {}", slot_index + 1);
    }

    info!("Started guiding graph polling thread for NINA slot {}", slot_index + 1);
}

/// Stop guiding graph polling thread for a NINA slot
pub fn stop_nina_guiding_thread(slot_index: usize) {
    info!("Stopping guiding graph polling thread for NINA slot {}", slot_index + 1);
    let mut threads = NINA_GUIDING_THREADS.lock().unwrap();
    if let Some((handle, sender)) = threads[slot_index].take() {
        let _ = sender.send(());
        // Don't wait for the thread to finish, just let it end naturally
        info!("Sent stop signal to guiding graph polling thread for NINA slot {}", slot_index + 1);
    }
}

/// Handle URL change for a NINA slot
pub async fn handle_nina_url_change(slot_index: usize, new_url: String, main_window: &MainWindow) {
    info!("Handling URL change for NINA slot {}: '{}'", slot_index + 1, new_url);

    if new_url.trim().is_empty() {
        info!("Empty URL provided for slot {}, stopping websocket and guiding thread, clearing images", slot_index + 1);
        // Empty URL - stop websocket and guiding thread
        stop_nina_websocket(slot_index);
        stop_nina_guiding_thread(slot_index);
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
    } else {
        info!("Valid URL provided for slot {}, starting websocket and guiding thread", slot_index + 1);
        // Valid URL - start websocket and guiding thread
        start_nina_websocket(slot_index, new_url.clone(), main_window);
        start_nina_guiding_thread(slot_index, new_url, main_window);
    }
}
