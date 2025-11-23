# SHARPpy compilation

## FOR LINUX

Create a ``python 3.9`` virtual environment in this directory:

```bash
mkdir venv
python3.9 -m venv venv/
source venv/bin/activate
```

Install dependencies:
```bash
pip install numpy==1.* matplotlib qtpy pyside2 requests python-dateutil pyinstaller
```

Then install sharppy

```bash
pip install sharppy --no-deps
```

- ### Test that everything is working properly

```bash
python create_sounding_gfs.py
```
You should then see appear a new ``sounding_gfs.png`` file in this directory

- ### Compile SHARPpy

Compile SHARPpy using pyinstaller, this will create a new executable at ``dist/create_sounding_gfs ``
```bash
pyinstaller --onefile --hidden-import=sharppy --add-data "venv/lib/python3.9/site-packages/sharppy/databases:sharppy/databases" create_sounding_gfs.py
```

## FOR WINDOWS

Create a ``python 3.9`` virtual environment in this directory:

Create the ``venv`` folder in api\SHARPpy and navigate to api\SHARPpy (in CMD)

make sure you are using python 3.9.*
```bash
python -v
```
should output ``3.9.*``, if so, continue with the process
```bash
python -m venv venv
venv\Scripts\Activate.bat
```

Install dependencies:
```bash
pip install numpy==1.* matplotlib qtpy pyside2 requests python-dateutil pyinstaller
```

Then install sharppy

```bash
pip install sharppy --no-deps
```

### **Make sure to move the .sharppy folder in ``C:\Users\"YOUR USER"\``**

- ### Test that everything is working properly

```bash
python create_sounding_gfs.py
```
You should then see appear a new ``sounding_gfs.png`` file in this directory

- ### Compile SHARPpy

Compile SHARPpy using pyinstaller, this will create a new executable at ``dist/create_sounding_gfs.exe ``
```bash
pyinstaller --onefile --hidden-import=sharppy --add-data "venv\lib\site-packages\sharppy\databases:sharppy\databases" create_sounding_gfs.py
```