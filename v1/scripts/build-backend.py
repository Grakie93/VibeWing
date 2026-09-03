#!/usr/bin/env python3
"""Build the VibeWing local backend for the current operating system."""
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
DIST=ROOT/'backend-dist'
WORK=ROOT/'backend-build'
SEP=';' if os.name=='nt' else ':'

def main():
    try:
        import PyInstaller  # noqa: F401
    except ImportError:
        raise SystemExit('PyInstaller is required. Run: python -m pip install -r requirements-build.txt')
    shutil.rmtree(DIST,ignore_errors=True); shutil.rmtree(WORK,ignore_errors=True)
    command=[
        sys.executable,'-m','PyInstaller','--noconfirm','--clean','--onefile',
        '--name','vibewing-backend','--distpath',str(DIST),'--workpath',str(WORK),
        '--specpath',str(WORK),
        '--add-data',f'{ROOT/"index.html"}{SEP}.',
        '--add-data',f'{ROOT/"assets"/"brand"}{SEP}assets/brand',
        str(ROOT/'app.py'),
    ]
    env=os.environ.copy(); env['PYINSTALLER_CONFIG_DIR']=str(WORK/'cache')
    subprocess.run(command,cwd=ROOT,env=env,check=True)
    print(f'Backend built: {DIST}')

if __name__=='__main__': main()
