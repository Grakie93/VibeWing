# Third-Party Notices

VibeWing includes and packages third-party open-source software. Each component remains subject to its own license.

The authoritative dependency versions are recorded in `package-lock.json`, `requirements-build.txt`, and the packaged Electron/PyInstaller artifacts. Before each stable release, regenerate and review a complete production dependency license report rather than treating this file as a static hand-maintained package inventory.

Major build and runtime components currently include:

- Electron — MIT License
- electron-builder and its dependencies — licenses declared by their respective packages
- PyInstaller — GPL-2.0-or-later with a special exception permitting distribution of bundled applications
- Python standard library — Python Software Foundation License

VibeWing does not claim ownership of third-party names, trademarks, or source code. Copies of license texts required by packaged dependencies must be included in release artifacts where their licenses require it.

If a required notice is missing, open an issue with the package name, version, and applicable license.
