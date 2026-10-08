#!/usr/bin/env python3
"""Download the pinned Sparkle SDK, verify its checksum, then extract it."""
import argparse
import hashlib
from pathlib import Path
import shutil
import sys
import tarfile
import tempfile
import urllib.request

root = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(root / 'scripts'))
from desktop_updates import configuration

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', required=True, type=Path)
args = parser.parse_args()
if args.output.exists():
    parser.error('--output must be a fresh directory')
config = configuration()['sparkle']
url = f"https://github.com/sparkle-project/Sparkle/releases/download/{config['version']}/Sparkle-{config['version']}.tar.xz"
with tempfile.TemporaryDirectory() as temporary:
    archive = Path(temporary) / 'sparkle.tar.xz'
    urllib.request.urlretrieve(url, archive)
    if hashlib.sha256(archive.read_bytes()).hexdigest() != config['archiveSha256']:
        raise RuntimeError('Sparkle archive checksum mismatch')
    extracted = Path(temporary) / 'sdk'
    extracted.mkdir()
    with tarfile.open(archive) as compressed:
        compressed.extractall(extracted, filter='data')
    for name in ['Sparkle.framework', 'bin/generate_appcast', 'bin/sign_update', 'LICENSE']:
        if not (extracted / name).exists():
            raise RuntimeError(f'Sparkle SDK is missing {name}')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    shutil.move(str(extracted), str(args.output))
print(args.output)
