extern crate pretty_env_logger;
#[macro_use] extern crate log;

use nina::*;
use plotters::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    // Test plotters functionality
    info!("Testing plotters functionality...");
    test_plotters()?;

    // Generate example guiding graph with made-up data
    info!("Generating example guiding graph...");
    generate_example_graph()?;

    Ok(())
}

fn test_plotters() -> Result<(), Box<dyn std::error::Error>> {
    info!("Testing plotters with manual drawing...");

    // Create test data - simple up and down pattern
    let ra_data: Vec<(f64, f64)> = vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.0), (3.0, 1.0), (4.0, 0.0)];
    let dec_data: Vec<(f64, f64)> = vec![(0.0, 1.0), (1.0, 0.0), (2.0, 1.0), (3.0, 0.0), (4.0, 1.0)];

    info!("RA data: {:?}, Dec data: {:?}", ra_data, dec_data);

    // Use plotters' built-in PNG backend to avoid conversion issues
    {
        let backend = plotters::backend::BitMapBackend::new("plotters_test.png", (800, 800));
        let root_area = backend.into_drawing_area();

        // Fill with white background
        root_area.fill(&WHITE)?;

        // Define chart area with margins
        let chart_area = root_area.margin(50, 50, 50, 50);

        // Calculate scaling factors
        let chart_width = 800.0 - 100.0; // accounting for margins
        let chart_height = 800.0 - 100.0;
        let x_scale = chart_width / 5.0; // 0 to 5 range
        let y_scale = chart_height / 5.0; // -1.5 to 1.5 range (some padding)
        let y_offset = chart_height / 5.0 + 50.0; // center Y and account for top margin

        info!("Chart dimensions: {}x{}, scale factors: x={}, y={}, y_offset={}",
              chart_width, chart_height, x_scale, y_scale, y_offset);

        // Draw RA line (blue) - draw individual segments for guaranteed visibility
        for i in 0..ra_data.len().saturating_sub(1) {
            let (x1, y1) = ra_data[i];
            let (x2, y2) = ra_data[i + 1];

            let screen_x1 = (x1 * x_scale + 50.0) as i32;
            let screen_y1 = (y_offset - y1 * y_scale) as i32;
            let screen_x2 = (x2 * x_scale + 50.0) as i32;
            let screen_y2 = (y_offset - y2 * y_scale) as i32;

            info!("RA segment: ({}, {}) to ({}, {})", screen_x1, screen_y1, screen_x2, screen_y2);

            chart_area.draw(&plotters::element::PathElement::new(
                vec![(screen_x1, screen_y1), (screen_x2, screen_y2)],
                BLUE.stroke_width(8), // Very thick for guaranteed visibility
            ))?;
        }

        // Draw Dec line (red) - draw individual segments for guaranteed visibility
        for i in 0..dec_data.len().saturating_sub(1) {
            let (x1, y1) = dec_data[i];
            let (x2, y2) = dec_data[i + 1];

            let screen_x1 = (x1 * x_scale + 50.0) as i32;
            let screen_y1 = (y_offset - y1 * y_scale) as i32;
            let screen_x2 = (x2 * x_scale + 50.0) as i32;
            let screen_y2 = (y_offset - y2 * y_scale) as i32;

            info!("Dec segment: ({}, {}) to ({}, {})", screen_x1, screen_y1, screen_x2, screen_y2);

            chart_area.draw(&plotters::element::PathElement::new(
                vec![(screen_x1, screen_y1), (screen_x2, screen_y2)],
                RED.stroke_width(8), // Very thick for guaranteed visibility
            ))?;
        }

        // Ensure the drawing is completed
        root_area.present()?;
    }

    // Verify the file was created
    if std::path::Path::new("plotters_test.png").exists() {
        info!("✅ Plotters test successful - saved plotters_test.png");
    } else {
        error!("❌ Plotters test failed - PNG file not created");
        return Err("PNG file not created".into());
    }

    info!("Plotters test completed successfully!");
    Ok(())
}

fn generate_example_graph() -> Result<(), Box<dyn std::error::Error>> {
    info!("Creating example guiding graph with made-up data...");

    // Create made-up RMS data
    let rms = RmsData {
        ra: 1.2,
        dec: 0.8,
        total: 1.4,
        ra_text: "1.2\"".to_string(),
        dec_text: "0.8\"".to_string(),
        total_text: "1.4\"".to_string(),
        peak_ra_text: "2.1\"".to_string(),
        peak_dec_text: "1.8\"".to_string(),
        scale: 1.0,
        peak_ra: 2.1,
        peak_dec: 1.8,
        data_points: 25,
    };

    // Create made-up guide steps with realistic guiding corrections
    let mut guide_steps = Vec::new();

    // Generate 25 guide steps with some realistic guiding behavior
    for i in 0..25 {
        let id = i as u32;

        // Create some realistic guiding corrections that vary over time
        // RA corrections: mostly small with occasional larger corrections
        let ra_distance_raw: f64 = match i % 8 {
            0 => 0.5,
            1 => -0.3,
            2 => 0.8,
            3 => -0.2,
            4 => 1.2,
            5 => -0.6,
            6 => 0.1,
            7 => -0.4,
            _ => 0.0,
        };

        // Dec corrections: similar pattern but slightly different
        let dec_distance_raw: f64 = match (i + 3) % 7 {
            0 => 0.4,
            1 => -0.5,
            2 => 0.7,
            3 => -0.1,
            4 => 0.9,
            5 => -0.3,
            6 => 0.2,
            _ => 0.0,
        };

        // Convert to display values (arcseconds)
        let ra_distance_raw_display = ra_distance_raw * 2.0; // Scale for display
        let dec_distance_raw_display = dec_distance_raw * 2.0;

        // Duration values (in milliseconds) - proportional to correction size
        let ra_duration = (ra_distance_raw.abs() * 200.0) as i32 + 50; // Base 50ms + correction
        let dec_duration = (dec_distance_raw.abs() * 180.0) as i32 + 45;

        // Add some dither points randomly
        let dither = if i == 10 || i == 18 { "dither" } else { "NaN" };

        let step = GuideStep {
            id,
            id_offset_left: id as f64 * 10.0,
            id_offset_right: (id + 1) as f64 * 10.0,
            ra_distance_raw,
            ra_distance_raw_display,
            ra_duration,
            dec_distance_raw,
            dec_distance_raw_display,
            dec_duration,
            dither: dither.to_string(),
        };

        guide_steps.push(step);
    }

    // Create the complete guide steps history
    let graph_data = GuideStepsHistory {
        rms,
        interval: 1000, // 1 second intervals
        max_y: 4,
        min_y: -4,
        max_duration_y: 500,
        min_duration_y: 0,
        guide_steps,
        history_size: 25,
        pixel_scale: 1.5,
        scale: serde_json::Value::Null,
    };

    info!("Created example data with {} guide steps", graph_data.guide_steps.len());

    // Generate the graph
    match generate_guiding_graph_png(&graph_data, 0) {
        Ok(png_data) => {
            info!("✅ Successfully generated example guiding graph! PNG size: {} bytes", png_data.len());

            // Also save to a specific filename for easy viewing
            match std::fs::write("example_guiding_graph.png", &png_data) {
                Ok(_) => info!("Example graph saved as 'example_guiding_graph.png'"),
                Err(e) => error!("Failed to save example graph: {}", e),
            }
        }
        Err(e) => {
            error!("❌ Failed to generate example guiding graph: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
