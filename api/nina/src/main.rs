extern crate pretty_env_logger;
#[macro_use] extern crate log;

use nina::{fetch_guiding_graph, fetch_prepared_image, PreparedImageParams, spawn_nina_websocket_listener, ImageSaveEvent};
use std::fs::File;
use std::io::Write;
use std::sync::mpsc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let base_url = "http://10.0.0.152:1888/v2/api";

    /*// Fetch guiding graph
    info!("Fetching guiding graph...");
    match fetch_guiding_graph(base_url).await {
        Ok(graph) => {
            let json = serde_json::to_string_pretty(&graph)?;
            let mut file = File::create("guiding_graph.json")?;
            file.write_all(json.as_bytes())?;
            info!("Saved guiding graph to guiding_graph.json");
        }
        Err(e) => {
            error!("Failed to fetch guiding graph: {}", e);
        }
    }

    // Fetch prepared image
    info!("Fetching prepared image...");
    let params = PreparedImageParams {
        auto_prepare: Some(true), // Get exactly what's shown in NINA
        ..Default::default()
    };
    match fetch_prepared_image(base_url, &params).await {
        Ok(image_bytes) => {
            let base64_image = base64::encode(&image_bytes);
            let image_json = serde_json::json!({
                "image_base64": base64_image,
                "size_bytes": image_bytes.len()
            });
            let json = serde_json::to_string_pretty(&image_json)?;
            let mut file = File::create("prepared_image.json")?;
            file.write_all(json.as_bytes())?;
            info!("Saved prepared image to prepared_image.json");
        }
        Err(e) => {
            error!("Failed to fetch prepared image: {}", e);
        }
    }*/
    
    // Test websocket event reception
    info!("Testing websocket event reception...");
    test_websocket_events(base_url).await?;

    Ok(())
}

async fn test_websocket_events(base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel();

    // Callback that sends events to our test channel
    let on_image_save = move |event: ImageSaveEvent| {
        info!("Received IMAGE-SAVE event: camera={}, filter={}, exposure={}",
              event.image_statistics.camera_name,
              event.image_statistics.filter,
              event.image_statistics.exposure_time);
        let _ = tx.send(event);
    };

    info!("Starting websocket listener for event testing...");
    let (_handle, stop_sender) = spawn_nina_websocket_listener(base_url.to_string(), on_image_save);

    info!("Websocket listener started. Waiting for IMAGE-SAVE events...");
    info!("Please save an image in NINA to test event reception.");
    info!("Waiting up to 60 seconds for an event...");

    // Wait for an event with timeout
    match rx.recv_timeout(Duration::from_secs(60)) {
        Ok(event) => {
            info!("✅ SUCCESS: Received IMAGE-SAVE event!");
            info!("Event details: camera={}, filter={}, exposure={:.1}s",
                  event.image_statistics.camera_name,
                  event.image_statistics.filter,
                  event.image_statistics.exposure_time);
            info!("Websocket subscription is working correctly!");
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            error!("❌ FAILURE: No IMAGE-SAVE events received within 60 seconds");
            error!("This indicates the websocket subscription is not working properly");
            error!("Possible issues:");
            error!("  - Wrong subscription message format");
            error!("  - NINA websocket server not sending events");
            error!("  - Network/firewall issues");
            return Err("No events received - subscription failed".into());
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            error!("❌ FAILURE: Event channel disconnected unexpectedly");
            return Err("Event channel disconnected".into());
        }
    }

    // Clean up
    info!("Stopping websocket listener...");
    let _ = stop_sender.send(());

    Ok(())
}
