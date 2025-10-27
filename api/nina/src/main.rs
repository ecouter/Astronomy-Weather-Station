use nina::{fetch_guiding_graph, fetch_prepared_image, PreparedImageParams};
use std::fs::File;
use std::io::Write;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_url = "http://localhost:1888/v2/api";

    // Fetch guiding graph
    println!("Fetching guiding graph...");
    match fetch_guiding_graph(base_url).await {
        Ok(graph) => {
            let json = serde_json::to_string_pretty(&graph)?;
            let mut file = File::create("guiding_graph.json")?;
            file.write_all(json.as_bytes())?;
            println!("Saved guiding graph to guiding_graph.json");
        }
        Err(e) => {
            eprintln!("Failed to fetch guiding graph: {}", e);
        }
    }

    // Fetch prepared image
    println!("Fetching prepared image...");
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
            println!("Saved prepared image to prepared_image.json");
        }
        Err(e) => {
            eprintln!("Failed to fetch prepared image: {}", e);
        }
    }

    Ok(())
}
