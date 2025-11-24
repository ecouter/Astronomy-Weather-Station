# Contributing to Astronomy Weather Station

Hello there, fellow sky gazer! We're thrilled you're interested in joining our little project. This app helps astronomers like us check weather conditions on the fly, especially for those Raspberry Pi-powered setups. Since we're still in pre-alpha, things are evolving fast your input could help shape the final product.

## Project Composition

Our project is built with Rust and organized into neat, modular parts:

- **API Crates**: We have separate Rust crates for different data sources, like meteoblue for weather forecasts, aurora for northern lights predictions, environment_canada for astronomy specific predictions, and others. Each lives in its own folder under the api/ directory, making it easy to add or tweak data providers.
  
- **Frontend**: The user interface is crafted using Slint, a sleek GUI framework, combined with Rust logic. The frontend/ folder holds the main app code, including UI files (.slint) for screens and Rust modules for handling data and logic. There's also a material-1.0/ directory with UI components for a polished look.

- **Build and Configuration**: Everything ties together with Cargo for Rust builds. You'll need to create a simple coordinates.json file with your location to get started.

## Contributing Basics

We'd love your help to make this better. Here's the easy way to get involved:

- **Get Set Up**: Fork the repo, clone it locally, and make sure Rust is installed. Build the frontend as per the README to test your changes.

- **Code with Care**: Follow Rust's style guide for consistency. Aim for clear, efficient code – we're fans of simplicity here. If adding new features, consider how they fit into the modular API structure.

- **Test Thoroughly**: Before sharing, run the app on a Raspberry Pi if possible, or simulate with test data. We value stability in something as critical as weather forecasts.

- **Share Your Work**: Create a branch for your changes (something descriptive like "add-rain-forecast"), commit often, and open a pull request. Suggestions are welcome!

Remember, this is a community effort, so be patient as we refine things. Got questions? Drop an issue in the repo.
