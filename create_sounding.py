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

# Add SHARPpy to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'api', 'SHARPpy', 'SHARPpy'))

import sharppy
import sharppy.sharptab.profile as profile
import sharppy.sharptab.interp as interp
import sharppy.sharptab.winds as winds
import sharppy.sharptab.utils as utils
import sharppy.sharptab.params as params
import sharppy.sharptab.thermo as thermo
from sharppy.viz.skew import plotSkewT
from sharppy.viz.hodo import plotHodo
from sharppy.viz.thermo import plotText
from sharppy.viz.kinematics import plotKinematics
from sharppy.viz.watch import plotWatch
from sharppy.viz.slinky import plotSlinky
from sharppy.viz.advection import plotAdvection
from sharppy.viz.stp import plotSTP

def read_grib2_profile(grib2_files, lat, lon):
    """
    Read GRIB2 files and extract atmospheric profile at a specific location.

    Parameters:
    grib2_files: List of GRIB2 file paths or directory containing GRIB2 files
    lat: Latitude of the profile location
    lon: Longitude of the profile location

    Returns:
    Dictionary containing profile data
    """
    if isinstance(grib2_files, str):
        if os.path.isdir(grib2_files):
            # Find all GRIB2 files in directory
            all_files = [os.path.join(grib2_files, f)
                        for f in os.listdir(grib2_files)
                        if f.endswith('.grib2')]
            # For testing, limit to key files
            key_patterns = ['AirTemp', 'RelativeHumidity', 'GeopotentialHeight', 'WindU', 'WindV']
            grib2_files = [f for f in all_files if any(pattern in f for pattern in key_patterns)][:20]  # Limit to 20 files
            print(f"Processing {len(grib2_files)} key GRIB2 files out of {len(all_files)} total")
        else:
            grib2_files = [grib2_files]

    # Initialize data structures - use dictionaries keyed by pressure level
    profile_data = {}  # pressure -> {'temp': val, 'rh': val, 'u': val, 'v': val, 'gh': val}

    # Process each GRIB2 file
    for grib_file in grib2_files:
        try:
            print(f"Processing {grib_file}")

            # Extract pressure level from filename (e.g., IsbL-1000 -> 1000 hPa)
            import re
            pres_match = re.search(r'IsbL-(\d+)', grib_file)
            if pres_match:
                pressure_level = int(pres_match.group(1))
                print(f"  Pressure level from filename: {pressure_level} hPa")
            else:
                print(f"  Could not extract pressure level from filename")
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
            dewpoint = thermo.wetbulb(pres, temp_c, rh * 100)
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

    return {
        'pressure': np.array(pressure_levels),
        'temperature': np.array(temperatures),
        'dewpoint': np.array(dewpoints),
        'u_wind': np.array(u_winds),
        'v_wind': np.array(v_winds),
        'height': np.array(heights)
    }


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

    # Handle winds (may have NaNs)
    wspd = np.full_like(pres, 5.0)  # Default wind speed
    wdir = np.full_like(pres, 270.0)  # Default wind direction

    if len(profile_data['u_wind']) > 0 and len(profile_data['v_wind']) > 0:
        u_wind = profile_data['u_wind'][valid_temp]
        v_wind = profile_data['v_wind'][valid_temp]

        valid_wind = ~(np.isnan(u_wind) | np.isnan(v_wind))
        if np.any(valid_wind):
            wspd[valid_wind], wdir[valid_wind] = utils.comp2vec(u_wind[valid_wind], v_wind[valid_wind])

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


def create_sounding_plot(prof, output_file='sounding.png', title='Atmospheric Sounding', use_gui=False):
    """
    Create a complete sounding overview plot and save as PNG.

    Parameters:
    prof: SHARPpy Profile object
    output_file: Output PNG file path
    title: Plot title
    use_gui: If True, use full SHARPpy GUI for ~3MB image; if False, use simple matplotlib plot
    """
    if use_gui:
        # Use full SHARPpy GUI to generate comprehensive image
        create_sharppy_gui_image(prof, output_file, title)
    else:
        # Create a simple matplotlib skew-T plot
        create_simple_plot(prof, output_file, title)


def create_simple_plot(prof, output_file='sounding.png', title='Atmospheric Sounding'):
    """
    Create a simple matplotlib skew-T plot.
    """
    # Create a simple matplotlib skew-T plot
    fig, ax = plt.subplots(figsize=(10, 8))

    # Plot temperature and dewpoint
    ax.plot(prof.tmpc, prof.pres, 'r-', linewidth=2, label='Temperature')
    ax.plot(prof.dwpc, prof.pres, 'g-', linewidth=2, label='Dewpoint')

    # Plot virtual temperature
    ax.plot(prof.vtmp, prof.pres, 'r--', linewidth=1, alpha=0.7, label='Virtual Temp')

    # Set up the plot
    ax.set_xlabel('Temperature (°C)')
    ax.set_ylabel('Pressure (hPa)')
    ax.set_title(title)
    ax.set_ylim(1050, 100)  # Reverse y-axis
    ax.set_xlim(-60, 40)
    ax.grid(True, alpha=0.3)

    # Add some thermodynamic information
    try:
        pcl = params.parcelx(prof, flag=1)  # Surface parcel
        info_text = f"""
        Surface Parcel:
        CAPE: {pcl.bplus:.0f} J/kg
        CIN: {pcl.bminus:.0f} J/kg
        LCL: {pcl.lclhght:.0f} m
        LFC: {pcl.lfchght:.0f} m
        EL: {pcl.elhght:.0f} m
        """
        ax.text(0.02, 0.98, info_text, transform=ax.transAxes,
                verticalalignment='top', fontsize=10,
                bbox=dict(boxstyle='round', facecolor='white', alpha=0.8))
    except:
        pass

    ax.legend()
    plt.tight_layout()
    plt.savefig(output_file, dpi=150, bbox_inches='tight')
    plt.close(fig)

    print(f"Simple sounding plot saved to {output_file}")


def create_sharppy_gui_image(prof, output_file='sounding.png', title='Atmospheric Sounding'):
    """
    Use SHARPpy's full GUI to generate a comprehensive ~3MB image in headless mode.
    """
    try:
        # Set up headless Qt environment
        os.environ['QT_QPA_PLATFORM'] = 'offscreen'

        # Import Qt modules
        from qtpy.QtWidgets import QApplication
        from qtpy.QtCore import Qt

        # Create QApplication if it doesn't exist
        app = QApplication.instance()
        if app is None:
            app = QApplication([])

        # Set attributes for headless operation
        app.setAttribute(Qt.AA_UseHighDpiPixmaps, True)

        # Import SHARPpy components
        sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'api', 'SHARPpy', 'SHARPpy'))

        from sharppy.viz.SPCWindow import SPCWidget
        from sutils.config import Config
        import sharppy.sharptab.prof_collection as prof_collection
        from datetime import datetime

        # Create profile collection directly from the profile object
        print("Creating profile collection...")
        prof_coll = prof_collection.ProfCollection(
            {'':[ prof ]},
            [datetime.now()],
        )
        prof_coll.setMeta('loc', title.split()[-1] if len(title.split()) > 1 else "GRIB2")
        prof_coll.setMeta('observed', True)
        prof_coll.setMeta('base_time', datetime.now())

        # Create config with a temporary config file
        import tempfile
        config_file = tempfile.mktemp(suffix='.ini')
        config = Config(config_file)

        # Initialize config with default preferences
        from sharppy.viz.preferences import PrefDialog
        PrefDialog.initConfig(config)

        # Create SPCWidget (the main GUI component) without showing it
        print("Creating SHARPpy GUI widget...")
        spc_widget = SPCWidget(cfg=config)

        # Add the profile collection
        spc_widget.addProfileCollection(prof_coll, "GRIB2_Profile")

        # Give Qt time to layout the widget
        app.processEvents()

        # Capture the widget as an image
        print("Capturing GUI image...")
        pixmap = spc_widget.grab()

        # Save the image
        success = pixmap.save(output_file, 'PNG', 100)
        if success:
            print(f"SHARPpy GUI image saved to {output_file} ({os.path.getsize(output_file)} bytes)")
        else:
            print("Failed to save GUI image")

    except Exception as e:
        print(f"Error in GUI image generation: {e}")
        import traceback
        traceback.print_exc()


def generate_sounding_overview(grib2_path, lat, lon, output_file='sounding.png', title=None, use_gui=False):
    """
    Main function to generate a complete sounding overview from GRIB2 files.

    Parameters:
    grib2_path: Path to GRIB2 file(s) or directory containing GRIB2 files
    lat: Latitude of the sounding location
    lon: Longitude of the sounding location
    output_file: Output PNG file path
    title: Plot title (auto-generated if None)
    use_gui: If True, use full SHARPpy GUI for ~3MB comprehensive image; if False, use simple matplotlib plot
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
    create_sounding_plot(prof, output_file, title, use_gui=use_gui)

    return output_file


if __name__ == '__main__':
    # Example usage - test both simple and GUI versions
    grib2_dir = 'model_data/grib2/dd.weather.gc.ca/today/model_rdps/10km/00/000'
    lat, lon = 45.0, -75.0  # Ottawa, Canada

    try:
        # Test simple plot first
        print("=== Testing Simple Plot ===")
        simple_file = generate_sounding_overview(grib2_dir, lat, lon, output_file='sounding_simple.png', use_gui=False)
        print(f"Success! Simple plot saved to {simple_file}")

        # Test GUI plot (may not work in headless environment)
        print("\n=== Testing GUI Plot ===")
        gui_file = generate_sounding_overview(grib2_dir, lat, lon, output_file='sounding_gui.png', use_gui=True)
        print(f"Success! GUI plot saved to {gui_file}")

    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
