"""Verify a direct-download DMG and relocated app ZIP, preserving code-signature extended attributes."""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import subprocess
import tempfile
import zipfile

from macos_release import verify_code

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--from', dest='source', required=True, type=Path)
parser.add_argument('--require-notarized', action='store_true')
args = parser.parse_args()
receipt = json.loads((args.source / 'manifest.json').read_text())
assert Path(receipt['archive']).name == receipt['archive']
archive = (args.source / receipt['archive']).resolve()
assert hashlib.sha256(archive.read_bytes()).hexdigest() == receipt['archiveSha256']
with zipfile.ZipFile(archive) as zipped:
    assert all(not Path(name).is_absolute() and '..' not in Path(name).parts for name in zipped.namelist())
with tempfile.TemporaryDirectory(prefix='monoprint-app-consumer-') as temporary:
    folder = Path(temporary)
    # Python zip extraction drops extended attributes, including signed scripts.
    subprocess.run(['ditto', '-x', '-k', str(archive), str(folder)], check=True)
    app = folder / 'Monoprint.app'
    verify_code(app)
    info = plistlib.loads((app / 'Contents/Info.plist').read_bytes())
    assert info['CFBundleShortVersionString'] == receipt['version']
    assert info['CFBundleIconName'] == 'Monoprint'
    framework = app / 'Contents/Frameworks/Sparkle.framework'
    if receipt.get('updates'):
        assert not receipt.get('appStore')
        assert info['SUFeedURL'] == receipt['updateFeed']
        assert info['SUPublicEDKey'] and framework.is_dir()
        assert (framework / 'Sparkle').is_symlink()
        assert (framework / 'Versions/Current').is_symlink()
        framework_info = plistlib.loads((framework / 'Resources/Info.plist').read_bytes())
        assert framework_info['CFBundleShortVersionString'] == receipt['sparkleVersion']
    else:
        assert not framework.exists()
        assert 'SUFeedURL' not in info and 'SUPublicEDKey' not in info
    resources = app / 'Contents/Resources'
    for name in ['Assets.car', 'Monoprint.icns', 'LICENSE', 'THIRD_PARTY_NOTICES.md',
                 'licenses/gui-dependencies.json', 'licenses/grammar-sources/hcl.json']:
        assert (resources / name).is_file(), name
    for entry in json.loads((resources / 'licenses/gui-dependencies.json').read_text()):
        if 'bundled_source' in entry:
            source = resources / 'licenses' / entry['bundled_source']
            assert hashlib.sha256(source.read_bytes()).hexdigest() == entry['sha256']
    snippet = folder / 'sample.ts'
    snippet.write_text('const message = "你好 👩🏽‍💻";\n')
    subprocess.run([str(app / 'Contents/MacOS/monoprint'), str(snippet), '--scale', '1',
                    '-o', str(folder / 'sample.png')], check=True)
    if args.require_notarized:
        assert receipt['signed'] and receipt['stapled'] and receipt['notarization']['status'] == 'Accepted'
        subprocess.run(['xcrun', 'stapler', 'validate', str(app)], check=True)
        subprocess.run(['spctl', '--assess', '--type', 'execute', str(app)], check=True)
    if not receipt.get('appStore'):
        name = receipt['diskImage']
        assert Path(name).name == name and name.endswith('.dmg')
        image = (args.source / name).resolve()
        assert image.stat().st_size == receipt['diskImageBytes']
        assert hashlib.sha256(image.read_bytes()).hexdigest() == receipt['diskImageSha256']
        verify_code(image)
        if args.require_notarized:
            assert receipt['diskImageStapled'] and receipt['diskImageNotarization']['status'] == 'Accepted'
            subprocess.run(['xcrun', 'stapler', 'validate', str(image)], check=True)
            subprocess.run(['spctl', '--assess', '--type', 'open', '--context',
                            'context:primary-signature', str(image)], check=True)
        mount = folder / 'mounted'
        subprocess.run(['hdiutil', 'attach', '-readonly', '-nobrowse', '-mountpoint', str(mount), str(image)], check=True)
        try:
            from ds_store import DSStore
            with DSStore.open(str(mount / '.DS_Store'), 'r') as settings:
                assert settings['Monoprint.app']['Iloc'] == (165, 176)
                assert settings['Applications']['Iloc'] == (435, 176)
                view = settings['.']['icvp']
                assert view['iconSize'] == 128 and view['textSize'] == 12
                assert view['backgroundType'] == 2 and view['backgroundImageAlias']
                window = settings['.']['bwsp']
                assert not window['ShowToolbar'] and not window['ShowSidebar']
                assert window['WindowBounds'] == '{{200, 200}, {600, 380}}'
            artwork = Path(__file__).resolve().parents[1] / 'apps/monoprint-gui/assets/dmg/proof-sheet.tiff'
            assert (mount / '.background.tiff').read_bytes() == artwork.read_bytes()
            assert (mount / 'Applications').is_symlink()
            assert (mount / 'Applications').readlink() == Path('/Applications')
            # Copy out as a user would; verify signatures and exact ZIP app contents.
            installed = folder / 'installed/Monoprint.app'
            installed.parent.mkdir()
            subprocess.run(['ditto', str(mount / 'Monoprint.app'), str(installed)], check=True)
            subprocess.run(['diff', '-r', str(app), str(installed)], check=True)
            verify_code(installed)
            if args.require_notarized:
                subprocess.run(['xcrun', 'stapler', 'validate', str(installed)], check=True)
                subprocess.run(['spctl', '--assess', '--type', 'execute', str(installed)], check=True)
            subprocess.run([str(installed / 'Contents/MacOS/monoprint'), str(snippet), '--scale', '1',
                            '-o', str(folder / 'dmg-sample.png')], check=True)
        finally:
            subprocess.run(['hdiutil', 'detach', str(mount)], check=True)
print('DMG and relocated ZIP signature, embedded icon, source archives and bundled CLI passed.')
