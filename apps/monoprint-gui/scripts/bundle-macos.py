#!/usr/bin/env python3
"""Build a local macOS .app, keeping UI dependencies and notices isolated."""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import sys
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--debug', action='store_true', help='Use the faster development build')
parser.add_argument('--app-store', action='store_true', help='Build with sandbox entitlements and private API exclusions')
parser.add_argument('--app-store-profile', type=Path, help='Mac App Store distribution provisioning profile')
parser.add_argument('--installer-identity', help='Mac App Store installer certificate (requires --app-store-profile and --archive-dir)')
parser.add_argument('--build-number', help='CFBundleVersion for App Store Connect; defaults to the package version')
parser.add_argument('--sign-identity', default='-', help='Code signing identity; defaults to local ad-hoc signing')
parser.add_argument('--notary-profile', help='Existing notarytool keychain profile; requires Developer ID signing')
parser.add_argument('--sign-keychain', type=Path)
parser.add_argument('--notary-keychain', type=Path)
parser.add_argument('--archive-dir', type=Path, help='Fresh directory for final ZIP, checksums and receipt')
parser.add_argument('--local', action='store_true', help='Allow dirty source; mark artifacts verification-only')
parser.add_argument('--updates', action='store_true', help='Embed Sparkle and enable signed updates for direct downloads')
parser.add_argument('--sparkle-sdk', type=Path, help='Checksum-verified SDK from prepare-sparkle.py')
parser.add_argument('--update-feed', help='Override release/updates.json Sparkle feed URL')
parser.add_argument('--update-public-key', help='Override release/updates.json Ed25519 public key (never the private key)')
args = parser.parse_args()
if sys.platform != 'darwin':
    parser.error('This bundle script is for macOS; use cargo run on other platforms.')
if args.notary_profile and (args.sign_identity == '-' or args.debug):
    parser.error('Notarization requires a release build and --sign-identity')
if args.app_store and args.notary_profile:
    parser.error('--app-store cannot be combined with Developer ID notarization')
if args.app_store and args.updates:
    parser.error('App Store builds cannot include Sparkle')
if (args.sparkle_sdk or args.update_feed or args.update_public_key) and not args.updates:
    parser.error('Sparkle arguments require --updates')
if args.updates and args.archive_dir and not args.local and not args.notary_profile:
    parser.error('Distributable update archives require Developer ID signing and notarization')
if (args.app_store_profile or args.installer_identity) and not args.app_store:
    parser.error('Store credentials require --app-store')
if args.app_store and args.sign_identity != '-' and not (args.app_store_profile and args.installer_identity and args.archive_dir):
    parser.error('Store distribution signing requires --app-store-profile, --installer-identity and --archive-dir; use ad-hoc signing for local sandbox tests')
if (args.app_store_profile or args.installer_identity) and (args.sign_identity == '-' or not args.app_store_profile or not args.installer_identity or not args.archive_dir):
    parser.error('A Store profile requires application and installer identities and --archive-dir')
if args.build_number and not re.fullmatch(r'[0-9]+(?:\.[0-9]+){0,2}', args.build_number):
    parser.error('--build-number must contain one to three numeric components')
gui = Path(__file__).resolve().parents[1]
root = gui.parents[1]
sys.path.insert(0, str(root / 'scripts'))
from macos_release import sign_app, notarize_app, zip_app, source_digest, verify_app_store_apis
from app_store_release import prepare_credentials, signed_entitlements, package_app, clear_bundle_quarantine
from desktop_updates import sparkle_settings, configuration
update_settings = sparkle_settings(args.update_feed, args.update_public_key) if args.updates else {}
if args.updates:
    if not args.sparkle_sdk or not (args.sparkle_sdk / 'Sparkle.framework').is_dir():
        parser.error('--updates requires --sparkle-sdk from prepare-sparkle.py')
    sdk_info = plistlib.loads((args.sparkle_sdk / 'Sparkle.framework/Resources/Info.plist').read_bytes())
    if sdk_info['CFBundleShortVersionString'] != configuration()['sparkle']['version']:
        parser.error('Sparkle SDK version does not match release/updates.json')
credentials = prepare_credentials(args.app_store_profile, 'app.monoprint.studio', args.sign_identity,
                                  args.installer_identity, args.sign_keychain) if args.app_store_profile else None
commit = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
dirty = bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=root, text=True).strip())
source_hash = source_digest(root)
if args.archive_dir and args.archive_dir.exists():
    parser.error('--archive-dir must be a fresh directory')
if args.archive_dir and dirty and not args.local:
    parser.error('Distribution archives require clean source; use --local for verification-only artifacts')
target = root / ('target/gui-app-store' if args.app_store else 'target/gui')
manifest = gui / 'Cargo.toml'
feature_args = ['--features', 'app-store'] if args.app_store else []
subprocess.run(['cargo', 'build', '--locked', '--manifest-path', str(manifest),
                '--target-dir', str(target), *feature_args, *([] if args.debug else ['--release'])], check=True)
if args.app_store:
    verify_app_store_apis(target / ('debug' if args.debug else 'release') / 'monoprint-gui')
# Shell-based Shortcuts launch the CLI outside the app's sandbox and cannot
# inherit its file-panel grants. Only ordinary builds include this bridge.
if not args.app_store:
    subprocess.run(['cargo', 'build', '--locked', '--manifest-path', str(root / 'Cargo.toml'),
                    '--bin', 'monoprint', '--target-dir', str(target), *([] if args.debug else ['--release'])], check=True)
host = next(line.removeprefix('host: ') for line in subprocess.check_output(['rustc', '-vV'], text=True).splitlines() if line.startswith('host: '))
metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--offline',
    '--manifest-path', str(manifest), '--format-version', '1', '--filter-platform', host, *feature_args]))
version = next(p['version'] for p in metadata['packages'] if p['name'] == 'monoprint-gui')
bundle = target / 'Monoprint.app'
# Assemble a fresh bundle: stale signed files/resources must not survive rebuilds.
staging = tempfile.TemporaryDirectory(prefix='monoprint-bundle-', dir=target)
assembled = Path(staging.name) / 'Monoprint.app'
contents = assembled / 'Contents'
(contents / 'MacOS').mkdir(parents=True, exist_ok=True)
resources = contents / 'Resources'
resources.mkdir(exist_ok=True)
if args.updates:
    framework = contents / 'Frameworks/Sparkle.framework'
    shutil.copytree(args.sparkle_sdk / 'Sparkle.framework', framework, symlinks=True)
    # Development headers/modules are unnecessary inside the shipped application.
    for name in ['Headers', 'Modules']:
        path = framework / name
        if path.is_symlink():
            path.unlink()
        versioned = framework / 'Versions/Current' / name
        if versioned.exists():
            shutil.rmtree(versioned)
subprocess.run([sys.executable, str(gui / 'scripts/build-icon.py'),
                '--output', str(resources)], check=True)
shutil.copy2(target / ('debug' if args.debug else 'release') / 'monoprint-gui', contents / 'MacOS/monoprint-gui')
if not args.app_store:
    shutil.copy2(target / ('debug' if args.debug else 'release') / 'monoprint', contents / 'MacOS/monoprint')
    shutil.copy2(gui / 'scripts/monoprint-shortcut', contents / 'MacOS/monoprint-shortcut')
    (contents / 'MacOS/monoprint-shortcut').chmod(0o755)
    subprocess.run([sys.executable, str(gui / 'scripts/build-shortcuts.py'), '--output', str(gui / 'assets/shortcuts'), '--check'], check=True)
    shortcuts = resources / 'Shortcuts'
    shortcuts.mkdir()
    for path in (gui / 'assets/shortcuts').glob('*.shortcut'):
        shutil.copy2(path, shortcuts / path.name)
(contents / 'Info.plist').write_bytes(plistlib.dumps({
    'CFBundleName': 'Monoprint', 'CFBundleDisplayName': 'Monoprint',
    'CFBundleIdentifier': 'app.monoprint.studio', 'CFBundleExecutable': 'monoprint-gui',
    'CFBundlePackageType': 'APPL', 'CFBundleIconFile': 'Monoprint.icns',
    'CFBundleIconName': 'Monoprint', 'CFBundleShortVersionString': version,
    'CFBundleVersion': args.build_number or version, 'NSHighResolutionCapable': True,
    'LSMinimumSystemVersion': '12.0',
    **update_settings,
    **({'LSApplicationCategoryType': 'public.app-category.developer-tools',
        'ITSAppUsesNonExemptEncryption': False,
        'NSHumanReadableCopyright': '2026 Seven East'} if args.app_store else {}),
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
if args.updates:
    shutil.copy2(args.sparkle_sdk / 'LICENSE', resources / 'licenses/Sparkle-LICENSE')
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
from gui_notices import package_notices
package_notices(root, gui, resources, metadata)
# Sign inner Mach-O executables first, then the enclosing app.
entitlements = gui / 'assets/app-store.entitlements' if args.app_store else None
helpers = gui / 'assets/app-store-helper.entitlements' if args.app_store else None
if credentials:
    shutil.copy2(args.app_store_profile, contents / 'embedded.provisionprofile')
    entitlements = signed_entitlements(entitlements, credentials, Path(staging.name) / 'store.entitlements')
if args.app_store:
    clear_bundle_quarantine(assembled)
sign_app(assembled, credentials['application'] if credentials else args.sign_identity,
         args.sign_keychain, entitlements, helpers)
notary = notarize_app(assembled, args.notary_profile, args.notary_keychain) if args.notary_profile else None
if commit != subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip():
    raise RuntimeError('Checkout revision changed while building')
if source_hash != source_digest(root):
    raise RuntimeError('Source inputs changed while building; rebuild the bundle')
final_dirty = bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=root, text=True).strip())
if args.archive_dir and final_dirty and not args.local:
    raise RuntimeError('Checkout changed while packaging')
if bundle.exists():
    shutil.rmtree(bundle)
shutil.move(str(assembled), str(bundle))
staging.cleanup()
if args.archive_dir:
    args.archive_dir.mkdir(parents=True)
    name = f'Monoprint-{version}-{host}{"-app-store" if args.app_store else ""}.zip'
    archive = args.archive_dir / name
    # The final archive is created after stapling; the submission ZIP is temporary.
    zip_app(bundle, archive)
    checksum = hashlib.sha256(archive.read_bytes()).hexdigest()
    (args.archive_dir / (name + '.sha256')).write_text(f'{checksum}  {name}\n')
    package = None
    if args.app_store:
        package = args.archive_dir / f'Monoprint-{version}-{host}-app-store.pkg'
        package_app(bundle, package, credentials['installer'] if credentials else None, args.sign_keychain)
        (args.archive_dir / (package.name + '.sha256')).write_text(f'{hashlib.sha256(package.read_bytes()).hexdigest()}  {package.name}\n')
    receipt = {'version': version, 'commit': commit, 'target': host,
        'appStore': args.app_store,
        'updates': bool(args.updates),
        'updateFeed': update_settings.get('SUFeedURL'),
        'sparkleVersion': configuration()['sparkle']['version'] if args.updates else None,
        'sandboxed': args.app_store,
        'storeDistributionSigned': bool(credentials),
        'installerPackage': package.name if package else None,
        'installerSha256': hashlib.sha256(package.read_bytes()).hexdigest() if package else None,
        'provisioningProfileUUID': credentials['profileUUID'] if credentials else None,
        'release': not args.local and not args.debug and (not args.app_store or bool(credentials)), 'dirty': final_dirty,
        'inputHash': source_hash,
        'signed': args.sign_identity != '-', 'notarization': notary,
        'stapled': bool(notary), 'archive': name, 'archiveSha256': checksum,
        'archiveBytes': archive.stat().st_size,
        'compiler': subprocess.check_output(['rustc', '-vV'], text=True).strip(),
        'xcode': subprocess.check_output(['xcodebuild', '-version'], text=True).strip()}
    (args.archive_dir / 'manifest.json').write_text(json.dumps(receipt, indent=2) + '\n')
print(bundle)
