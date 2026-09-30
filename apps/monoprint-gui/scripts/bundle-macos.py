#!/usr/bin/env python3
"""Build a local macOS .app, keeping UI dependencies and notices isolated."""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tomllib

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--debug', action='store_true', help='Use the faster development build')
parser.add_argument('--sign-identity', default='-', help='Developer ID Application identity; defaults to local ad-hoc signing')
parser.add_argument('--notary-profile', help='Existing notarytool keychain profile; requires Developer ID signing')
args = parser.parse_args()
if sys.platform != 'darwin':
    parser.error('This bundle script is for macOS; use cargo run on other platforms.')
if args.notary_profile and (args.sign_identity == '-' or args.debug):
    parser.error('Notarization requires a release build and --sign-identity')
gui = Path(__file__).resolve().parents[1]
root = gui.parents[1]
target = root / 'target/gui'
manifest = gui / 'Cargo.toml'
subprocess.run(['cargo', 'build', '--locked', '--manifest-path', str(manifest),
                '--target-dir', str(target), *([] if args.debug else ['--release'])], check=True)
# The shortcuts use a stable bundled executable, never the interactive shell's PATH.
subprocess.run(['cargo', 'build', '--locked', '--manifest-path', str(root / 'Cargo.toml'),
                '--bin', 'monoprint', '--target-dir', str(target), *([] if args.debug else ['--release'])], check=True)
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
shutil.copy2(target / ('debug' if args.debug else 'release') / 'monoprint', contents / 'MacOS/monoprint')
shutil.copy2(gui / 'scripts/monoprint-shortcut', contents / 'MacOS/monoprint-shortcut')
(contents / 'MacOS/monoprint-shortcut').chmod(0o755)
subprocess.run([sys.executable, str(gui / 'scripts/build-shortcuts.py'), '--output', str(gui / 'assets/shortcuts'), '--check'], check=True)
shortcuts = resources / 'Shortcuts'
if shortcuts.exists():
    shutil.rmtree(shortcuts)
shortcuts.mkdir()
for path in (gui / 'assets/shortcuts').glob('*.shortcut'):
    shutil.copy2(path, shortcuts / path.name)
(contents / 'Info.plist').write_bytes(plistlib.dumps({
    'CFBundleName': 'Monoprint', 'CFBundleDisplayName': 'Monoprint',
    'CFBundleIdentifier': 'app.monoprint.studio', 'CFBundleExecutable': 'monoprint-gui',
    'CFBundlePackageType': 'APPL', 'CFBundleIconFile': 'Monoprint.icns', 'CFBundleShortVersionString': version,
    'CFBundleVersion': version, 'NSHighResolutionCapable': True,
    'LSMinimumSystemVersion': '12.0',
    'CFBundleDocumentTypes': [{
        'CFBundleTypeName': 'UTF-8 Source Code', 'CFBundleTypeRole': 'Viewer',
        'LSHandlerRank': 'Alternate',
        'LSItemContentTypes': ['public.source-code', 'public.plain-text', 'public.json', 'public.xml'],
    }],
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
lock = tomllib.loads((gui / 'Cargo.lock').read_text())
checksums = {(p['name'], p['version']): p.get('checksum') for p in lock['package']}
for package in sorted(metadata['packages'], key=lambda p: (p['name'], p['version'])):
    if not package.get('source'):
        continue
    source = Path(package['manifest_path']).parent
    files = sorted(p for p in source.rglob('*') if p.is_file() and p.name.upper().startswith(('LICENSE', 'LICENCE', 'COPYING', 'NOTICE')))
    record = {k: package[k] for k in ['name', 'version', 'license', 'repository']} | {
        'source': f"https://crates.io/api/v1/crates/{package['name']}/{package['version']}/download",
        'notice_files': [str(p.relative_to(source)) for p in files],
    }
    if package.get('license') in ('EUPL-1.2', 'MPL-2.0'):
        archive_name = f"{package['name']}-{package['version']}.crate"
        archive = source.parents[2] / 'cache' / source.parent.name / archive_name
        source_bytes = archive.read_bytes()
        checksum = hashlib.sha256(source_bytes).hexdigest()
        if checksum != checksums[(package['name'], package['version'])]:
            raise RuntimeError(f'Corresponding source checksum mismatch: {archive_name}')
        destination = resources / 'licenses/sources' / archive_name
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(source_bytes)
        record['bundled_source'] = f'sources/{archive_name}'
        record['sha256'] = checksum
    inventory.append(record)
    for path in files:
        notices.append(f"\n{'=' * 72}\n{package['name']} {package['version']} — {path.relative_to(source)}\n{'=' * 72}\n" + path.read_text(errors='replace'))
(resources / 'licenses/gui-dependencies.json').write_text(json.dumps(inventory, indent=2) + '\n')
(resources / 'licenses/gui-dependency-notices.txt').write_text('Packaged notices from the optional GUI lockfile, including host build/test dependencies.\n' + ''.join(notices))
(resources / 'licenses/sources/README.md').write_text(
    '# Corresponding dependency source\n\nUnmodified Giallo (EUPL-1.2) and MPL-2.0 dependency source archives accompany this app. '
    'Extract each `.crate` as a gzip-compressed tar archive. Versions, checksums and exact upstream source URLs are in '
    '`../gui-dependencies.json`; license texts are in `../gui-dependency-notices.txt`. MPL grammar sources are in '
    '`../grammar-sources/`. Original third-party licenses remain applicable.\n')
shutil.copy2(manifest.parent / 'Cargo.lock', resources / 'gui-Cargo.lock')
# Local builds retain ad-hoc signing. Public distribution uses a Developer ID.
subprocess.run(['codesign', '--force', '--deep', '--sign', args.sign_identity,
                *([] if args.sign_identity == '-' else ['--options', 'runtime', '--timestamp']),
                str(bundle)], check=True)
subprocess.run(['codesign', '--verify', '--deep', '--strict', str(bundle)], check=True)
if args.notary_profile:
    archive = target / 'Monoprint-notarization.zip'
    if archive.exists():
        archive.unlink()
    subprocess.run(['ditto', '-c', '-k', '--keepParent', str(bundle), str(archive)], check=True)
    result = json.loads(subprocess.check_output([
        'xcrun', 'notarytool', 'submit', str(archive), '--keychain-profile', args.notary_profile,
        '--wait', '--output-format', 'json'], text=True))
    if result.get('status') != 'Accepted':
        raise RuntimeError(f"Notarization was not accepted: {result.get('status')}")
    subprocess.run(['xcrun', 'stapler', 'staple', str(bundle)], check=True)
    subprocess.run(['xcrun', 'stapler', 'validate', str(bundle)], check=True)
print(bundle)
