use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sky quality information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkyQuality {
    /// Sky magnitude
    pub magnitude: String,
    /// Bortle class
    pub bortle_class: String,
    /// Brightness information
    pub brightness: Vec<String>,
    /// Artificial brightness information
    pub artif_brightness: Vec<String>,
}

/// General information about the forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralInfo {
    /// Last generation information
    pub last_gen: LastGenInfo,
    /// Forecast period
    pub forecast: ForecastPeriod,
    /// Timezone
    pub timezone: String,
}

/// Last generation timestamp information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastGenInfo {
    /// Date of last generation
    pub date: String,
    /// Time of last generation
    pub time: String,
}

/// Forecast period information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastPeriod {
    /// Starting day of forecast
    pub from_day: String,
    /// Ending day of forecast
    pub to_day: String,
}

/// Moon information for a specific day
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoonInfo {
    /// Moon rise time
    pub rise: String,
    /// Moon set time
    pub set: String,
    /// Moon phase information
    pub phase: MoonPhase,
}

/// Moon phase information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoonPhase {
    /// Phase name
    pub name: String,
    /// Phase percentage
    pub percentage: String,
}

/// Sunlight/twilight information for a specific day
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SunlightInfo {
    /// Sunrise time
    pub rise: String,
    /// Sunset time
    pub set: String,
    /// Solar transit time
    pub transit: String,
    /// Civil dawn/dusk times
    pub civil_dark: Vec<String>,
    /// Nautical dawn/dusk times
    pub nautical_dark: Vec<String>,
    /// Astronomical dawn/dusk times
    pub astro_dark: Vec<String>,
}

/// Hourly weather data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyData {
    /// Weather conditions
    pub conditions: String,
    /// Total cloud cover percentage
    pub total_clouds: String,
    /// Low cloud cover percentage
    pub low_clouds: String,
    /// Mid cloud cover percentage
    pub mid_clouds: String,
    /// High cloud cover percentage
    pub high_clouds: String,
    /// Visibility in kilometers
    pub visibility: String,
    /// Fog information
    pub fog: String,
    /// Precipitation type
    pub prec_type: String,
    /// Precipitation probability
    pub prec_probability: String,
    /// Precipitation amount
    pub prec_amount: String,
    /// Wind information
    pub wind: WindInfo,
    /// Frost information
    pub frost: String,
    /// Temperature information
    pub temperature: TemperatureInfo,
    /// Relative humidity percentage
    pub rel_humidity: String,
    /// Atmospheric pressure
    pub pressure: String,
    /// Ozone level
    pub ozone: String,
}

/// Wind information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindInfo {
    /// Wind speed in km/h
    pub speed: String,
    /// Wind direction
    pub direction: String,
}

/// Temperature information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureInfo {
    /// General temperature
    pub general: String,
    /// Feels-like temperature
    pub feels_like: String,
    /// Dew point temperature
    pub dew_point: String,
}

/// Information for a single day in the forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayInfo {
    /// Date information
    pub date: DateInfo,
    /// Sun information
    pub sun: SunlightInfo,
    /// Moon information
    pub moon: MoonInfo,
    /// Hourly data for this day
    pub hours: HashMap<String, HourlyData>,
}

/// Date information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateInfo {
    /// Long date format
    pub long: String,
    /// Short date format
    pub short: String,
}

/// Main forecast structure containing all data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearOutsideForecast {
    /// General information
    pub gen_info: GeneralInfo,
    /// Sky quality information
    pub sky_quality: SkyQuality,
    /// Forecast data organized by day
    pub forecast: HashMap<String, DayInfo>,
}

/// ClearOutside API client
pub struct ClearOutsideAPI {
    /// Base URL for the API
    url: String,
    /// HTML content
    html_content: String,
}

impl ClearOutsideAPI {
    /// Create a new ClearOutsideAPI instance
    ///
    /// # Arguments
    /// * `lat` - Latitude with 2 decimal places
    /// * `long` - Longitude with 2 decimal places
    /// * `view` - View type: "current", "midday", or "midnight"
    pub async fn new(lat: &str, long: &str, view: Option<&str>) -> Result<Self, anyhow::Error> {
        if long.len() < 4 || lat.len() < 4 {
            return Err(anyhow::anyhow!("Parameter long or lat is badly specified"));
        }

        let view = view.unwrap_or("midday");
        let url = format!("https://clearoutside.com/forecast/{}/{}?view={}", lat, long, view);

        let client = reqwest::Client::new();
        let response = client.get(&url).send().await?;
        let html_content = response.text().await?;

        Ok(Self { url, html_content })
    }

    /// Update weather information by fetching new data
    pub async fn update(&mut self) -> Result<(), anyhow::Error> {
        let client = reqwest::Client::new();
        let response = client.get(&self.url.clone()).send().await?;
        self.html_content = response.text().await?;
        Ok(())
    }

    /// Parse and extract forecast data from the HTML
    pub fn pull(&self) -> Result<ClearOutsideForecast, anyhow::Error> {
        let document = scraper::Html::parse_document(&self.html_content);
        let content_selector = scraper::Selector::parse("div.container.content").unwrap();

        let content = document
            .select(&content_selector)
            .next()
            .ok_or_else(|| anyhow::anyhow!("Could not find content container"))?;

        // Parse sky quality information
        let sky_quality = self.parse_sky_quality(&content)?;

        // Parse general information
        let gen_info = self.parse_general_info(&content)?;

        // Parse forecast data
        let forecast = self.parse_forecast(&content)?;

        Ok(ClearOutsideForecast {
            gen_info,
            sky_quality,
            forecast,
        })
    }

    /// Parse sky quality information
    fn parse_sky_quality(&self, content: &scraper::ElementRef) -> Result<SkyQuality, anyhow::Error> {
        let span_selector = scraper::Selector::parse("span.btn").unwrap();

        let skyq_element = content
            .select(&span_selector)
            .next()
            .ok_or_else(|| anyhow::anyhow!("Could not find sky quality element"))?;

        let text = skyq_element.text().collect::<Vec<_>>().join(" ");

        // Parse the text similar to Python version
        let parts: Vec<&str> = text.split(": ").collect();
        if parts.len() < 2 {
            return Err(anyhow::anyhow!("Invalid sky quality format"));
        }

        let data_part = parts[1];
        let sections: Vec<&str> = data_part.split(". ").collect();

        if sections.len() < 4 {
            return Err(anyhow::anyhow!("Not enough sky quality sections"));
        }

        let mut skyq_raw = Vec::new();
        for section in sections.iter().take(4) {
            let cleaned = section.replace("\u{a0}", "");
            let parts: Vec<String> = cleaned.split_whitespace().map(|s| s.to_string()).collect();
            skyq_raw.push(parts);
        }

        Ok(SkyQuality {
            magnitude: skyq_raw[0][0].clone(),
            bortle_class: skyq_raw[1][1].clone(),
            brightness: vec![skyq_raw[2][0].clone(), skyq_raw[2][1].clone()],
            artif_brightness: vec![
                skyq_raw[3][0].clone(),
                skyq_raw[3][1].replace("\u{3bc}", "").to_string()
            ],
        })
    }

    /// Parse general information
    fn parse_general_info(&self, content: &scraper::ElementRef) -> Result<GeneralInfo, anyhow::Error> {
        let h2_selector = scraper::Selector::parse("h2").unwrap();

        let h2_element = content
            .select(&h2_selector)
            .next()
            .ok_or_else(|| anyhow::anyhow!("Could not find h2 element"))?;

        let text = h2_element.text().collect::<Vec<_>>().join(" ");

        // Parse similar to Python version
        let parts: Vec<&str> = text.split(". ").collect();

        if parts.len() < 3 {
            return Err(anyhow::anyhow!("Invalid general info format"));
        }

        let geninfo_raw: Vec<Vec<&str>> = parts.iter()
            .map(|part| part.split_whitespace().collect())
            .collect();

        Ok(GeneralInfo {
            last_gen: LastGenInfo {
                date: geninfo_raw[0][1].to_string(),
                time: geninfo_raw[0][2].to_string(),
            },
            forecast: ForecastPeriod {
                from_day: geninfo_raw[1][1].to_string(),
                to_day: geninfo_raw[1][geninfo_raw[1].len() - 1].to_string(),
            },
            timezone: geninfo_raw[geninfo_raw.len() - 1][1].to_string(),
        })
    }

    /// Parse forecast data
    fn parse_forecast(&self, content: &scraper::ElementRef) -> Result<HashMap<String, DayInfo>, anyhow::Error> {
        let fc_selector = scraper::Selector::parse("div.fc").unwrap();
        let fc_day_selector = scraper::Selector::parse("div.fc_day").unwrap();

        let fc_element = content
            .select(&fc_selector)
            .next()
            .ok_or_else(|| anyhow::anyhow!("Could not find forecast container"))?;

        let forecast_days = fc_element.select(&fc_day_selector);
        let mut forecast = HashMap::new();

        for (day_index, day) in forecast_days.enumerate() {
            let day_info = self.parse_day_info(day, day_index)?;
            forecast.insert(format!("day-{}", day_index), day_info);
        }

        Ok(forecast)
    }

    /// Parse information for a single day
    fn parse_day_info(&self, day: scraper::ElementRef, _day_index: usize) -> Result<DayInfo, anyhow::Error> {
        // Parse date information
        let date = self.parse_date_info(day)?;

        // Parse moon information
        let moon = self.parse_moon_info(day)?;

        // Parse sunlight information
        let sun = self.parse_sunlight_info(day)?;

        // Parse hourly data
        let hours = self.parse_hourly_data(day)?;

        Ok(DayInfo {
            date,
            sun,
            moon,
            hours,
        })
    }

    /// Parse date information
    fn parse_date_info(&self, day: scraper::ElementRef) -> Result<DateInfo, anyhow::Error> {
        let date_selector = scraper::Selector::parse("div.fc_day_date").unwrap();

        let date_element = day
            .select(&date_selector)
            .next()
            .ok_or_else(|| anyhow::anyhow!("Could not find date element"))?;

        let text = date_element.text().collect::<Vec<_>>().join(" ");

        // Parse similar to Python version
        let parts: Vec<&str> = text.split_whitespace().collect();

        if parts.len() >= 2 {
            Ok(DateInfo {
                long: parts[0].to_string(),
                short: parts[1].to_string(),
            })
        } else {
            Err(anyhow::anyhow!("Invalid date format"))
        }
    }

    /// Parse moon information
    fn parse_moon_info(&self, day: scraper::ElementRef) -> Result<MoonInfo, anyhow::Error> {
        let moon_selector = scraper::Selector::parse("div.fc_moon").unwrap();
        let phase_selector = scraper::Selector::parse("span.fc_moon_phase").unwrap();
        let percentage_selector = scraper::Selector::parse("span.fc_moon_percentage").unwrap();

        let moon_element = day
            .select(&moon_selector)
            .next()
            .ok_or_else(|| anyhow::anyhow!("Could not find moon element"))?;

        let phase_element = moon_element.select(&phase_selector).next()
            .ok_or_else(|| anyhow::anyhow!("Could not find moon phase element"))?;

        let percentage_element = moon_element.select(&percentage_selector).next()
            .ok_or_else(|| anyhow::anyhow!("Could not find moon percentage element"))?;

        let phase_text = phase_element.text().collect::<Vec<_>>().join(" ");
        let percentage_text = percentage_element.text().collect::<Vec<_>>().join(" ");

        // Parse meridian data from data-content attribute
        let data_content = moon_element.value().attr("data-content")
            .unwrap_or("");

        let parts: Vec<&str> = data_content.split_whitespace().collect();

        let rise = if parts.len() > 7 { parts[parts.len() - 7].to_string() } else { String::new() };
        let set = if parts.len() > 2 { parts[parts.len() - 2].to_string() } else { String::new() };

        Ok(MoonInfo {
            rise,
            set,
            phase: MoonPhase {
                name: phase_text,
                percentage: percentage_text,
            },
        })
    }

    /// Parse sunlight information
    fn parse_sunlight_info(&self, day: scraper::ElementRef) -> Result<SunlightInfo, anyhow::Error> {
        let daylight_selector = scraper::Selector::parse("div.fc_daylight").unwrap();

        let daylight_element = day
            .select(&daylight_selector)
            .next()
            .ok_or_else(|| anyhow::anyhow!("Could not find daylight element"))?;

        let text = daylight_element.text().collect::<Vec<_>>().join(" ");

        // Parse EXACTLY like Python version
        let mut parts: Vec<&str> = text.split('.').collect();

        if parts.len() < 2 {
            return Err(anyhow::anyhow!("Invalid sunlight format"));
        }

        // Remove the second element (index 1) like Python: sunlight_raw_.pop(1)
        if parts.len() > 1 {
            parts.remove(1);
        }

        let mut sunlight_raw = Vec::new();
        for part in parts.iter() {
            // Split by spaces and clean up each item, exactly like Python
            for item in part.split(' ') {
                let cleaned = item.replace(',', "").trim().to_string();
                if !cleaned.is_empty() {
                    sunlight_raw.push(cleaned);
                }
            }
        }

        // Debug output to see the actual structure
        println!("DEBUG: Full sunlight_raw: {:?}", sunlight_raw);

        // Extract times using the EXACT same indices as Python
        let rise = sunlight_raw.get(3).cloned().unwrap_or_default();
        let set = sunlight_raw.get(5).cloned().unwrap_or_default();
        let transit = sunlight_raw.get(7).cloned().unwrap_or_default();

        let civil_dark = vec![
            sunlight_raw.get(10).cloned().unwrap_or_default(),
            sunlight_raw.get(12).cloned().unwrap_or_default(),
        ];

        let nautical_dark = vec![
            sunlight_raw.get(15).cloned().unwrap_or_default(),
            sunlight_raw.get(17).cloned().unwrap_or_default(),
        ];

        let astro_dark = vec![
            sunlight_raw.get(20).cloned().unwrap_or_default(),
            sunlight_raw.get(22).cloned().unwrap_or_default(),
        ];

        Ok(SunlightInfo {
            rise,
            set,
            transit,
            civil_dark,
            nautical_dark,
            astro_dark,
        })
    }

    /// Parse hourly data for a day
    fn parse_hourly_data(&self, day: scraper::ElementRef) -> Result<HashMap<String, HourlyData>, anyhow::Error> {
        let mut hours = HashMap::new();

        // Parse hourly ratings (conditions)
        let hours_raw = self.parse_hourly_ratings(day)?;

        // Parse detailed hourly data
        let details_raw = self.parse_hourly_details(day)?;

        // Combine the data
        for (hour, condition) in hours_raw {
            if let Some(details) = details_raw.get(&hour) {
                // Apply unit conversions like in Python
                let visibility_raw = details.get("visibility").cloned().unwrap_or_else(|| "0".to_string());
                let visibility_km = if let Ok(vis_int) = visibility_raw.parse::<i32>() {
                    format!("{:.2}", (vis_int as f64) * 1.609344)
                } else {
                    "0.0".to_string()
                };

                let wind_speed_raw = details.get("wind_speed").cloned().unwrap_or_else(|| "0".to_string());
                let wind_speed_kmh = if let Ok(speed_int) = wind_speed_raw.parse::<i32>() {
                    format!("{:.2}", (speed_int as f64) * 1.609344)
                } else {
                    "0.0".to_string()
                };

                let hourly_data = HourlyData {
                    conditions: condition,
                    total_clouds: details.get("total_clouds").cloned().unwrap_or_else(|| "0".to_string()),
                    low_clouds: details.get("low_clouds").cloned().unwrap_or_else(|| "0".to_string()),
                    mid_clouds: details.get("mid_clouds").cloned().unwrap_or_else(|| "0".to_string()),
                    high_clouds: details.get("high_clouds").cloned().unwrap_or_else(|| "0".to_string()),
                    visibility: visibility_km,
                    fog: details.get("fog").cloned().unwrap_or_else(|| "0".to_string()),
                    prec_type: details.get("prec_type").cloned().unwrap_or_else(|| "none".to_string()),
                    prec_probability: details.get("prec_probability").cloned().unwrap_or_else(|| "0".to_string()),
                    prec_amount: details.get("prec_amount").cloned().unwrap_or_else(|| "0".to_string()),
                    wind: WindInfo {
                        speed: wind_speed_kmh,
                        direction: details.get("wind_direction").cloned().unwrap_or_else(|| "unknown".to_string()),
                    },
                    frost: details.get("frost").cloned().unwrap_or_else(|| "none".to_string()),
                    temperature: TemperatureInfo {
                        general: details.get("temperature").cloned().unwrap_or_else(|| "0".to_string()),
                        feels_like: details.get("feels_like").cloned().unwrap_or_else(|| "0".to_string()),
                        dew_point: details.get("dew_point").cloned().unwrap_or_else(|| "0".to_string()),
                    },
                    rel_humidity: details.get("rel_humidity").cloned().unwrap_or_else(|| "0".to_string()),
                    pressure: details.get("pressure").cloned().unwrap_or_else(|| "0".to_string()),
                    ozone: details.get("ozone").cloned().unwrap_or_else(|| "0".to_string()),
                };
                hours.insert(hour, hourly_data);
            }
        }

        Ok(hours)
    }

    /// Parse hourly ratings (conditions like "good", "bad", etc.)
    fn parse_hourly_ratings(&self, day: scraper::ElementRef) -> Result<HashMap<String, String>, anyhow::Error> {
        let mut ratings = HashMap::new();

        let hours_selector = scraper::Selector::parse("div.fc_hours.fc_hour_ratings").unwrap();
        let li_selector = scraper::Selector::parse("li").unwrap();

        if let Some(hours_element) = day.select(&hours_selector).next() {
            let li_elements = hours_element.select(&li_selector).collect::<Vec<_>>();

            for (_index, li) in li_elements.iter().enumerate() {
                let text = li.text().collect::<String>();
                // Parse similar to Python: x.get_text()[1:].split(" ")
                // The [1:] removes the first character, then split by space
                let processed_text = text.chars().skip(1).collect::<String>();
                let parts: Vec<&str> = processed_text.split_whitespace().collect();

                if parts.len() >= 2 {
                    // First part is hour, second part is condition
                    let hour = parts[0].to_string();
                    let condition = parts[1].to_string().to_lowercase();
                    ratings.insert(hour, condition);
                }
            }
        }

        Ok(ratings)
    }

    /// Parse detailed hourly data (clouds, visibility, temperature, etc.)
    fn parse_hourly_details(&self, day: scraper::ElementRef) -> Result<HashMap<String, HashMap<String, String>>, anyhow::Error> {
        let mut details_raw: Vec<Vec<String>> = Vec::new();

        let detail_selector = scraper::Selector::parse("div.fc_detail.hidden-xs").unwrap();
        let row_selector = scraper::Selector::parse("div.fc_detail_row").unwrap();
        let li_selector = scraper::Selector::parse("li").unwrap();

        if let Some(detail_element) = day.select(&detail_selector).next() {
            let rows = detail_element.select(&row_selector).collect::<Vec<_>>();

            for (row_index, row) in rows.iter().enumerate() {
                let li_elements = row.select(&li_selector).collect::<Vec<_>>();
                let mut row_values: Vec<String> = Vec::new();

                match row_index {
                    4 => {
                        // Skip ISS row (case 4 in Python)
                        continue;
                    }
                    7 => {
                        // Precipitation case - use title attribute
                        for li in li_elements {
                            if let Some(title) = li.value().attr("title") {
                                let processed_title = title.replace(" ", "-").to_lowercase();
                                row_values.push(processed_title);
                            } else {
                                row_values.push("none".to_string());
                            }
                        }
                    }
                    10 => {
                        // Wind case - use class and text
                        for li in li_elements {
                            if let Some(class) = li.value().attr("class") {
                                if class.contains("fc_") {
                                    let direction = class.split("fc_").nth(1).unwrap_or("unknown");
                                    let speed = li.text().collect::<String>().trim().to_string();
                                    row_values.push(format!("{}|{}", direction, speed));
                                } else {
                                    row_values.push("unknown|0".to_string());
                                }
                            } else {
                                row_values.push("unknown|0".to_string());
                            }
                        }
                    }
                    11 => {
                        // Frost case - check class
                        for li in li_elements {
                            if let Some(class) = li.value().attr("class") {
                                if class != "fc_none" {
                                    row_values.push("frost".to_string());
                                } else {
                                    row_values.push("none".to_string());
                                }
                            } else {
                                row_values.push("none".to_string());
                            }
                        }
                    }
                    12 | 13 | 14 => {
                        // Temperature, humidity, pressure cases - direct text
                        for li in li_elements {
                            let text = li.text().collect::<String>().trim().to_string();
                            row_values.push(text);
                        }
                    }
                    _ => {
                        // General case - replace "-" with "0"
                        for li in li_elements {
                            let text = li.text().collect::<String>().trim().to_string();
                            let processed = if text == "-" { "0".to_string() } else { text };
                            row_values.push(processed);
                        }
                    }
                }

                if !row_values.is_empty() {
                    details_raw.push(row_values);
                }
            }
        }

        // Convert the 2D vector to the expected HashMap format
        let mut details = HashMap::new();
        for (row_index, row_values) in details_raw.iter().enumerate() {
            match row_index {
                0 => self.add_hourly_values(row_values, "total_clouds", &mut details),
                1 => self.add_hourly_values(row_values, "low_clouds", &mut details),
                2 => self.add_hourly_values(row_values, "mid_clouds", &mut details),
                3 => self.add_hourly_values(row_values, "high_clouds", &mut details),
                4 => self.add_hourly_values(row_values, "visibility", &mut details),
                5 => self.add_hourly_values(row_values, "fog", &mut details),
                6 => self.add_hourly_values(row_values, "prec_type", &mut details),
                7 => self.add_hourly_values(row_values, "prec_probability", &mut details),
                8 => self.add_hourly_values(row_values, "prec_amount", &mut details),
                9 => self.add_hourly_wind_values(row_values, &mut details),
                10 => self.add_hourly_values(row_values, "frost", &mut details),
                11 => self.add_hourly_values(row_values, "temperature", &mut details),
                12 => self.add_hourly_values(row_values, "feels_like", &mut details),
                13 => self.add_hourly_values(row_values, "dew_point", &mut details),
                14 => self.add_hourly_values(row_values, "rel_humidity", &mut details),
                15 => self.add_hourly_values(row_values, "pressure", &mut details),
                16 => self.add_hourly_values(row_values, "ozone", &mut details),
                _ => {}
            }
        }

        Ok(details)
    }

    /// Helper function to add hourly values to the details map
    fn add_hourly_values(&self, values: &[String], field_name: &str, details: &mut HashMap<String, HashMap<String, String>>) {
        for (i, value) in values.iter().enumerate() {
            let hour = i.to_string();
            details.entry(hour).or_insert_with(HashMap::new).insert(field_name.to_string(), value.clone());
        }
    }

    /// Helper function to add wind values (special parsing needed)
    fn add_hourly_wind_values(&self, values: &[String], details: &mut HashMap<String, HashMap<String, String>>) {
        for (i, value) in values.iter().enumerate() {
            let hour = i.to_string();
            if let Some((direction, speed)) = value.split_once('|') {
                details.entry(hour.clone()).or_insert_with(HashMap::new).insert("wind_direction".to_string(), direction.to_string());
                details.entry(hour).or_insert_with(HashMap::new).insert("wind_speed".to_string(), speed.to_string());
            } else {
                details.entry(hour.clone()).or_insert_with(HashMap::new).insert("wind_direction".to_string(), "unknown".to_string());
                details.entry(hour).or_insert_with(HashMap::new).insert("wind_speed".to_string(), value.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clearoutside_api_creation() {
        // This would require a real API call, so we'll just test the structure
        // In a real implementation, you might use mock data or a test server
        let api_result = ClearOutsideAPI::new("45.50", "-73.57", Some("midday")).await;
        assert!(api_result.is_ok());
    }
}
