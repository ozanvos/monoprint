#!/usr/bin/env python3
"""Package assets/icon.png at all standard macOS icon sizes using system tools."""
from pathlib import Path
import subprocess
import sys
import tempfile


def main():
    if sys.platform != 'darwin':
        raise SystemExit('Building the .icns asset requires macOS sips and iconutil.')
    assets = Path(__file__).resolve().parents[1] / 'assets'
    source = assets / 'icon.png'
    destination = assets / 'Monoprint.icns'
    with tempfile.TemporaryDirectory(prefix='monoprint-icon-') as directory:
        iconset = Path(directory) / 'Monoprint.iconset'
        iconset.mkdir()
        for size in (16, 32, 128, 256, 512):
            for scale in (1, 2):
                pixels = str(size * scale)
                suffix = '@2x' if scale == 2 else ''
                output = iconset / f'icon_{size}x{size}{suffix}.png'
                subprocess.run([
                    'sips', '-z', pixels, pixels, str(source), '--out', str(output),
                ], check=True, stdout=subprocess.DEVNULL)
        # Complete the container before replacing the checked-in bundle asset.
        compiled = Path(directory) / 'Monoprint.icns'
        subprocess.run(['iconutil', '-c', 'icns', str(iconset), '-o', str(compiled)], check=True)
        destination.write_bytes(compiled.read_bytes())
    print(destination)


if __name__ == '__main__':
    main()
