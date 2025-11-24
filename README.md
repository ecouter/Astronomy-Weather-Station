# Astronomy Weather Station
 A RaspberryPi Weather Station and Observation planner made for Astronomy. Project is in pre-alpha, expect lots of changes to api, backend, and UI.


# To Run the Program

## Install dependencies:
Install ``rustup``

[Build SHARPpy](api/SHARPpy/README.md)

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
**Optionnal arguments for more info: ``RUST_LOG=info``**

**LINUX:**
```bash
./target/release/weather_frontend
```

**WINDOWS:**
```bash
target\release\weather_frontend.exe
```

# DEBUG


- **KNOWN BUG:** Background map image is not loading
**Fix:** Create the ``images`` folder in frontend/ui/. This is where the map file is saved.