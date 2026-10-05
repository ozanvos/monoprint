#!/usr/bin/env python3
"""Capture isolated native demo windows; never load the user's saved session.

Requires macOS Screen Recording permission and an existing --app-store bundle.
No UI automation session is used, so macOS recording controls stay out of images.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import tempfile
import time


def main():
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bundle', type=Path,
                        default=root / 'target/gui-app-store/Monoprint.app')
    parser.add_argument('--window-list-helper', type=Path, required=True,
                        help='Screenshot skill take_screenshot.py location')
    parser.add_argument('--output', type=Path,
                        default=root / 'output/app-store-capture/native')
    parser.add_argument('--fixture', help='Capture only this fixture stem')
    parser.add_argument('--executable', type=Path,
                        help='Optional capture-only executable using the same app UI')
    parser.add_argument('--interface', choices=['dark', 'light'], default='dark',
                        help='Interface selected in the capture executable; does not retouch pixels')
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='monoprint-store-assets-') as tmp:
        capture_name = f'Monoprint Asset Capture {Path(tmp).name.rsplit("-", 1)[-1]}'
        bundle = Path(tmp) / f'{capture_name}.app'
        shutil.copytree(args.bundle, bundle)
        executable = bundle / 'Contents/MacOS/monoprint-gui'
        if args.executable:
            shutil.copy2(args.executable, executable)
        info_path = bundle / 'Contents/Info.plist'
        info = plistlib.loads(info_path.read_bytes())
        info['CFBundleIdentifier'] = 'app.monoprint.studio.assets-native'
        info['CFBundleName'] = capture_name
        info['CFBundleDisplayName'] = capture_name
        info_path.write_bytes(plistlib.dumps(info))
        subprocess.run(['codesign', '--force', '--deep', '--sign', '-', str(bundle)], check=True)
        fixtures = sorted((root / 'release/app-store/fixtures').glob('*.json'))
        if args.fixture:
            fixtures = [fixture for fixture in fixtures if fixture.stem == args.fixture]
            if not fixtures:
                parser.error('Unknown fixture')
        for fixture in fixtures:
            session = Path(tmp) / 'session.json'
            shutil.copyfile(fixture, session)
            listing = ['python3', str(args.window_list_helper), '--list-windows',
                       '--app', 'Monoprint Asset Capture']
            before = subprocess.run(listing, check=True, capture_output=True, text=True)
            existing_ids = {row.split('\t')[0] for row in before.stdout.splitlines()}
            env = os.environ | {'MONOPRINT_SESSION': str(session)}
            process = subprocess.Popen([str(bundle / 'Contents/MacOS/monoprint-gui')], env=env)
            try:
                deadline = time.monotonic() + 20
                window_id = None
                while time.monotonic() < deadline:
                    if process.poll() is not None:
                        raise RuntimeError('Capture app exited before rendering')
                    result = subprocess.run(listing, check=True, capture_output=True, text=True)
                    for row in result.stdout.splitlines():
                        columns = row.split('\t')
                        if (len(columns) == 4 and columns[2] == 'Monoprint'
                                and columns[0] not in existing_ids):
                            window_id = columns[0]
                    if window_id:
                        break
                    time.sleep(0.2)
                if not window_id:
                    raise RuntimeError('Native capture window did not appear')
                # Preview parsing/rendering runs asynchronously after window creation.
                time.sleep(3)
                destination = args.output / f'{fixture.stem}.png'
                subprocess.run(['screencapture', '-x', '-o', '-a', f'-l{window_id}',
                                str(destination)], check=True)
                provenance = {
                    'interfaceAppearance': args.interface,
                    'captureBinarySHA256': hashlib.sha256(executable.read_bytes()).hexdigest(),
                    'startup': ('capture-only copy; existing appearance::apply(true, None, cx) '
                                'selected at startup' if args.executable else 'unchanged production app'),
                }
                destination.with_suffix('.json').write_text(json.dumps(provenance, indent=2) + '\n')
                print(destination, flush=True)
            finally:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
    print(json.dumps({'capture': 'native macOS window', 'fixtures': len(fixtures)}))


if __name__ == '__main__':
    main()
