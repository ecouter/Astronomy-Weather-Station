#!/usr/bin/env python3
"""
SHARPpy Sounding Generator from GRIB2 Files

This script reads Environment Canada RDPS GRIB2 files and creates
a complete atmospheric sounding overview using SHARPpy.
"""

import os
import sys
import numpy as np
import xarray as xr
import cfgrib
import matplotlib
matplotlib.use('Agg')  # Use non-interactive backend
import matplotlib.pyplot as plt
import pickle
import hashlib
from datetime import datetime

# Add SHARPpy to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'api', 'SHARPpy', 'SHARPpy'))

import sharppy
import sharppy.sharptab.profile as profile
import sharppy.sharptab.interp as interp
import sharppy.sharptab.winds as winds
import sharppy.sharptab.utils as utils
import sharppy.sharptab.params as params
import sharppy.sharptab.thermo as thermo

# Import SHARPpy GUI components for complete rendering
from sharppy.viz.SPCWindow import SPCWindow
from sutils.config import Config
from datetime import datetime
import platform

# Set up Qt for headless operation
import os
os.environ['QT_QPA_PLATFORM'] = 'offscreen'
from qtpy.QtWidgets import QApplication, QWidget
from qtpy.QtCore import Qt, Signal
QApplication.setAttribute(Qt.AA_EnableHighDpiScaling)
QApplication.setAttribute(Qt.AA_UseHighDpiPixmaps)

# Import matplotlib plotting functions
from sharppy.plot.skew import draw_title, draw_dry_adiabats, draw_mixing_ratio_lines, draw_moist_adiabats
from sharppy.plot.skew import plot_wind_axes, plot_wind_barbs, draw_heights, plot_sig_levels
from sharppy.plot.skew import draw_effective_inflow_layer, plotHodo, draw_hodo_inset
from matplotlib.ticker import ScalarFormatter, MultipleLocator, NullFormatter
from matplotlib.collections import LineCollection
import matplotlib.transforms as transforms
import matplotlib.gridspec as gridspec

def get_cache_key(grib2_path, lat, lon):
    """
    Generate a cache key based on GRIB2 directory, coordinates, and file modification times.

    Parameters:
    grib2_path: Path to GRIB2 directory
    lat: Latitude
    lon: Longitude

    Returns:
    String cache key
    """
    if os.path.isdir(grib2_path):
        # Get all relevant GRIB2 files
        all_files = [os.path.join(grib2_path, f)
                    for f in os.listdir(grib2_path)
                    if f.endswith('.grib2')]
        key_patterns = ['TMP_ISBL', 'RH_ISBL', 'HGT_ISBL', 'UGRD_ISBL', 'VGRD_ISBL']
        grib2_files = sorted([f for f in all_files if any(pattern in f for pattern in key_patterns)][:50])

        # Create hash based on file paths, modification times, and coordinates
        hasher = hashlib.md5()
        hasher.update(f"{grib2_path}:{lat}:{lon}".encode())

        for grib_file in grib2_files:
            if os.path.exists(grib_file):
                mtime = os.path.getmtime(grib_file)
                hasher.update(f"{grib_file}:{mtime}".encode())

        return hasher.hexdigest()
    else:
        # Single file
        if os.path.exists(grib2_path):
            mtime = os.path.getmtime(grib2_path)
            return hashlib.md5(f"{grib2_path}:{lat}:{lon}:{mtime}".encode()).hexdigest()
        else:
            return None


def save_profile_cache(cache_key, profile_data):
    """
    Save profile data to cache file.

    Parameters:
    cache_key: Cache key string
    profile_data: Profile data dictionary
    """
    cache_dir = os.path.join(os.path.dirname(__file__), 'cache')
    os.makedirs(cache_dir, exist_ok=True)

    cache_file = os.path.join(cache_dir, f"profile_{cache_key}.pkl")
    with open(cache_file, 'wb') as f:
        pickle.dump({
            'timestamp': datetime.now(),
            'data': profile_data
        }, f)
    print(f"Saved profile data to cache: {cache_file}")


def load_profile_cache(cache_key):
    """
    Load profile data from cache if available and valid.

    Parameters:
    cache_key: Cache key string

    Returns:
    Profile data dictionary or None if cache miss/invalid
    """
    cache_dir = os.path.join(os.path.dirname(__file__), 'cache')
    cache_file = os.path.join(cache_dir, f"profile_{cache_key}.pkl")

    if not os.path.exists(cache_file):
        return None

    try:
        with open(cache_file, 'rb') as f:
            cached = pickle.load(f)

        # Check if cache is less than 1 hour old
        if (datetime.now() - cached['timestamp']).total_seconds() < 3600:
            print(f"Loaded profile data from cache: {cache_file}")
            return cached['data']
        else:
            print("Cache is stale, reprocessing...")
            return None
    except Exception as e:
        print(f"Error loading cache: {e}")
        return None


def read_grib2_profile(grib2_files, lat, lon):
    """
    Read GRIB2 files and extract atmospheric profile at a specific location.
    Uses caching to avoid reprocessing the same data.

    Parameters:
    grib2_files: List of GRIB2 file paths or directory containing GRIB2 files
    lat: Latitude of the profile location
    lon: Longitude of the profile location

    Returns:
    Dictionary containing profile data
    """
    # Determine the path for caching
    if isinstance(grib2_files, str):
        cache_path = grib2_files
    else:
        cache_path = grib2_files[0] if grib2_files else ""

    # Check cache first
    cache_key = get_cache_key(cache_path, lat, lon)
    if cache_key:
        cached_data = load_profile_cache(cache_key)
        if cached_data is not None:
            return cached_data

    # Cache miss - process the data
    print("Processing GRIB2 data (cache miss)...")

    if isinstance(grib2_files, str):
        if os.path.isdir(grib2_files):
            # Find all GRIB2 files in directory
            all_files = [os.path.join(grib2_files, f)
                        for f in os.listdir(grib2_files)
                        if f.endswith('.grib2')]
            # For testing, limit to key files - look for TMP, RH, HGT, UGRD, VGRD at isobaric levels
            key_patterns = ['TMP_ISBL', 'RH_ISBL', 'HGT_ISBL', 'UGRD_ISBL', 'VGRD_ISBL']
            grib2_files = [f for f in all_files if any(pattern in f for pattern in key_patterns)][:50]  # Limit to 50 files
            print(f"Processing {len(grib2_files)} key GRIB2 files out of {len(all_files)} total")
        else:
            grib2_files = [grib2_files]

    # Initialize data structures - use dictionaries keyed by pressure level
    profile_data = {}  # pressure -> {'temp': val, 'rh': val, 'u': val, 'v': val, 'gh': val}

    # Process each GRIB2 file
    for grib_file in grib2_files:
        try:
            print(f"Processing {grib_file}")

            # Extract pressure level from filename (e.g., TMP_ISBL_1000 -> 1000 hPa)
            import re
            pres_match = re.search(r'ISBL_(\d+)', grib_file)
            if pres_match:
                pressure_level = int(pres_match.group(1))
                print(f"  Pressure level from filename: {pressure_level} hPa")
            else:
                print(f"  Could not extract pressure level from filename: {grib_file}")
                continue

            ds = cfgrib.open_dataset(grib_file, backend_kwargs={'indexpath': ''})
            print(f"  Opened dataset with vars: {list(ds.data_vars)}, dims: {list(ds.dims)}")

            # Extract data at the specified location
            # Find nearest grid point - handle different dimension names
            if 'latitude' in ds.dims:
                lat_idx = np.abs(ds.latitude.values - lat).argmin()
                lon_idx = np.abs(ds.longitude.values - lon).argmin()
            elif 'y' in ds.dims:
                # Some GRIB2 files use 'y', 'x' instead of 'latitude', 'longitude'
                lat_idx = np.abs(ds.y.values - lat).argmin()
                lon_idx = np.abs(ds.x.values - lon).argmin()
            else:
                print(f"Unknown coordinate dimensions in {grib_file}: {list(ds.dims.keys())}")
                ds.close()
                continue

            # Initialize data entry for this pressure level if not exists
            if pressure_level not in profile_data:
                profile_data[pressure_level] = {}

            # Check what variables are available
            if 't' in ds.data_vars:  # Temperature
                print("  Found temperature data")
                if 'latitude' in ds.dims:
                    temp_value = ds['t'].isel(latitude=lat_idx, longitude=lon_idx).values
                else:
                    temp_value = ds['t'].isel(y=lat_idx, x=lon_idx).values
                profile_data[pressure_level]['temp'] = temp_value - 273.15  # Convert K to C
                print(f"    Temperature: {temp_value} K -> {temp_value - 273.15} C")

            if 'r' in ds.data_vars:  # Relative humidity
                print("  Found relative humidity data")
                if 'latitude' in ds.dims:
                    rh_value = ds['r'].isel(latitude=lat_idx, longitude=lon_idx).values
                else:
                    rh_value = ds['r'].isel(y=lat_idx, x=lon_idx).values
                profile_data[pressure_level]['rh'] = rh_value / 100.0  # Convert % to fraction
                print(f"    Relative humidity: {rh_value} -> {rh_value / 100.0}")

            if 'u' in ds.data_vars:  # U wind component
                print("  Found U wind data")
                if 'latitude' in ds.dims:
                    u_value = ds['u'].isel(latitude=lat_idx, longitude=lon_idx).values
                else:
                    u_value = ds['u'].isel(y=lat_idx, x=lon_idx).values
                profile_data[pressure_level]['u'] = u_value * 1.94384  # Convert m/s to knots
                print(f"    U wind: {u_value} m/s -> {u_value * 1.94384} knots")

            if 'v' in ds.data_vars:  # V wind component
                print("  Found V wind data")
                if 'latitude' in ds.dims:
                    v_value = ds['v'].isel(latitude=lat_idx, longitude=lon_idx).values
                else:
                    v_value = ds['v'].isel(y=lat_idx, x=lon_idx).values
                profile_data[pressure_level]['v'] = v_value * 1.94384  # Convert m/s to knots
                print(f"    V wind: {v_value} m/s -> {v_value * 1.94384} knots")

            if 'gh' in ds.data_vars:  # Geopotential height
                print("  Found geopotential height data")
                if 'latitude' in ds.dims:
                    gh_value = ds['gh'].isel(latitude=lat_idx, longitude=lon_idx).values
                else:
                    gh_value = ds['gh'].isel(y=lat_idx, x=lon_idx).values
                profile_data[pressure_level]['gh'] = gh_value
                print(f"    Geopotential height: {gh_value} m")

            ds.close()

        except Exception as e:
            print(f"Error processing {grib_file}: {e}")
            import traceback
            traceback.print_exc()
            continue

    # Now convert the profile_data dictionary to arrays
    # Only include levels that have temperature data
    pressure_levels = []
    temperatures = []
    dewpoints = []
    u_winds = []
    v_winds = []
    heights = []

    for pres in sorted(profile_data.keys(), reverse=True):  # Sort descending
        data = profile_data[pres]

        # Temperature is required
        if 'temp' not in data:
            continue  # Skip levels without temperature

        pressure_levels.append(pres)
        temperatures.append(data['temp'])

        # Calculate dewpoint if we have RH and temp
        if 'rh' in data:
            temp_c = data['temp']
            rh = data['rh']
            # Calculate dewpoint from temperature and relative humidity
            # RH = 100 * e_s(td) / e_s(t), so e_s(td) = RH * e_s(t)
            # td = temp_at_vappres(e_s(td))
            dewpoint = thermo.temp_at_vappres(rh * thermo.vappres(temp_c))
            dewpoints.append(dewpoint)
        else:
            dewpoints.append(np.nan)

        # Winds
        if 'u' in data:
            u_winds.append(data['u'])
        else:
            u_winds.append(np.nan)

        if 'v' in data:
            v_winds.append(data['v'])
        else:
            v_winds.append(np.nan)

        # Heights
        if 'gh' in data:
            heights.append(data['gh'])
        else:
            heights.append(np.nan)

    print(f"Final profile data: pressure={len(pressure_levels)}, temp={len(temperatures)}, height={len(heights)}")

    profile_data = {
        'pressure': np.array(pressure_levels),
        'temperature': np.array(temperatures),
        'dewpoint': np.array(dewpoints),
        'u_wind': np.array(u_winds),
        'v_wind': np.array(v_winds),
        'height': np.array(heights)
    }

    # Save to cache
    if cache_key:
        save_profile_cache(cache_key, profile_data)

    return profile_data


def create_sharppy_profile(profile_data):
    """
    Create a SHARPpy Profile object from extracted data.

    Parameters:
    profile_data: Dictionary with pressure, temperature, dewpoint, winds, height

    Returns:
    SHARPpy Profile object
    """
    pres = profile_data['pressure']
    tmpc = profile_data['temperature']
    hght = profile_data['height']

    # Filter out levels where temperature or pressure is NaN
    valid_temp = ~(np.isnan(tmpc) | np.isnan(pres))

    pres = pres[valid_temp]
    tmpc = tmpc[valid_temp]
    hght = hght[valid_temp]

    # For heights, interpolate missing values
    if np.any(np.isnan(hght)):
        # Simple interpolation for missing heights
        valid_hght_mask = ~np.isnan(hght)
        if np.sum(valid_hght_mask) >= 2:
            from scipy import interpolate
            f = interpolate.interp1d(pres[valid_hght_mask], hght[valid_hght_mask],
                                   bounds_error=False, fill_value='extrapolate')
            hght = f(pres)
        else:
            # If we don't have enough height data, use a simple hydrostatic approximation
            # Assume surface height is 0 and use standard atmosphere lapse rate
            hght = np.zeros_like(pres)
            for i in range(1, len(pres)):
                # Approximate height difference using hydrostatic equation
                # h2 = h1 + (R*T/g) * ln(p1/p2)
                T_avg = (tmpc[i-1] + tmpc[i]) / 2 + 273.15  # Convert back to K
                hght[i] = hght[i-1] + (287 * T_avg / 9.81) * np.log(pres[i-1] / pres[i])

    # Handle dewpoint (may have NaNs)
    dwpc = profile_data['dewpoint'][valid_temp]
    dwpc = np.where(np.isnan(dwpc), tmpc - 10, dwpc)  # Default to 10C dewpoint depression

    # Handle winds (may have NaNs) - ensure no NaN values
    wspd = np.full_like(pres, 5.0)  # Default wind speed
    wdir = np.full_like(pres, 270.0)  # Default wind direction

    if len(profile_data['u_wind']) > 0 and len(profile_data['v_wind']) > 0:
        u_wind = profile_data['u_wind'][valid_temp]
        v_wind = profile_data['v_wind'][valid_temp]

        # Replace NaN values with defaults
        u_wind = np.where(np.isnan(u_wind), 0.0, u_wind)
        v_wind = np.where(np.isnan(v_wind), 0.0, v_wind)

        # Calculate wind speed and direction
        wspd_calc, wdir_calc = utils.comp2vec(u_wind, v_wind)

        # Ensure no NaN or infinite values
        wspd_calc = np.where(np.isfinite(wspd_calc) & (wspd_calc >= 0), wspd_calc, 5.0)
        wdir_calc = np.where(np.isfinite(wdir_calc), wdir_calc, 270.0)

        wspd = wspd_calc
        wdir = wdir_calc

    print(f"Creating profile with {len(pres)} levels")
    print(f"Pressure range: {pres.min()} - {pres.max()} hPa")
    print(f"Temperature range: {tmpc.min():.1f} - {tmpc.max():.1f} C")
    print(f"Height range: {hght.min():.0f} - {hght.max():.0f} m")

    # Create profile
    from datetime import datetime
    prof = profile.create_profile(
        profile='convective',
        pres=pres,
        hght=hght,
        tmpc=tmpc,
        dwpc=dwpc,
        wspd=wspd,
        wdir=wdir,
        date=datetime.now(),  # Provide current date
        location='GRIB2',  # Provide location
        missing=-9999,
        strictQC=False
    )

    return prof


def convert_profile_to_sharppy_format(profile_data, location="GRIB2", lat=45.0, lon=-75.0):
    """
    Convert profile data to SHARPpy text format for GUI processing.

    Parameters:
    profile_data: Dictionary with pressure, temperature, dewpoint, winds, height
    location: Station/location name
    lat: Latitude
    lon: Longitude

    Returns:
    String containing SHARPpy formatted data
    """
    from datetime import datetime

    # Create title line
    time_str = datetime.now().strftime('%y%m%d/%H%M')
    title_line = f"{location:>4}   {time_str}   {lat:.1f},{lon:.1f}"

    # Create data lines
    data_lines = []
    data_lines.append("   LEVEL       HGHT       TEMP       DWPT       WDIR       WSPD")
    data_lines.append("-------------------------------------------------------------------")
    data_lines.append("%RAW%")

    for i in range(len(profile_data['pressure'])):
        pres = profile_data['pressure'][i]
        hght = profile_data['height'][i]
        temp = profile_data['temperature'][i]
        dwpt = profile_data['dewpoint'][i]

        # Handle wind data
        if np.isnan(profile_data['u_wind'][i]) or np.isnan(profile_data['v_wind'][i]):
            wdir = -9999.0
            wspd = -9999.0
        else:
            # Convert u,v to direction and speed
            u = profile_data['u_wind'][i]
            v = profile_data['v_wind'][i]
            wspd = np.sqrt(u**2 + v**2)
            wdir = (270 - np.degrees(np.arctan2(v, u))) % 360

        # Format line: pressure, height, temp, dewpt, wdir, wspd
        line = "%8.2f,  %8.2f,  %8.2f,  %8.2f,  %8.2f,  %8.2f" % (pres, hght, temp, dwpt, wdir, wspd)
        data_lines.append(line)

    data_lines.append("%END%")

    # Combine all lines
    full_data = ["%TITLE%", title_line] + data_lines

    return "\n".join(full_data)


def create_sounding_plot(prof, output_file='sounding.png', title='Atmospheric Sounding'):
    """
    Create a complete sounding overview plot using SHARPpy's full GUI.

    Parameters:
    prof: SHARPpy Profile object
    output_file: Output PNG file path
    title: Plot title
    """
    # Use full SHARPpy GUI to generate comprehensive image
    create_sharppy_gui_image(prof, output_file, title)





def create_sharppy_gui_image(prof, output_file='sounding.png', title='Atmospheric Sounding'):
    """
    Create a comprehensive sounding plot using SHARPpy's complete GUI rendering system.
    This replicates the exact same output as the SHARPpy GUI when clicking "Generate Profiles".
    """
    try:
        print("Creating comprehensive sounding plot using SHARPpy GUI...")

        # Initialize Qt application for off-screen rendering
        app = QApplication.instance()
        if app is None:
            app = QApplication([])

        # Create a ProfCollection to hold our profile (like the GUI does)
        import sharppy.sharptab.prof_collection as prof_collection
        prof_col = prof_collection.ProfCollection(
            {'':[ prof ]},
            [ datetime.now() ],
        )

        # Set metadata like the GUI does
        prof_col.setMeta('model', 'GRIB2')
        prof_col.setMeta('run', datetime.now())
        prof_col.setMeta('base_time', datetime.now())  # Base time for forecast hour calculation
        prof_col.setMeta('loc', title)
        prof_col.setMeta('fhour', 0)  # Single profile, not a forecast
        prof_col.setMeta('observed', False)  # This is model data

        # Create configuration object with a temporary config file
        import tempfile
        temp_config = tempfile.NamedTemporaryFile(mode='w', suffix='.ini', delete=False)
        temp_config.close()

        config = Config(temp_config.name)

        # Initialize preferences like the GUI does
        from sharppy.viz.preferences import PrefDialog
        PrefDialog.initConfig(config)

        # Initialize other config sections
        config.initialize({
            ('insets', 'left_inset'): 'SARS',
            ('insets', 'right_inset'): 'STP STATS',
            ('parcel_types', 'pcl1'): 'SFC',
            ('parcel_types', 'pcl2'): 'ML',
            ('parcel_types', 'pcl3'): 'FCST',
            ('parcel_types', 'pcl4'): 'MU',
            ('paths', 'save_img'): os.path.dirname(output_file) or os.getcwd(),
            ('paths', 'save_txt'): os.path.dirname(output_file) or os.getcwd(),
        })

        # Create a dummy parent object with the required signal and methods
        class DummyParent(QWidget):
            config_changed = Signal(Config)

            def preferencesbox(self):
                pass  # Dummy method

        dummy_parent = DummyParent()

        # Create SPCWindow (off-screen) with dummy parent
        spc_window = SPCWindow(parent=dummy_parent, cfg=config)

        # Add the profile collection (this triggers all the GUI rendering)
        spc_window.addProfileCollection(prof_col, focus=True)

        # Save the complete GUI image using the same method as the GUI
        spc_window.spc_widget.pixmapToFile(output_file)

        # Check if file was actually created
        if os.path.exists(output_file):
            file_size = os.path.getsize(output_file)
            print(f"SHARPpy GUI sounding plot saved to {output_file} ({file_size} bytes)")
        else:
            raise Exception(f"Failed to save image to {output_file}")

        # Clean up
        spc_window.close()

    except Exception as e:
        print(f"Error in SHARPpy GUI rendering: {e}")
        import traceback
        traceback.print_exc()


def generate_sounding_overview(grib2_path, lat, lon, output_file='sounding.png', title=None):
    """
    Main function to generate a complete sounding overview from GRIB2 files.

    Parameters:
    grib2_path: Path to GRIB2 file(s) or directory containing GRIB2 files
    lat: Latitude of the sounding location
    lon: Longitude of the sounding location
    output_file: Output PNG file path
    title: Plot title (auto-generated if None)
    """
    print(f"Generating sounding for location {lat}N, {lon}E")

    # Read GRIB2 data
    profile_data = read_grib2_profile(grib2_path, lat, lon)

    if len(profile_data['pressure']) == 0:
        raise ValueError("No valid profile data found in GRIB2 files")

    print(f"Extracted profile with {len(profile_data['pressure'])} levels")

    # Create SHARPpy profile
    prof = create_sharppy_profile(profile_data)

    # Generate title if not provided
    if title is None:
        title = f"Atmospheric Sounding - {lat:.2f}N, {lon:.2f}E"

    # Create and save plot
    create_sounding_plot(prof, output_file, title)

    return output_file


def create_mock_profile():
    """
    Create a mock atmospheric profile for testing purposes.
    """
    # Create a simple atmospheric profile similar to a typical summer day
    pressures = np.array([1000, 950, 900, 850, 800, 750, 700, 650, 600, 550, 500, 450, 400, 350, 300, 250, 200])
    temperatures = np.array([25, 20, 15, 10, 5, 0, -5, -10, -15, -20, -25, -30, -35, -40, -45, -50, -55])
    dewpoints = np.array([20, 15, 10, 5, 0, -5, -10, -15, -20, -25, -30, -40, -50, -60, -70, -80, -90])
    heights = np.array([0, 500, 1000, 1500, 2000, 2500, 3000, 3500, 4000, 4500, 5500, 6500, 7500, 8500, 9500, 11000, 12000])
    u_winds = np.array([5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55, 60, 65, 70, 75, 80, 85])
    v_winds = np.array([0, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55, 60, 65, 70, 75, 80])

    return {
        'pressure': pressures,
        'temperature': temperatures,
        'dewpoint': dewpoints,
        'u_wind': u_winds,
        'v_wind': v_winds,
        'height': heights
    }


def test_with_mock_data():
    """
    Test the sounding generation with mock data.
    """
    print("=== Testing with Mock Data ===")

    # Create mock profile data
    profile_data = create_mock_profile()

    # Create SHARPpy profile
    prof = create_sharppy_profile(profile_data)

    # Generate SHARPpy GUI plot
    print("Generating SHARPpy GUI plot...")
    create_sounding_plot(prof, output_file='sounding_mock.png', title='Mock Atmospheric Sounding')

    print("Test completed!")


def test_with_grib2_data():
    """
    Test the sounding generation with real GRIB2 data.
    """
    print("=== Testing with Real GRIB2 Data ===")

    # Use the GRIB2 directory
    grib2_dir = 'model_data/grib2'
    lat, lon = 45.0, -75.0  # Ottawa, Canada

    try:
        # Generate SHARPpy GUI plot from GRIB2 data
        print("Generating SHARPpy GUI plot from GRIB2 data...")
        output_file = generate_sounding_overview(grib2_dir, lat, lon, output_file='sounding_grib2.png')
        print(f"Success! SHARPpy GUI plot saved to {output_file}")

    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()


if __name__ == '__main__':
    # Test with real GRIB2 data
    try:
        test_with_grib2_data()
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
