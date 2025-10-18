use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;

/// Structure representing a single seeing data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeeingData {
    /// Day of the forecast (e.g., "2025-10-12")
    pub day: String,
    /// Hour of the day (0-23)
    pub hour: u8,
    /// Low cloud cover percentage (0-100)
    pub clouds_low_pct: u8,
    /// Mid cloud cover percentage (0-100)
    pub clouds_mid_pct: u8,
    /// High cloud cover percentage (0-100)
    pub clouds_high_pct: u8,
    /// Seeing quality in arc seconds
    pub seeing_arcsec: f32,
    /// Seeing index 1 (1-5)
    pub index1: u8,
    /// Seeing index 2 (1-5)
    pub index2: u8,
    /// Jet stream speed in m/s (optional, as it's noted >20m/s bad seeing)
    pub jetstream_ms: Option<f32>,
    /// Bad layers bottom height in km
    pub bad_layers_bot_km: Option<f32>,
    /// Bad layers top height in km
    pub bad_layers_top_km: Option<f32>,
    /// Bad layers K per 100m temperature gradient
    pub bad_layers_k_per_100m: Option<f32>,
    /// Ground temperature in Celsius
    pub temp_c: f32,
    /// Relative humidity percentage
    pub humidity_pct: u8,
}

/// Fetch astronomy seeing data from meteoblue API
pub async fn fetch_meteoblue_data(lat: f64, lon: f64) -> Result<Vec<SeeingData>, anyhow::Error> {
    // Build the URL - meteoblue uses N for north, E for east (with positive lon)
    // Format: https://www.meteoblue.com/en/weather/outdoorsports/seeing/{lat}N-{lon}E
    let lat_str = format!("{:.3}", lat);
    let url = if lon >= 0.0 {
        format!("https://www.meteoblue.com/en/weather/outdoorsports/seeing/{}N-{:.3}E", lat_str, lon)
    } else {
        format!("https://www.meteoblue.com/en/weather/outdoorsports/seeing/{}N-{:.3}E", lat_str, -lon)
    };

    // Make HTTP request
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    let html = response.text().await?;

    // Parse HTML and extract data
    let data = parse_seeing_table(&html)?;

    // Save to JSON file
    save_to_json(&data, lat, lon)?;

    Ok(data)
}

/// Parse the seeing table from HTML
pub fn parse_seeing_table(html: &str) -> Result<Vec<SeeingData>, anyhow::Error> {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);
    let table_selector = Selector::parse("table.table-seeing").unwrap();
    let tr_selector = Selector::parse("tr").unwrap(); // Get all rows in table
    let td_selector = Selector::parse("td").unwrap();

    let mut seeing_data = Vec::new();
    let mut current_day = String::new();

    // Find the table
    if let Some(table) = document.select(&table_selector).next() {
        // Iterate through ALL rows in the table
        let rows: Vec<_> = table.select(&tr_selector).collect();

        for (row_idx, row) in rows.iter().enumerate() {
            let class_attr = row.value().attr("class").unwrap_or("");

            // Check for new-day rows (they contain date information)
            if let Some(new_day_td) = row.select(&Selector::parse("td.new-day").unwrap()).next() {
                let day_text = new_day_td.text().collect::<Vec<_>>().join(" ");
                // Extract date from format like "Sun 2025-10-12 sunrise:..."
                if let Some(date_part) = day_text.split_whitespace().find(|s| s.contains('-') && s.len() == 10) {
                    current_day = date_part.to_string();
                }
                continue; // Skip non-data rows
            }

            if class_attr.contains("hour-row") {
                // Skip if we don't have a current day set or it's empty
                if current_day.is_empty() {
                    println!("Skipping row {}: no current day set", row_idx);
                    continue;
                }

                // Extract data cells
                let cells: Vec<String> = row.select(&td_selector)
                    .map(|cell| cell.text()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .trim()
                        .to_string()
                    )
                    .collect();

                if cells.len() >= 14 { // Ensure we have enough cells (13 + celestial body column)
                    // Parse hour from first cell (remove background-color: etc)
                    let hour_text = cells[0].split(',')
                        .next()
                        .unwrap_or(&cells[0])
                        .chars()
                        .take_while(|c| c.is_digit(10))
                        .collect::<String>();
                    let hour: u8 = hour_text.parse().unwrap_or(0);

                    // Parse cloud covers (cells 1-3)
                    let clouds_low_pct: u8 = cells[1].chars()
                        .take_while(|c| c.is_digit(10))
                        .collect::<String>()
                        .parse().unwrap_or(0);

                    let clouds_mid_pct: u8 = cells[2].chars()
                        .take_while(|c| c.is_digit(10))
                        .collect::<String>()
                        .parse().unwrap_or(0);

                    let clouds_high_pct: u8 = cells[3].chars()
                        .take_while(|c| c.is_digit(10))
                        .collect::<String>()
                        .parse().unwrap_or(0);

                    // Parse seeing (cell 4)
                    let seeing_arcsec: f32 = cells[4].parse().unwrap_or(0.0);

                    // Parse indices (cells 5-6)
                    let index1: u8 = cells[5].chars()
                        .take_while(|c| c.is_digit(10))
                        .collect::<String>()
                        .parse().unwrap_or(0);

                    let index2: u8 = cells[6].chars()
                        .take_while(|c| c.is_digit(10))
                        .collect::<String>()
                        .parse().unwrap_or(0);

                    // Parse jet stream (cell 7)
                    let jetstream_ms = if cells[7].contains(' ') {
                        cells[7].split(' ')
                            .next()
                            .and_then(|s| s.parse::<f32>().ok())
                    } else {
                        None
                    };

                    // Parse bad layers (cells 8-10)
                    let bad_layers_bot_km = if cells[8].chars().any(|c| c.is_digit(10)) {
                        cells[8].parse::<f32>().ok()
                    } else { None };

                    let bad_layers_top_km = if cells[9].chars().any(|c| c.is_digit(10)) {
                        cells[9].parse::<f32>().ok()
                    } else { None };

                    let bad_layers_k_per_100m = if cells[10].chars().any(|c| c.is_digit(10)) {
                        cells[10].split('K').next()
                            .and_then(|s| s.trim().parse::<f32>().ok())
                    } else { None };

                    // Parse temperature and humidity (cells 11-12)
                    let temp_c: f32 = cells[11].split(' ')
                        .next()
                        .and_then(|s| s.chars().take_while(|c| c.is_digit(10) || *c == '.' || *c == '-').collect::<String>().parse().ok())
                        .unwrap_or(0.0);

                    let humidity_pct: u8 = cells[12].chars()
                        .take_while(|c| c.is_digit(10))
                        .collect::<String>()
                        .parse().unwrap_or(0);

                    // Add to our data
                    seeing_data.push(SeeingData {
                        day: current_day.clone(),
                        hour,
                        clouds_low_pct,
                        clouds_mid_pct,
                        clouds_high_pct,
                        seeing_arcsec,
                        index1,
                        index2,
                        jetstream_ms,
                        bad_layers_bot_km,
                        bad_layers_top_km,
                        bad_layers_k_per_100m,
                        temp_c,
                        humidity_pct,
                    });
                }
            }
        }
    } else {
        println!("Table not found!");
    }

    Ok(seeing_data)
}

/// Save seeing data to JSON file, grouped by day
fn save_to_json(data: &[SeeingData], lat: f64, lon: f64) -> Result<(), anyhow::Error> {
    use std::collections::BTreeMap;

    // Group data by day
    let mut grouped_data: BTreeMap<String, Vec<&SeeingData>> = BTreeMap::new();
    for item in data {
        grouped_data.entry(item.day.clone()).or_insert_with(Vec::new).push(item);
    }

    // Convert to the final structure
    let mut final_data = serde_json::Map::new();
    for (day, day_items) in grouped_data {
        // Remove the "day" field from each item since it's now the key
        let day_data: Vec<serde_json::Value> = day_items.iter().map(|item| {
            let mut map = serde_json::Map::new();
            map.insert("hour".to_string(), serde_json::json!(item.hour));
            map.insert("clouds_low_pct".to_string(), serde_json::json!(item.clouds_low_pct));
            map.insert("clouds_mid_pct".to_string(), serde_json::json!(item.clouds_mid_pct));
            map.insert("clouds_high_pct".to_string(), serde_json::json!(item.clouds_high_pct));
            map.insert("seeing_arcsec".to_string(), serde_json::json!(item.seeing_arcsec));
            map.insert("index1".to_string(), serde_json::json!(item.index1));
            map.insert("index2".to_string(), serde_json::json!(item.index2));
            map.insert("jetstream_ms".to_string(), serde_json::json!(item.jetstream_ms));
            map.insert("bad_layers_bot_km".to_string(), serde_json::json!(item.bad_layers_bot_km));
            map.insert("bad_layers_top_km".to_string(), serde_json::json!(item.bad_layers_top_km));
            map.insert("bad_layers_k_per_100m".to_string(), serde_json::json!(item.bad_layers_k_per_100m));
            map.insert("temp_c".to_string(), serde_json::json!(item.temp_c));
            map.insert("humidity_pct".to_string(), serde_json::json!(item.humidity_pct));
            serde_json::Value::Object(map)
        }).collect();
        final_data.insert(day, serde_json::Value::Array(day_data));
    }

    let filename = format!("meteoblue_data.json");
    let json_data = serde_json::to_string_pretty(&final_data)?;
    let mut file = File::create(filename)?;
    file.write_all(json_data.as_bytes())?;
    Ok(())
}
