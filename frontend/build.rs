use std::process::Command;

fn main() {
    // Compile the SLINT UI
    slint_build::compile("ui/environment_canada.slint").unwrap();
    slint_build::compile("ui/main.slint").unwrap();
}