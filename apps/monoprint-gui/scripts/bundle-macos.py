#!/usr/bin/env python3
"""Build a local macOS .app, keeping UI dependencies and notices isolated."""
import argparse
import json
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--debug', action='store_true', help='Use the faster development build')
args = parser.parse_args()
if sys.platform != 'darwin':
    parser.error('This bundle script is for macOS; use cargo run on other platforms.')
gui = Path(__file__).resolve().parents[1]
root = gui.parents[1]
target = root / 'target/gui'
manifest = gui / 'Cargo.toml'
subprocess.run(['cargo', 'build', '--locked', '--manifest-path', str(manifest),
                '--target-dir', str(target), *([] if args.debug else ['--release'])], check=True)
host = next(line.removeprefix('host: ') for line in subprocess.check_output(['rustc', '-vV'], text=True).splitlines() if line.startswith('host: '))
metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--offline',
    '--manifest-path', str(manifest), '--format-version', '1', '--filter-platform', host]))
version = next(p['version'] for p in metadata['packages'] if p['name'] == 'monoprint-gui')
bundle = target / 'Monoprint.app'
contents = bundle / 'Contents'
(contents / 'MacOS').mkdir(parents=True, exist_ok=True)
resources = contents / 'Resources'
resources.mkdir(exist_ok=True)
shutil.copy2(gui / 'assets/Monoprint.icns', resources / 'Monoprint.icns')
shutil.copy2(target / ('debug' if args.debug else 'release') / 'monoprint-gui', contents / 'MacOS/monoprint-gui')
(contents / 'Info.plist').write_bytes(plistlib.dumps({
    'CFBundleName': 'Monoprint', 'CFBundleDisplayName': 'Monoprint',
    'CFBundleIdentifier': 'app.monoprint.studio', 'CFBundleExecutable': 'monoprint-gui',
    'CFBundlePackageType': 'APPL', 'CFBundleIconFile': 'Monoprint.icns', 'CFBundleShortVersionString': version,
    'CFBundleVersion': version, 'NSHighResolutionCapable': True,
    'LSMinimumSystemVersion': '12.0',
}))
# Preserve renderer assets' notices and source obligations alongside this binary.
for name in ['LICENSE', 'THIRD_PARTY_NOTICES.md']:
    shutil.copy2(root / name, resources / name)
shutil.copytree(root / 'licenses', resources / 'licenses', dirs_exist_ok=True)
fonts = resources / 'licenses/fonts'
fonts.mkdir(exist_ok=True)
for path in (root / 'assets/fonts').glob('*-OFL.txt'):
    shutil.copy2(path, fonts / path.name)
# Fallback fonts are embedded in the executable; ship their provenance and
# license texts without duplicating the large font binaries in Resources.
fallback = root / 'assets/fonts/fallback'
for path in [fallback / 'manifest.json', fallback / 'NOTICE.txt', *fallback.rglob('*-OFL.txt')]:
    destination = fonts / 'fallback' / path.relative_to(fallback)
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(path, destination)
# Additional native UI dependencies have their own lockfile and provenance.
inventory = []
notices = []
for package in sorted(metadata['packages'], key=lambda p: (p['name'], p['version'])):
    if not package.get('source'):
        continue
    source = Path(package['manifest_path']).parent
    files = sorted(p for p in source.rglob('*') if p.is_file() and p.name.upper().startswith(('LICENSE', 'LICENCE', 'COPYING', 'NOTICE')))
    inventory.append({k: package[k] for k in ['name', 'version', 'license', 'repository']} | {
        'source': f"https://crates.io/api/v1/crates/{package['name']}/{package['version']}/download",
        'notice_files': [str(p.relative_to(source)) for p in files],
    })
    for path in files:
        notices.append(f"\n{'=' * 72}\n{package['name']} {package['version']} — {path.relative_to(source)}\n{'=' * 72}\n" + path.read_text(errors='replace'))
(resources / 'licenses/gui-dependencies.json').write_text(json.dumps(inventory, indent=2) + '\n')
(resources / 'licenses/gui-dependency-notices.txt').write_text('Packaged notices from the optional GUI lockfile, including host build/test dependencies.\n' + ''.join(notices))
shutil.copy2(manifest.parent / 'Cargo.lock', resources / 'gui-Cargo.lock')
# Local ad-hoc signing makes the copied bundle usable; this is not notarization.
subprocess.run(['codesign', '--force', '--deep', '--sign', '-', str(bundle)], check=True)
print(bundle)
