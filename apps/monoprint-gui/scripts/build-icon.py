#!/usr/bin/env python3
"""Compile the Icon Composer source and macOS fallback using Apple's asset compiler."""
import argparse
from pathlib import Path
import subprocess
import sys
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    gui = Path(__file__).resolve().parents[1]
    parser.add_argument('--output', type=Path, default=gui.parents[1] / 'target/gui/icon')
    args = parser.parse_args()
    if sys.platform != 'darwin':
        parser.error('Compiling the layered icon requires macOS and Xcode 26 or later.')
    with tempfile.TemporaryDirectory(prefix='monoprint-icon-') as directory:
        compiled = Path(directory)
        subprocess.run([
            'xcrun', 'actool', str(gui / 'assets/Monoprint.icon'),
            '--compile', str(compiled), '--app-icon', 'Monoprint',
            '--platform', 'macosx', '--target-device', 'mac',
            '--minimum-deployment-target', '12.0',
            '--output-partial-info-plist', str(compiled / 'icon-info.plist'),
            '--output-format', 'human-readable-text',
        ], check=True)
        # Read both artifacts before updating the destination. Missing fallback
        # generation or an unsupported compiler fails instead of using a stale icon.
        artifacts = {name: (compiled / name).read_bytes()
                     for name in ('Assets.car', 'Monoprint.icns')}
        args.output.mkdir(parents=True, exist_ok=True)
        for name, data in artifacts.items():
            (args.output / name).write_bytes(data)
    print(args.output)


if __name__ == '__main__':
    main()
