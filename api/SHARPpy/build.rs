use std::env;
use std::path::PathBuf;

fn main() {
    // Set the Python executable to use the conda environment
    let conda_prefix = env::var("CONDA_PREFIX").unwrap_or_else(|_| {
        // Fallback to a reasonable default
        "/home/boris/.conda/envs/devel".to_string()
    });

    let python_executable = PathBuf::from(&conda_prefix).join("bin").join("python");

    // Tell PyO3 to use this Python executable
    println!("cargo:rustc-env=PYO3_PYTHON={}", python_executable.display());

    // Also set the library path for runtime
    let lib_path = PathBuf::from(&conda_prefix).join("lib");
    println!("cargo:rustc-link-search=native={}", lib_path.display());
}
