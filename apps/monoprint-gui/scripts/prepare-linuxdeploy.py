#!/usr/bin/env python3
"""Fetch the pinned linuxdeploy x64 tool and verify its SHA-256."""
import argparse
import hashlib
from pathlib import Path
import sys
import tempfile
import urllib.request

root = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(root / 'scripts'))
from desktop_updates import configuration

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', required=True, type=Path)
args = parser.parse_args()
if args.output.exists():
    parser.error('--output must be a fresh path')
config = configuration()['linuxdeploy']
url = f"https://github.com/linuxdeploy/linuxdeploy/releases/download/{config['version']}/linuxdeploy-x86_64.AppImage"
args.output.parent.mkdir(parents=True, exist_ok=True)
with tempfile.NamedTemporaryFile(dir=args.output.parent) as temporary:
    urllib.request.urlretrieve(url, temporary.name)
    if hashlib.sha256(Path(temporary.name).read_bytes()).hexdigest() != config['archiveSha256']:
        raise RuntimeError('linuxdeploy checksum mismatch')
    # Refuse to clobber an output created while downloading.
    with args.output.open('xb') as output:
        output.write(Path(temporary.name).read_bytes())
args.output.chmod(0o755)
print(args.output)
