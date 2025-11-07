use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::path::Path;

/// Generate a GFS atmospheric sounding plot using SHARPpy (async version)
pub async fn generate_gfs_sounding_async(lat: f64, lon: f64, output_file: Option<String>, title: Option<String>) -> Result<String, anyhow::Error> {
    // Use tokio::task::spawn_blocking to run the synchronous Python code in a blocking task
    tokio::task::spawn_blocking(move || {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| -> Result<String, anyhow::Error> {
            // Import sys to add the path
            let sys = py.import("sys").map_err(|e| anyhow::anyhow!("Failed to import sys: {}", e))?;
            let path_attr = sys.getattr("path").map_err(|e| anyhow::anyhow!("Failed to get sys.path: {}", e))?;

            // Try to downcast to PyList
            let sys_path = match path_attr.downcast::<PyList>() {
                Ok(list) => list,
                Err(_) => return Err(anyhow::anyhow!("Failed to get sys.path as PyList")),
            };

            // Add the current directory and SHARPpy directory to Python path
            let current_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
            let sharppy_path = current_dir.join("SHARPpy");

            sys_path.insert(0, current_dir.to_str().unwrap()).map_err(|e| anyhow::anyhow!("Failed to insert current dir: {}", e))?;
            sys_path.insert(0, sharppy_path.to_str().unwrap()).map_err(|e| anyhow::anyhow!("Failed to insert SHARPpy path: {}", e))?;

            // Import the create_sounding_gfs module
            let create_sounding = py.import("create_sounding_gfs").map_err(|e| anyhow::anyhow!("Failed to import create_sounding_gfs: {}", e))?;

            // Get the generate_gfs_sounding function
            let generate_func = create_sounding.getattr("generate_gfs_sounding").map_err(|e| anyhow::anyhow!("Failed to get generate_gfs_sounding function: {}", e))?;

            // Prepare arguments
            let args = PyDict::new(py);
            args.set_item("lat", lat).map_err(|e| anyhow::anyhow!("Failed to set lat: {}", e))?;
            args.set_item("lon", lon).map_err(|e| anyhow::anyhow!("Failed to set lon: {}", e))?;

            if let Some(output) = output_file {
                args.set_item("output_file", output).map_err(|e| anyhow::anyhow!("Failed to set output_file: {}", e))?;
            }

            if let Some(t) = title {
                args.set_item("title", t).map_err(|e| anyhow::anyhow!("Failed to set title: {}", e))?;
            }

            // Call the function
            let result_obj = generate_func.call((), Some(args)).map_err(|e| anyhow::anyhow!("Failed to call generate_gfs_sounding: {}", e))?;
            let result: String = result_obj.extract().map_err(|e| anyhow::anyhow!("Failed to extract result: {}", e))?;

            Ok(result)
        })
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
