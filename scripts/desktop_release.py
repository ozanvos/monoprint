"""Assemble source-bound desktop artifacts. Publishing is owned by CI."""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import tomllib
import xml.etree.ElementTree as ET
import zipfile

from macos_release import source_digest

ROOT = Path(__file__).resolve().parents[1]
SPARKLE = '{http://www.andymatuschak.org/xml-namespaces/sparkle}'


def read_json(path):
    return json.loads(path.read_text())


def assemble(source, output, *, version, commit, input_hash, repository, distribution):
    base = f'https://github.com/{repository}/releases'
    latest = base + '/latest/download/'
    tagged = base + f'/download/v{version}/'
    files = {}
    downloads = {}

    def add(folder, name, digest, size=None, destination=None):
        if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]*', name):
            raise ValueError('Unsafe artifact filename')
        file = source / folder / name
        if file.is_symlink() or not file.is_file():
            raise ValueError(f'Missing regular artifact: {name}')
        data = file.read_bytes()
        if hashlib.sha256(data).hexdigest() != digest or (size is not None and len(data) != size):
            raise ValueError(f'Artifact checksum/size mismatch: {name}')
        destination = destination or name
        if destination in files:
            raise ValueError(f'Duplicate artifact: {destination}')
        files[destination] = {'source': file, 'sha256': digest, 'bytes': len(data)}
        return destination

    for folder, runtime in [('macos-app', 'macos-arm64'), ('desktop-win-x64', 'win-x64'), ('desktop-linux-x64', 'linux-x64')]:
        receipt = read_json(source / folder / 'manifest.json')
        if (receipt.get('version') != version or receipt.get('commit') != commit
                or receipt.get('inputHash') != input_hash or receipt.get('dirty')
                or receipt.get('verificationFailed')):
            raise ValueError(f'Artifact source/version mismatch: {folder}')
        if distribution and not receipt.get('release'):
            raise ValueError(f'Non-release artifact: {folder}')
        if runtime == 'macos-arm64':
            if receipt.get('target') != 'aarch64-apple-darwin' or receipt.get('appStore'):
                raise ValueError('Expected direct-download macOS arm64 package')
            name = add(folder, receipt['archive'], receipt['archiveSha256'], receipt['archiveBytes'])
            if distribution:
                if not (receipt.get('signed') and receipt.get('stapled') and receipt.get('updates')
                        and (receipt.get('notarization') or {}).get('status') == 'Accepted'
                        and receipt.get('updateFeed') == latest + 'appcast-macos-arm64.xml'):
                    raise ValueError('macOS requires signing, notarization and the release update feed')
                feed = read_json(source / 'macos-feed/manifest.json')
                for key in ['version', 'commit', 'archive', 'archiveSha256', 'updateFeed']:
                    if feed.get(key) != receipt.get(key):
                        raise ValueError(f'Sparkle receipt mismatch: {key}')
                add('macos-feed', feed['appcast'], feed['appcastSha256'], destination='appcast-macos-arm64.xml')
                with zipfile.ZipFile(files[name]['source']) as archive:
                    info = plistlib.loads(archive.read('Monoprint.app/Contents/Info.plist'))
                if (info.get('SUPublicEDKey') != feed.get('publicKey')
                        or info.get('SUFeedURL') != receipt['updateFeed']
                        or info.get('CFBundleShortVersionString') != version):
                    raise ValueError('Embedded Sparkle settings differ from the signed feed')
                items = ET.parse(files['appcast-macos-arm64.xml']['source']).findall('./channel/item')
                if len(items) != 1:
                    raise ValueError('Expected exactly one full Sparkle update')
                enclosure = items[0].find('enclosure')
                if (enclosure is None or enclosure.get('url') != tagged + name
                        or int(enclosure.get('length', '0')) != files[name]['bytes']
                        or not enclosure.get(SPARKLE + 'edSignature')):
                    raise ValueError('Sparkle enclosure does not match the shipping archive')
        else:
            if (receipt.get('runtime') != runtime or receipt.get('channel') != runtime
                    or not receipt.get('verified') or not receipt.get('updates')
                    or receipt.get('updateFeed') != latest):
                raise ValueError(f'Invalid desktop update configuration: {runtime}')
            if distribution and runtime == 'win-x64' and not receipt.get('signed'):
                raise ValueError('Windows requires Authenticode signing')
            artifacts = receipt['artifacts']
            for entry in artifacts:
                add(folder, entry['file'], entry['sha256'], entry['bytes'])
            feed_name = f'releases.{runtime}.json'
            if feed_name not in files:
                raise ValueError(f'Missing update feed: {runtime}')
            feed = read_json(files[feed_name]['source'])
            full = False
            for asset in feed.get('Assets', []):
                entry = files.get(asset['FileName'])
                if (not entry or asset['Version'] != version
                        or asset['SHA256'].lower() != entry['sha256'] or int(asset['Size']) != entry['bytes']):
                    raise ValueError(f'Velopack feed does not match shipping bytes: {runtime}')
                full |= asset.get('Type') == 'Full'
            if not full:
                raise ValueError(f'Missing full update package: {runtime}')
            suffix = 'Setup.exe' if runtime == 'win-x64' else '.AppImage'
            installers = [entry['file'] for entry in artifacts if entry['file'].endswith(suffix)]
            if len(installers) != 1:
                raise ValueError(f'Expected one installer: {runtime}')
            name = installers[0]
        downloads[runtime] = {'file': name, 'url': tagged + name,
                              'sha256': files[name]['sha256'], 'bytes': files[name]['bytes']}

    output.mkdir(parents=True, exist_ok=False)
    for name, entry in files.items():
        shutil.copyfile(entry['source'], output / name)
    manifest = {'version': version, 'tag': f'v{version}', 'commit': commit, 'repository': repository,
                'distribution': distribution, 'downloads': downloads,
                'files': [{'file': name, 'sha256': entry['sha256'], 'bytes': entry['bytes']} for name, entry in files.items()]}
    (output / 'release.json').write_text(json.dumps(manifest, indent=2) + '\n')
    checksums = {name: entry['sha256'] for name, entry in files.items()}
    checksums['release.json'] = hashlib.sha256((output / 'release.json').read_bytes()).hexdigest()
    (output / 'SHA256SUMS').write_text(''.join(f'{digest}  {name}\n' for name, digest in sorted(checksums.items())))
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--from', dest='source', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--distribution', action='store_true')
    args = parser.parse_args()
    manifest = assemble(args.source, args.out,
        version=tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version'],
        commit=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        input_hash=source_digest(ROOT), repository=read_json(ROOT / 'release/config.json')['repository'],
        distribution=args.distribution)
    print(f"Verified {len(manifest['files'])} desktop assets for {manifest['tag']}")


if __name__ == '__main__':
    main()
