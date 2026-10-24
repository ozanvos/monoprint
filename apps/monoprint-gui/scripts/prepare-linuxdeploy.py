#!/usr/bin/env python3
"""Fetch the pinned linuxdeploy tool for a Linux runtime and verify its SHA-256."""
import argparse
import hashlib
from pathlib import Path
import sys
import tempfile
import urllib.request

root = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(root / 'scripts'))
from desktop_updates import linuxdeploy_settings

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', required=True, type=Path)
parser.add_argument('--runtime', choices=['linux-x64', 'linux-arm64'], default='linux-x64')
args = parser.parse_args()
if args.output.exists():
    parser.error('--output must be a fresh path')
url, digest = linuxdeploy_settings(args.runtime)
args.output.parent.mkdir(parents=True, exist_ok=True)
with tempfile.NamedTemporaryFile(dir=args.output.parent) as temporary:
    urllib.request.urlretrieve(url, temporary.name)
    if hashlib.sha256(Path(temporary.name).read_bytes()).hexdigest() != digest:
        raise RuntimeError('linuxdeploy checksum mismatch')
    # Refuse to clobber an output created while downloading.
    with args.output.open('xb') as output:
        output.write(Path(temporary.name).read_bytes())
args.output.chmod(0o755)
print(args.output)
