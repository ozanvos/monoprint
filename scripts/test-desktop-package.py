"""Verify packaged desktop bytes, Velopack feed integrity and runtime contents."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import struct
import sys
import tempfile
import zipfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--from', dest='source', required=True, type=Path)
args = parser.parse_args()
receipt = json.loads((args.source / 'manifest.json').read_text())
assert receipt['updates'] and receipt['updateFeed'].startswith('https://')
artifacts = {entry['file']: entry for entry in receipt['artifacts']}
for name, entry in artifacts.items():
    assert Path(name).name == name and '\\' not in name
    file = args.source / name
    assert file.stat().st_size == entry['bytes']
    assert hashlib.sha256(file.read_bytes()).hexdigest() == entry['sha256'], name
feed = json.loads((args.source / f"releases.{receipt['channel']}.json").read_text())
assert feed['Assets'], 'Empty update feed'
for asset in feed['Assets']:
    assert asset['FileName'] in artifacts
    assert asset['SHA256'].lower() == artifacts[asset['FileName']]['sha256']
    assert int(asset['Size']) == artifacts[asset['FileName']]['bytes']
    assert asset['Version'] == receipt['version']
packages = list(args.source.glob('*-full.nupkg'))
assert packages, 'Missing full update package'
for package in packages:
    with zipfile.ZipFile(package) as archive:
        names = archive.namelist()
        assert all(not Path(name).is_absolute() and '..' not in Path(name).parts for name in names)
        if receipt['runtime'].startswith('win-'):
            for suffix in ['monoprint-gui.exe', 'resources/LICENSE', 'resources/licenses/gui-dependencies.json']:
                assert any(name.endswith('/' + suffix) for name in names), suffix
            main_name = next(name for name in names if name.endswith('/monoprint-gui.exe'))
            executable = archive.read(main_name)
            pe = struct.unpack_from('<I', executable, 0x3c)[0]
            assert executable[pe:pe + 4] == b'PE\0\0'
            assert struct.unpack_from('<H', executable, pe + 24 + 68)[0] == 2, 'GUI releases must not open a console window'
            if receipt['signed'] and sys.platform == 'win32':
                with tempfile.TemporaryDirectory() as temporary:
                    for index, name in enumerate(name for name in names if name.lower().endswith('.exe')):
                        binary = Path(temporary) / f'{index}.exe'
                        binary.write_bytes(archive.read(name))
                        subprocess.run(['powershell', '-NoProfile', '-Command',
                            'if ((Get-AuthenticodeSignature -FilePath $env:MONOPRINT_VERIFY_EXE).Status -ne "Valid") { exit 1 }'],
                            env=os.environ | {'MONOPRINT_VERIFY_EXE': str(binary)}, check=True)
        else:
            assert any(name.endswith('.AppImage') for name in names)
if receipt['runtime'].startswith('linux-'):
    images = list(args.source.glob('*.AppImage'))
    assert len(images) == 1
    machine = {'linux-x64': 62, 'linux-arm64': 183}[receipt['runtime']]

    def verify_elf(path):
        with path.open('rb') as binary:
            header = binary.read(20)
        assert header[:6] == b'\x7fELF\x02\x01', f'Expected little-endian ELF64: {path}'
        assert struct.unpack_from('<H', header, 18)[0] == machine, f'Wrong architecture: {path}'

    verify_elf(images[0])
    if sys.platform == 'linux':
        with tempfile.TemporaryDirectory(prefix='monoprint-appimage-check-') as temporary:
            image = images[0].resolve()
            image.chmod(0o755)
            subprocess.run([str(image), '--appimage-extract'], cwd=temporary, check=True, stdout=subprocess.DEVNULL)
            folder = Path(temporary) / 'squashfs-root'
            for name in ['AppRun', 'usr/bin/monoprint-gui', 'usr/bin/UpdateNix', 'usr/bin/sq.version',
                         'usr/bin/resources/LICENSE', 'usr/bin/resources/licenses/gui-dependencies.json']:
                assert (folder / name).is_file(), name
            for name in ['monoprint-gui', 'UpdateNix']:
                verify_elf(folder / 'usr/bin' / name)
            libraries = subprocess.check_output(['ldd', str(folder / 'usr/bin/monoprint-gui')],
                env=os.environ | {'LD_LIBRARY_PATH': str(folder / 'usr/lib')}, text=True)
            assert 'not found' not in libraries, libraries
if receipt['runtime'].startswith('win-'):
    installers = list(args.source.glob('*Setup.exe'))
    assert installers, 'Missing Windows installer'
    if receipt['signed'] and sys.platform == 'win32':
        for installer in installers:
            subprocess.run(['powershell', '-NoProfile', '-Command',
                'if ((Get-AuthenticodeSignature -FilePath $env:MONOPRINT_VERIFY_EXE).Status -ne "Valid") { exit 1 }'],
                env=os.environ | {'MONOPRINT_VERIFY_EXE': str(installer.resolve())}, check=True)
print('Desktop artifacts, feed hashes, full update packages and runtime contents passed.')
