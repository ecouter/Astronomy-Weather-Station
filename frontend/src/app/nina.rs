use std::sync::Mutex;
use once_cell::sync::Lazy;
use crate::MainWindow;
use crate::app::utils::decode_png_to_slint_image;

pub static NINA_URLS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(vec![
    "http://localhost:1888".to_string(),
    "http://localhost:1889".to_string(),
    "http://localhost:1890".to_string(),
    "http://localhost:1891".to_string(),
    "http://localhost:1892".to_string(),
    "http://localhost:1893".to_string(),
]));

pub async fn update_nina_images(main_window: &MainWindow) -> Result<(), Box<dyn std::error::Error>> {
    use nina::{fetch_prepared_image, PreparedImageParams};

    println!("Updating Nina images...");

    // Same parameters for all images
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

    // Get base URLs from storage
    let base_urls = {
        let urls = NINA_URLS.lock().unwrap();
        urls.clone()
    };

    // Set URL properties for UI
    if base_urls.len() >= 6 {
        main_window.set_nina_url1(base_urls[0].clone().into());
        main_window.set_nina_url2(base_urls[1].clone().into());
        main_window.set_nina_url3(base_urls[2].clone().into());
        main_window.set_nina_url4(base_urls[3].clone().into());
        main_window.set_nina_url5(base_urls[4].clone().into());
        main_window.set_nina_url6(base_urls[5].clone().into());
    }

    // Fetch images concurrently
    let mut tasks = Vec::new();
    for base_url in base_urls {
        let params = image_params.clone();
        let url = base_url.clone();
        let task = tokio::spawn(async move {
            match fetch_prepared_image(&url, &params).await {
                Ok(data) => Some(data),
                Err(e) => {
                    eprintln!("Failed to fetch Nina prepared image from {}: {}", url, e);
                    None
                }
            }
        });
        tasks.push(task);
    }

    // Wait for all tasks to complete
    let mut images_data = Vec::new();
    for task in tasks {
        if let Ok(Some(data)) = task.await {
            images_data.push(data);
        } else {
            // Add empty data for failed fetches
            images_data.push(Vec::new());
        }
    }

    // Decode and set images (only if we have data)
    if images_data.len() >= 6 {
        for i in 0..6 {
            if !images_data[i].is_empty() {
                match decode_png_to_slint_image(&images_data[i]) {
                    Ok(slint_image) => {
                        match i {
                            0 => main_window.set_nina_image1(slint_image),
                            1 => main_window.set_nina_image2(slint_image),
                            2 => main_window.set_nina_image3(slint_image),
                            3 => main_window.set_nina_image4(slint_image),
                            4 => main_window.set_nina_image5(slint_image),
                            5 => main_window.set_nina_image6(slint_image),
                            _ => {}
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to decode Nina image {}: {}", i + 1, e);
                    }
                }
            } else {
                // Clear the image if no data was fetched
                let empty_image = slint::Image::default();
                match i {
                    0 => main_window.set_nina_image1(empty_image),
                    1 => main_window.set_nina_image2(empty_image),
                    2 => main_window.set_nina_image3(empty_image),
                    3 => main_window.set_nina_image4(empty_image),
                    4 => main_window.set_nina_image5(empty_image),
                    5 => main_window.set_nina_image6(empty_image),
                    _ => {}
                }
            }
        }
    }

    println!("Nina images updated successfully");
    Ok(())
}
