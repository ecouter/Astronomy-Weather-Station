use std::path::PathBuf;
use std::process::Stdio;

/// Generate a GFS atmospheric sounding plot using SHARPpy executable (async version)
pub async fn generate_gfs_sounding_async(lat: f64, lon: f64, output_file: Option<String>, _title: Option<String>) -> Result<String, anyhow::Error> {
    // Use tokio::task::spawn_blocking to run the synchronous command execution in a blocking task
    tokio::task::spawn_blocking(move || {
        // Get the path to the executable
        let exe_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("dist")
            .join("create_sounding_gfs");

        // Verify executable exists
        if !exe_path.exists() {
            return Err(anyhow::anyhow!("SHARPpy executable not found at: {}", exe_path.display()));
        }

        // Build command arguments
        let mut args = vec![
            "--lat".to_string(),
            format!("{}", lat),
            "--lon".to_string(),
            format!("{}", lon),
        ];

        // Add output file if specified
        let final_output_path = if let Some(output) = &output_file {
            args.extend_from_slice(&["--output".to_string(), output.clone()]);
            output.clone()
        } else {
            "sounding_gfs.png".to_string()
        };

        // Execute the command asynchronously, but since we're in spawn_blocking, use std::process::Command
        let output = std::process::Command::new(&exe_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to execute SHARPpy executable: {}", e))?;

        // Check if command was successful
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("SHARPpy executable failed: {}", stderr));
        }

        // Verify output file was created
        if !std::path::Path::new(&final_output_path).exists() {
            return Err(anyhow::anyhow!("SHARPpy executable did not create output file: {}", final_output_path));
        }

        // Log success
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            if !stdout.trim().is_empty() {
                println!("SHARPpy output: {}", stdout.trim());
            }
        }

        Ok(final_output_path)
    }).await?
}

/// Generate a GFS atmospheric sounding plot using SHARPpy (synchronous version for compatibility)
pub fn generate_gfs_sounding(lat: f64, lon: f64, output_file: Option<&str>, title: Option<&str>) -> Result<String, anyhow::Error> {
    // For backward compatibility, provide a synchronous wrapper
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(generate_gfs_sounding_async(
        lat,
        lon,
        output_file.map(|s| s.to_string()),
        title.map(|s| s.to_string())
    ))
}

/// Structure representing sounding parameters
#[derive(Debug, Clone)]
pub struct SoundingParams {
    pub lat: f64,
    pub lon: f64,
    pub output_file: Option<String>,
    pub title: Option<String>,
}

impl SoundingParams {
    pub fn new(lat: f64, lon: f64) -> Self {
        Self {
            lat,
            lon,
            output_file: None,
            title: None,
        }
    }

    pub fn with_output_file(mut self, output_file: String) -> Self {
        self.output_file = Some(output_file);
        self
    }

    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }
}

/// Generate sounding using functional approach
pub fn generate_sounding(params: SoundingParams) -> Result<String, anyhow::Error> {
    let output_file = params.output_file.as_deref();
    let title = params.title.as_deref();

    generate_gfs_sounding(params.lat, params.lon, output_file, title)
}
