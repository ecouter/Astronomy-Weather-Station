#!/usr/bin/env python3
"""
SHARPpy Sounding Generator from Online GFS BUFKIT Data

This script downloads GFS BUFKIT data from online sources and creates
a complete atmospheric sounding overview using SHARPpy.
"""

import os
import sys
import numpy as np
import matplotlib
matplotlib.use('Agg')  # Use non-interactive backend
import matplotlib.pyplot as plt
from datetime import datetime, timedelta

# Add SHARPpy to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'SHARPpy'))

import sharppy
import sharppy.sharptab.profile as profile
import sharppy.sharptab.interp as interp
import sharppy.sharptab.winds as winds
import sharppy.sharptab.utils as utils
import sharppy.sharptab.params as params
import sharppy.sharptab.thermo as thermo

# Import SHARPpy data source modules
from datasources import data_source

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


def find_nearest_station(data_source, lat, lon, cycle_time=None):
    """
    Find the nearest GFS station to the given coordinates.

    Parameters:
    data_source: SHARPpy DataSource object
    lat: Latitude
    lon: Longitude
    cycle_time: datetime object to check for available stations

    Returns:
    Station dictionary with location info
    """
    if cycle_time is None:
        cycle_time = datetime.utcnow()

    # Get all available stations for the given cycle
    try:
        stations = data_source.getAvailableAtTime(cycle_time)
    except Exception as e:
        print(f"Error getting available stations: {e}")
        # Fallback: use all points from the CSV
        stations = data_source.getPoints()
        print(f"Using fallback station list with {len(stations)} stations")

    if not stations:
        raise ValueError("No GFS stations available")

    # Find nearest station
    min_distance = float('inf')
    nearest_station = None

    for station in stations:
        station_lat = station['lat']
        station_lon = station['lon']

        # Calculate distance using haversine formula
        dlat = np.radians(station_lat - lat)
        dlon = np.radians(station_lon - lon)
        a = np.sin(dlat/2)**2 + np.cos(np.radians(lat)) * np.cos(np.radians(station_lat)) * np.sin(dlon/2)**2
        c = 2 * np.arctan2(np.sqrt(a), np.sqrt(1-a))
        distance = 6371 * c  # Earth radius in km

        if distance < min_distance:
            min_distance = distance
            nearest_station = station

    print(f"Nearest GFS station: {nearest_station['icao']} at {nearest_station['lat']:.2f}N, {nearest_station['lon']:.2f}E (distance: {min_distance:.1f} km)")
    return nearest_station


def get_latest_gfs_cycle(data_source):
    """
    Get the most recent available GFS cycle.

    Parameters:
    data_source: SHARPpy DataSource object

    Returns:
    datetime object of the latest cycle
    """
    try:
        latest_cycle = data_source.getMostRecentCycle()
        print(f"Latest GFS cycle: {latest_cycle}")
        return latest_cycle
    except Exception as e:
        print(f"Error getting latest cycle: {e}")
        # Fallback to current time rounded to nearest 6 hours
        now = datetime.utcnow()
        hour = (now.hour // 6) * 6
        cycle_time = now.replace(hour=hour, minute=0, second=0, microsecond=0)
        print(f"Using fallback cycle: {cycle_time}")
        return cycle_time


def download_gfs_profile(data_source, station, cycle_time):
    """
    Download and decode GFS BUFKIT data for a specific station and cycle.

    Parameters:
    data_source: SHARPpy DataSource object
    station: Station dictionary
    cycle_time: datetime object

    Returns:
    SHARPpy Profile object
    """
    try:
        # Get decoder class and URL
        decoder_class, url = data_source.getDecoderAndURL(station, cycle_time)
        print(f"Downloading GFS data from: {url}")

        # Create decoder instance and parse the data
        decoder_instance = decoder_class(url)
        prof_collection = decoder_instance.getProfiles()

        # Get the profile (usually the first/only profile in the collection)
        prof = prof_collection.getHighlightedProf()

        print(f"Successfully downloaded GFS profile for {station['icao']}")
        print(f"Profile location: {prof.location}")
        print(f"Profile time: {prof.date}")
        print(f"Profile has {len(prof.pres)} levels")

        return prof

    except Exception as e:
        print(f"Error downloading GFS data: {e}")
        raise


def create_sounding_plot(prof, output_file='sounding.png', title='GFS Atmospheric Sounding'):
    """
    Create a complete sounding overview plot using SHARPpy's full GUI.

    Parameters:
    prof: SHARPpy Profile object
    output_file: Output PNG file path
    title: Plot title
    """
    # Use full SHARPpy GUI to generate comprehensive image
    create_sharppy_gui_image(prof, output_file, title)


def create_sharppy_gui_image(prof, output_file='sounding.png', title='GFS Atmospheric Sounding'):
    """
    Create a comprehensive sounding plot using SHARPpy's complete GUI rendering system.
    This replicates the exact same output as the SHARPpy GUI when clicking "Generate Profiles".
    """
    try:
        print("Creating comprehensive GFS sounding plot using SHARPpy GUI...")

        # Initialize Qt application for off-screen rendering
        app = QApplication.instance()
        if app is None:
            app = QApplication([])

        # Create a ProfCollection to hold our profile (like the GUI does)
        import sharppy.sharptab.prof_collection as prof_collection
        prof_col = prof_collection.ProfCollection(
            {'':[ prof ]},
            [ prof.date ],
        )

        # Set metadata like the GUI does
        prof_col.setMeta('model', 'GFS')
        prof_col.setMeta('run', prof.date)
        prof_col.setMeta('base_time', prof.date)  # Base time for forecast hour calculation
        prof_col.setMeta('loc', title)
        prof_col.setMeta('fhour', 0)  # Analysis profile
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
            print(f"SHARPpy GUI GFS sounding plot saved to {output_file} ({file_size} bytes)")
        else:
            raise Exception(f"Failed to save image to {output_file}")

        # Clean up
        spc_window.close()

    except Exception as e:
        print(f"Error in SHARPpy GUI rendering: {e}")
        import traceback
        traceback.print_exc()


def generate_gfs_sounding(lat, lon, output_file='sounding_gfs.png', title=None):
    """
    Main function to generate a complete GFS sounding overview.

    Parameters:
    lat: Latitude of the sounding location
    lon: Longitude of the sounding location
    output_file: Output PNG file path
    title: Plot title (auto-generated if None)

    Returns:
    Output file path
    """
    print(f"Generating GFS sounding for location {lat}N, {lon}E")

    # Load SHARPpy data sources
    print("Loading SHARPpy data sources...")
    data_sources = data_source.loadDataSources()

    if 'GFS' not in data_sources:
        raise ValueError("GFS data source not available")

    gfs_source = data_sources['GFS']

    # Try to find available data by checking recent cycles
    prof = None
    station = None
    cycle_time = None

    # Try the last few cycles (current and previous)
    for hours_back in [0, 6, 12, 18, 24]:
        try:
            test_cycle = datetime.utcnow() - timedelta(hours=hours_back)
            test_cycle = test_cycle.replace(hour=(test_cycle.hour // 6) * 6, minute=0, second=0, microsecond=0)

            print(f"Trying cycle: {test_cycle}")

            # Get available stations for this cycle
            try:
                available_stations = gfs_source.getAvailableAtTime(test_cycle)
                if available_stations:
                    print(f"Found {len(available_stations)} available stations for cycle {test_cycle}")

                    # Find nearest available station
                    min_distance = float('inf')
                    nearest_station = None

                    for avail_station in available_stations:
                        station_lat = avail_station['lat']
                        station_lon = avail_station['lon']

                        # Calculate distance
                        dlat = np.radians(station_lat - lat)
                        dlon = np.radians(station_lon - lon)
                        a = np.sin(dlat/2)**2 + np.cos(np.radians(lat)) * np.cos(np.radians(station_lat)) * np.sin(dlon/2)**2
                        c = 2 * np.arctan2(np.sqrt(a), np.sqrt(1-a))
                        distance = 6371 * c  # Earth radius in km

                        if distance < min_distance:
                            min_distance = distance
                            nearest_station = avail_station

                    if nearest_station:
                        print(f"Nearest available station: {nearest_station['icao']} at {nearest_station['lat']:.2f}N, {nearest_station['lon']:.2f}E (distance: {min_distance:.1f} km)")

                        # Try to download the profile
                        try:
                            prof = download_gfs_profile(gfs_source, nearest_station, test_cycle)
                            station = nearest_station
                            cycle_time = test_cycle
                            break
                        except Exception as e:
                            print(f"Failed to download profile for {nearest_station['icao']}: {e}")
                            continue
            except Exception as e:
                print(f"Error checking cycle {test_cycle}: {e}")
                continue

        except Exception as e:
            print(f"Error with cycle {test_cycle}: {e}")
            continue

    if prof is None:
        raise ValueError("Could not find available GFS data for any recent cycle")

    # Generate title if not provided
    if title is None:
        title = f"GFS Sounding - {station['icao']} ({lat:.2f}N, {lon:.2f}E)"

    # Create and save plot
    create_sounding_plot(prof, output_file, title)

    return output_file


def test_gfs_sounding():
    """
    Test the GFS sounding generation.
    """
    print("=== Testing GFS Sounding Generation ===")

    # Test with Ottawa, Canada coordinates
    lat, lon = 48, -70.5

    try:
        output_file = generate_gfs_sounding(lat, lon, output_file='sounding_gfs.png')
        print(f"Success! GFS sounding saved to {output_file}")
        return True
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
        return False


if __name__ == '__main__':
    # Test GFS sounding generation
    success = test_gfs_sounding()
    if not success:
        sys.exit(1)
