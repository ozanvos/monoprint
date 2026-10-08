#!/usr/bin/env python3
"""Prepare a signed Sparkle feed from a verified, notarized release ZIP. No upload."""
import argparse
import hashlib
import json
import plistlib
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
import zipfile

root = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(root / 'scripts'))
from desktop_updates import https_url

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--from', dest='source', type=Path, required=True)
parser.add_argument('--sparkle-sdk', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True, help='Fresh staging directory')
parser.add_argument('--download-url', required=True, help='HTTPS directory where the ZIP and feed will be hosted')
parser.add_argument('--ed-key-file', type=Path, help='Private signing key; otherwise use Sparkle keychain account')
args = parser.parse_args()
url = https_url(args.download_url).rstrip('/') + '/'
receipt = json.loads((args.source / 'manifest.json').read_text())
if not all([receipt.get('release'), not receipt.get('dirty'), receipt.get('updates'), receipt.get('signed'),
            receipt.get('stapled'), (receipt.get('notarization') or {}).get('status') == 'Accepted']):
    parser.error('Feed generation requires a clean, update-enabled, signed and notarized release')
if receipt.get('appStore'):
    parser.error('Store builds cannot enter a Sparkle feed')
archive_name = receipt['archive']
if Path(archive_name).name != archive_name:
    parser.error('Unsafe archive name')
archive = args.source / archive_name
if hashlib.sha256(archive.read_bytes()).hexdigest() != receipt['archiveSha256']:
    parser.error('Archive checksum mismatch')
with zipfile.ZipFile(archive) as zipped:
    info = plistlib.loads(zipped.read('Monoprint.app/Contents/Info.plist'))
if info['CFBundleShortVersionString'] != receipt['version'] or info.get('SUFeedURL') != receipt['updateFeed']:
    parser.error('Embedded app version/feed differs from the receipt')
public_key = info['SUPublicEDKey']
if args.output.exists():
    parser.error('--output must be a fresh directory')
args.output.mkdir(parents=True)
shutil.copy2(archive, args.output / archive_name)
notes = root / 'release/notes' / f"{receipt['version']}.md"
if notes.is_file():
    shutil.copy2(notes, args.output / Path(archive_name).with_suffix('.md'))
command = [str((args.sparkle_sdk / 'bin/generate_appcast').resolve()), '--download-url-prefix', url,
           '--embed-release-notes', '--maximum-deltas', '0', '-o', str((args.output / 'appcast.xml').resolve())]
if args.ed_key_file:
    command += ['--ed-key-file', str(args.ed_key_file.resolve())]
command.append(str(args.output.resolve()))
subprocess.run(command, check=True)
appcast = args.output / 'appcast.xml'
items = ET.parse(appcast).findall('./channel/item')
if not items or any(not item.find('enclosure').get('{http://www.andymatuschak.org/xml-namespaces/sparkle}edSignature') for item in items):
    raise RuntimeError('Generated appcast is missing signed update enclosures')
for item in items:
    enclosure = item.find('enclosure')
    if int(enclosure.get('length')) != archive.stat().st_size:
        raise RuntimeError('Appcast enclosure size differs from the shipping archive')
    signature = enclosure.get('{http://www.andymatuschak.org/xml-namespaces/sparkle}edSignature')
    with tempfile.TemporaryDirectory(prefix='monoprint-signature-cache-') as cache:
        subprocess.run(['xcrun', 'swift', '-module-cache-path', cache,
                        str(root / 'scripts/verify-sparkle-signature.swift'),
                        str(archive.resolve()), signature, public_key], check=True)
print(appcast)
