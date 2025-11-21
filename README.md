# RaspberryPi-Astronomy-Weather-Station
 A RaspberryPi Weather Station and Observation planner made for Astronomy. 


# To Run the Program

## Install dependencies:
Install ``rustup``

[Build SHARPpy (look at the readme)](api/SHARPpy/README.md)

## Build the project

**Navigate to directory**

```bash
cd frontend
```

**Build**

```bash
cargo build --release
```

## Create config files
**Create ``coordinates.json`` in the project directory (where this README is)**
```json
{
    "lat":"LATITUDE_IN_DECIMALS",
    "lon":"LONGITUDE_IN_DECIMALS"
}
```
## Start
**Optionnal arguments: ``RUST_LOG=``{debug or info}**
```bash
./target/release/weather_frontend
```
