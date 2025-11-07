# RaspberryPi-Astronomy-Weather-Station
 A RaspberryPi Weather Station and Observation planner made for Astronomy. 


# To Run the Program

## Install dependencies:
#### Install **rustup**
#### For arch-based distros:
```bash
yay python-conda
```

## Conda initialisation:

#### Init conda
```bash
conda init
```

Restart your shell after initialisation
#### Create new environment
```bash
conda env create -f api/SHARPpy/SHARPpy/environment.yml
```
#### Switch to environment
```bash
conda activate devel
```
## Build the project

#### Navigate to directory

```bash
cd frontend
```

#### Build

```bash
cargo build --release
```

## Create config files
#### Create ``coordinates.json`` in the project directory (where this README is)
```json
{
    "lat":"LATITUDE_IN_DECIMALS",
    "lon":"LONGITUDE_IN_DECIMALS"
}
```
## Start
##### Optionnal arguments: ``RUST_LOG=``{debug or info}
```bash
LD_LIBRARY_PATH=$CONDA_PREFIX/lib:$LD_LIBRARY_PATH ./target/release/weather_frontend
```
