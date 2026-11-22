use std::time::Duration;
use std::thread;
use std::sync::mpsc;
use std::sync::Mutex;
use std::rc::Rc;
use once_cell::sync::Lazy;
use chrono::{Timelike, Datelike};
use slint::Image;

pub fn decode_png_to_slint_image(image_data: &[u8]) -> Result<slint::Image, Box<dyn std::error::Error>> {
    use image::ImageFormat;

    // Auto-detect the image format and decode
    let img = image::load_from_memory(image_data)?;

    // Convert to RGBA8 format
    let rgba_img = img.to_rgba8();

    // Get dimensions
    let width = rgba_img.width() as u32;
    let height = rgba_img.height() as u32;

    // Convert to raw pixel data (RGBA format)
    let raw_pixels: Vec<u8> = rgba_img.into_raw();

    // Create Slint image from the pixel buffer (RGBA format)
    let pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&raw_pixels, width, height);
    Ok(slint::Image::from_rgba8(pixel_buffer))
}

pub fn decode_png_to_slint_image_with_black_transparency(image_data: &[u8]) -> Result<slint::Image, Box<dyn std::error::Error>> {
    use image::ImageFormat;

    // Auto-detect the image format and decode
    let img = image::load_from_memory(image_data)?;

    // Convert to RGBA8 format
    let mut rgba_img = img.to_rgba8();

    // Get dimensions
    let width = rgba_img.width() as u32;
    let height = rgba_img.height() as u32;

    // Process pixels: make them white with transparency based on original brightness
    // Formula: transparency = max_value - pixel_value
    // Since pixels are grayscale (R=G=B), use any channel as the brightness value
    for pixel in rgba_img.pixels_mut() {
        let brightness = pixel[0]; // Use red channel as brightness (assuming R=G=B)

        // Set pixel to white (255, 255, 255)
        pixel[0] = 255; // R
        pixel[1] = 255; // G
        pixel[2] = 255; // B

        // Set alpha based on brightness: transparency = 255 - brightness
        pixel[3] = brightness;
    }

    // Convert to raw pixel data (RGBA format)
    let raw_pixels: Vec<u8> = rgba_img.into_raw();

    // Create Slint image from the pixel buffer (RGBA format)
    let pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&raw_pixels, width, height);
    Ok(slint::Image::from_rgba8(pixel_buffer))
}

pub fn decode_gif_to_slint_image(gif_data: &[u8]) -> Result<slint::Image, Box<dyn std::error::Error>> {
    use std::io::Cursor;

    // Decode the GIF data
    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = decoder.read_info(Cursor::new(gif_data))?;

    // Read the first frame
    if let Some(frame) = decoder.read_next_frame()? {
        // Get dimensions
        let width = frame.width as u32;
        let height = frame.height as u32;

        // The frame buffer contains RGBA data
        let raw_pixels = frame.buffer.clone();

        // Create Slint image from the pixel buffer (RGBA format)
        let pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&raw_pixels, width, height);
        Ok(slint::Image::from_rgba8(pixel_buffer))
    } else {
        Err("No frames in GIF".into())
    }
}

pub fn resize_png_if_needed(image_data: Vec<u8>, max_width: u32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let img = image::load_from_memory(&image_data)?;

    let width = img.width();

    if width > max_width {
        let height = img.height();
        let new_width = max_width;
        let new_height = (height as f32 * (max_width as f32 / width as f32)) as u32;

        let resized = img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);

        let mut buffer = Vec::new();
        resized.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageFormat::Png)?;

        Ok(buffer)
    } else {
        Ok(image_data)
    }
}

pub fn blend_images(image1_data: &[u8], image2_data: &[u8], weight1: f32, weight2: f32) -> Result<slint::Image, Box<dyn std::error::Error>> {
    use image::ImageFormat;

    // Decode both PNG images
    let img1 = image::load_from_memory_with_format(image1_data, ImageFormat::Png)?;
    let img2 = image::load_from_memory_with_format(image2_data, ImageFormat::Png)?;

    // Convert to RGBA8 format
    let rgba1 = img1.to_rgba8();
    let rgba2 = img2.to_rgba8();

    // Ensure images have the same dimensions
    let width = rgba1.width();
    let height = rgba1.height();
    if rgba2.width() != width || rgba2.height() != height {
        return Err("Images must have the same dimensions for blending".into());
    }

    // Get raw pixel data
    let pixels1 = rgba1.into_raw();
    let pixels2 = rgba2.into_raw();

    // Create blended pixel data
    let mut blended_pixels = Vec::with_capacity(pixels1.len());

    for (p1, p2) in pixels1.chunks(4).zip(pixels2.chunks(4)) {
        // Blend each RGBA component
        let r = (p1[0] as f32 * weight1 + p2[0] as f32 * weight2) as u8;
        let g = (p1[1] as f32 * weight1 + p2[1] as f32 * weight2) as u8;
        let b = (p1[2] as f32 * weight1 + p2[2] as f32 * weight2) as u8;
        let a = (p1[3] as f32 * weight1 + p2[3] as f32 * weight2) as u8;

        blended_pixels.extend_from_slice(&[r, g, b, a]);
    }

    // Create Slint image from blended pixel buffer
    let pixel_buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&blended_pixels, width as u32, height as u32);
    Ok(slint::Image::from_rgba8(pixel_buffer))
}

pub fn calculate_env_canada_forecast_time(model_run_str: &str, index: usize, is_seeing: bool) -> String {
    // Parse the UTC hour from model run string (format: YYYYMMDDHH)
    if let Ok(utc_hour) = &model_run_str[8..10].parse::<u32>() {
        // Calculate the forecast hour offset
        let hour_offset = if is_seeing {
            (index + 1) * 3 // Seeing forecasts are every 3 hours
        } else {
            index + 1 // Other forecasts are every 1 hour
        };

        // Add the offset to the model run hour
        let forecast_utc_hour = (*utc_hour + hour_offset as u32) % 24;

        // Create a datetime for today at the forecast UTC hour
        let now = chrono::Utc::now();
        let forecast_utc_datetime = chrono::NaiveDateTime::new(
            now.date_naive(),
            chrono::NaiveTime::from_hms_opt(forecast_utc_hour, 0, 0).unwrap()
        );

        // Convert to UTC DateTime and then to local time
        let utc_datetime = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(forecast_utc_datetime, chrono::Utc);
        let local_datetime = utc_datetime.with_timezone(&chrono::Local);

        // Format as H:MM AM/PM
        let local_hour = local_datetime.hour();
        let am_pm = if local_hour >= 12 { "PM" } else { "AM" };
        let display_hour = if local_hour == 0 {
            12
        } else if local_hour > 12 {
            local_hour - 12
        } else {
            local_hour
        };

        format!("{}:00 {}", display_hour, am_pm)
    } else {
        // Fallback if parsing fails
        if is_seeing {
            format!("+{}h", (index + 1) * 3)
        } else {
            format!("+{}h", index + 1)
        }
    }
}

pub fn parse_hour(time_str: &str) -> Result<u32, Box<dyn std::error::Error>> {
    // Parse time like "18:11" to get hour
    let hour_str = time_str.split(':').next().ok_or("Invalid time format")?;
    Ok(hour_str.parse()?)
}

/// Combine the rain and snow radar composites into one map. The model's
/// DominantPrecipType layer decides, per pixel, which radar is appropriate:
/// liquid types use the rain radar, frozen types use the snow radar. Where the
/// model has no data (or neither radar has signal) the other radar is used as
/// a fallback so precipitation is never silently dropped.
pub fn blend_radar_by_type(
    rain_data: &[u8],
    snow_data: &[u8],
    type_data: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use image::{ImageFormat, RgbaImage};

    let rain = image::load_from_memory_with_format(rain_data, ImageFormat::Png)?.to_rgba8();
    let snow = image::load_from_memory_with_format(snow_data, ImageFormat::Png)?.to_rgba8();
    let types = image::load_from_memory_with_format(type_data, ImageFormat::Png)?.to_rgba8();

    let (width, height) = rain.dimensions();
    if snow.dimensions() != (width, height) || types.dimensions() != (width, height) {
        return Err("radar and precipitation-type images must have the same dimensions".into());
    }

    // Colours from the GeoMet DominantPrecipType legend.
    const FROZEN: &[[u8; 3]] = &[
        [166, 206, 238], // Snow
        [233, 127, 211], // Ice Pellets
        [249, 136, 36],  // Freezing Drizzle
        [238, 94, 62],   // Freezing Rain
    ];
    const LIQUID: &[[u8; 3]] = &[
        [188, 231, 190], // Rain
        [249, 225, 118], // Drizzle
    ];

    fn matches(colour: [u8; 3], palette: &[[u8; 3]]) -> bool {
        palette.iter().any(|c| {
            (colour[0] as i16 - c[0] as i16).abs() <= 12
                && (colour[1] as i16 - c[1] as i16).abs() <= 12
                && (colour[2] as i16 - c[2] as i16).abs() <= 12
        })
    }

    let rp = rain.as_raw();
    let sp = snow.as_raw();
    let tp = types.as_raw();
    let mut out = vec![0u8; rp.len()];

    for idx in 0..(width as usize * height as usize) {
        let o = idx * 4;
        let rain_alpha = rp[o + 3];
        let snow_alpha = sp[o + 3];

        let use_snow = if tp[o + 3] > 0 {
            let colour = [tp[o], tp[o + 1], tp[o + 2]];
            if matches(colour, FROZEN) {
                true
            } else if matches(colour, LIQUID) {
                false
            } else {
                snow_alpha > rain_alpha
            }
        } else {
            snow_alpha > rain_alpha
        };

        let (primary, fallback) = if use_snow {
            (&sp[o..o + 4], &rp[o..o + 4])
        } else {
            (&rp[o..o + 4], &sp[o..o + 4])
        };

        if primary[3] > 0 {
            out[o..o + 4].copy_from_slice(primary);
        } else {
            out[o..o + 4].copy_from_slice(fallback);
        }
    }

    let blended = RgbaImage::from_raw(width, height, out)
        .ok_or("failed to build blended radar image")?;
    let mut bytes = Vec::new();
    blended.write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::Png)?;
    Ok(bytes)
}
