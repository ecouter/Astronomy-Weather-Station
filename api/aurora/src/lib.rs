use log::*;
use chrono::{Duration, Timelike, Utc};
use futures::future::join_all;

/// Struct to hold all aurora all-sky images
#[derive(Debug, Clone)]
pub struct AuroraAllSkyImages {
    pub kjell_henriksen_observatory_norway: Vec<u8>,
    pub hankasalmi_finland: Vec<u8>,
    pub yellowknife_canada: Vec<u8>,
    pub athabasca_canada: Vec<u8>,
    pub glacier_national_park_usa: Vec<u8>,
    pub hansville_usa: Vec<u8>,
    pub isle_royale_national_park_usa: Vec<u8>,
    pub heiligenblut_austria: Vec<u8>,
    pub calgary_canada: Vec<u8>,
    pub hobart_australia: Vec<u8>,
}

/// Fetch aurora forecast image for northern hemisphere
pub async fn fetch_aurora_forecast() -> Result<Vec<u8>, anyhow::Error> {
    let url = "https://services.swpc.noaa.gov/images/aurora-forecast-northern-hemisphere.jpg";
    fetch_image(url).await
}

/// Fetch ACE real-time solar wind image
pub async fn fetch_ace_real_time_solar_wind() -> Result<Vec<u8>, anyhow::Error> {
    let url = "https://services.swpc.noaa.gov/images/ace-mag-swepam-2-hour.gif";
    fetch_image(url).await
}

/// Fetch DSCOVR real-time solar wind data image
pub async fn fetch_dscovr_solar_wind() -> Result<Vec<u8>, anyhow::Error> {
    let url = "https://services.swpc.noaa.gov/images/geospace/geospace_1_day.png";
    fetch_image(url).await
}

/// Fetch space weather overview image
pub async fn fetch_space_weather_overview() -> Result<Vec<u8>, anyhow::Error> {
    let url = "https://services.swpc.noaa.gov/images/swx-overview-large.gif";
    fetch_image(url).await
}

/// Fetch ACE EPAM image
pub async fn fetch_ace_epam() -> Result<Vec<u8>, anyhow::Error> {
    let url = "https://services.swpc.noaa.gov/images/ace-epam-p-24-hour.gif";
    fetch_image(url).await
}

/// Fetch Canadian magnetic observatories image
pub async fn fetch_canadian_magnetic() -> Result<Vec<u8>, anyhow::Error> {
    let url = "https://www.spaceweather.gc.ca/generated_plots/summary/plots/stackplot_e.png";
    fetch_image(url).await
}

/// Fetch alerts timeline image
pub async fn fetch_alerts_timeline() -> Result<Vec<u8>, anyhow::Error> {
    let url = "https://services.swpc.noaa.gov/images/notifications-in-effect-timeline.png";
    fetch_image(url).await
}

/// Fetch tonight's aurora forecast image
pub async fn fetch_tonights_aurora_forecast() -> Result<Vec<u8>, anyhow::Error> {
    let url = "https://services.swpc.noaa.gov/experimental/images/aurora_dashboard/tonights_static_viewline_forecast.png";
    fetch_image(url).await
}

/// Fetch tomorrow night's aurora forecast image
pub async fn fetch_tomorrow_aurora_forecast() -> Result<Vec<u8>, anyhow::Error> {
    let url = "https://services.swpc.noaa.gov/experimental/images/aurora_dashboard/tomorrow_nights_static_viewline_forecast.png";
    fetch_image(url).await
}

/// Fetch WSA-ENLIL prediction image
pub async fn fetch_wsa_enlil() -> Result<Vec<u8>, anyhow::Error> {
    let url = "https://services.swpc.noaa.gov/images/animations/enlil/latest.jpg";
    fetch_image(url).await
}

/// Fetch all aurora all-sky images from various locations
pub async fn fetch_all_aurora_images() -> Result<AuroraAllSkyImages, anyhow::Error> {
    // Generate dynamic URL for Heiligenblut, Austria (UTC+1, floored to nearest half-hour)
    let utc_plus_1 = Utc::now() + Duration::hours(1);
    let floored_minute = if utc_plus_1.minute() < 30 { 0 } else { 30 };
    let dt = utc_plus_1.with_minute(floored_minute).unwrap();
    let date_str = dt.format("%Y/%m/%d").to_string();
    let time_str = dt.format("%H%M").to_string();
    let austrian_url = format!("https://www.foto-webcam.eu/webcam/wallackhaus/{}/{}_hd.jpg", date_str, time_str);

    // List of URLs in specified order
    let urls = vec![
        "https://kho.unis.no/SD/pics/cam3a.jpg",  // Kjell Henriksen Observatory, Norway
        "https://www.ursa.fi/yhd/sirius/sivut/kuvat/ImageLastFTP_AllSKY.jpg",  // Hankasalmi, finland
        "https://auroramax.phys.ucalgary.ca/recent/recent_1080p.jpg",  // yellowknife, canada
        "https://autumn.athabascau.ca/magdata/HiQam/allsky/HiQam-resize.jpg",  // Athabasca, canada
        "https://glacier.org/webcam/dark_sky_nps.jpg",  // glacier national park, USA
        "https://skunkbayweather.com/Canon/NightCam.jpg",  // Hansville, USA
        "https://www.nps.gov/webcams-isro/northshore.jpg",  // Isle Royale National park, USA
        &austrian_url,  // Heiligenblut, Austria
        "https://cam01.sci.ucalgary.ca/AllSkyCam/AllSkyCurrentImage.JPG",  // Calgary, canada
        "https://mtwellington-images.hobartcity.com.au/images/platform.jpg",  // Hobart, Australia
    ];

    // Fetch all images concurrently
    let futures: Vec<_> = urls.iter().map(|url| fetch_image(url)).collect();
    let results: Vec<Result<Vec<u8>, _>> = join_all(futures).await;

    // Collect results, propagating any errors
    let images: Vec<Vec<u8>> = results.into_iter().collect::<Result<_, _>>()?;

    // Construct and return the struct
    Ok(AuroraAllSkyImages {
        kjell_henriksen_observatory_norway: images[0].clone(),
        hankasalmi_finland: images[1].clone(),
        yellowknife_canada: images[2].clone(),
        athabasca_canada: images[3].clone(),
        glacier_national_park_usa: images[4].clone(),
        hansville_usa: images[5].clone(),
        isle_royale_national_park_usa: images[6].clone(),
        heiligenblut_austria: images[7].clone(),
        calgary_canada: images[8].clone(),
        hobart_australia: images[9].clone(),
    })
}

/// Generic function to fetch image bytes from a URL
async fn fetch_image(url: &str) -> Result<Vec<u8>, anyhow::Error> {
    info!("Fetching image from: {}", url);

    let client = reqwest::Client::builder().build()?;
    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("HTTP {}: {}", response.status(), url));
    }

    let bytes = response.bytes().await?;
    info!("Successfully fetched {} bytes from {}", bytes.len(), url);

    Ok(bytes.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_aurora_forecast() {
        let result = fetch_aurora_forecast().await;
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(!data.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_ace_real_time_solar_wind() {
        let result = fetch_ace_real_time_solar_wind().await;
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(!data.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_dscovr_solar_wind() {
        let result = fetch_dscovr_solar_wind().await;
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(!data.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_space_weather_overview() {
        let result = fetch_space_weather_overview().await;
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(!data.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_ace_epam() {
        let result = fetch_ace_epam().await;
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(!data.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_canadian_magnetic() {
        let result = fetch_canadian_magnetic().await;
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(!data.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_alerts_timeline() {
        let result = fetch_alerts_timeline().await;
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(!data.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_tonights_aurora_forecast() {
        let result = fetch_tonights_aurora_forecast().await;
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(!data.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_tomorrow_aurora_forecast() {
        let result = fetch_tomorrow_aurora_forecast().await;
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(!data.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_wsa_enlil() {
        let result = fetch_wsa_enlil().await;
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(!data.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_all_aurora_images() {
        let result = fetch_all_aurora_images().await;
        assert!(result.is_ok());
        let images = result.unwrap();
        assert!(!images.kjell_henriksen_observatory_norway.is_empty());
        assert!(!images.hankasalmi_finland.is_empty());
        assert!(!images.yellowknife_canada.is_empty());
        assert!(!images.athabasca_canada.is_empty());
        assert!(!images.glacier_national_park_usa.is_empty());
        assert!(!images.hansville_usa.is_empty());
        assert!(!images.isle_royale_national_park_usa.is_empty());
        assert!(!images.heiligenblut_austria.is_empty());
        assert!(!images.calgary_canada.is_empty());
        assert!(!images.hobart_australia.is_empty());
    }
}
