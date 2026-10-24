"""Qualify a real Linux package: update replacement, native window and restart.

Run under xvfb-run on the packaging runner. All install/cache/session paths stay
in a temporary directory; this never changes the runner's home or desktop.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--from', dest='source', required=True, type=Path)
args = parser.parse_args()
source = args.source.resolve()
root = Path(__file__).resolve().parents[1]
receipt = json.loads((source / 'manifest.json').read_text())
assert receipt['runtime'] in ('linux-x64', 'linux-arm64')
original, = source.glob('*.AppImage')
with tempfile.TemporaryDirectory(prefix='monoprint-linux-qualification-') as temporary:
    fixture = Path(temporary)
    image = fixture / 'Monoprint.AppImage'
    shutil.copy2(original, image)
    image.chmod(0o755)
    subprocess.run([str(image), '--appimage-extract'], cwd=fixture, check=True, stdout=subprocess.DEVNULL)
    binary_dir = fixture / 'squashfs-root/usr/bin'
    manifest = (binary_dir / 'sq.version').read_text()
    old, count = re.subn(r'<version>[^<]+</version>', '<version>0.0.1</version>', manifest)
    assert count == 1 and old != manifest, 'Need a distinct older version for update qualification'
    (fixture / 'old.version').write_text(old)
    # Appending a harmless marker keeps the test host executable while proving
    # the updater actually replaces it with the exact packaged bytes.
    with image.open('ab') as stream:
        stream.write(b'\nmonoprint-isolated-older-host\n')
    subprocess.run([
        'cargo', 'test', '--locked', '--manifest-path', str(root / 'apps/monoprint-gui/Cargo.toml'),
        '--target-dir', str(root / 'target/gui'), '--test', 'linux_update', '--', '--ignored', '--nocapture',
    ], check=True, env=os.environ | {
        'MONOPRINT_UPDATE_FIXTURE': str(fixture), 'MONOPRINT_PACKAGE_SOURCE': str(source),
        'MONOPRINT_TEST_UPDATE_CHANNEL': receipt['runtime'],
    })
    expected = digest(original)
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline:
        if image.exists() and digest(image) == expected:
            break
        time.sleep(0.5)
    assert image.exists() and digest(image) == expected, 'Velopack did not replace the AppImage'
    assert (fixture / 'downloaded-version').read_text() == receipt['version']
    session = fixture / 'session.json'
    environment = os.environ | {
        'APPIMAGE_EXTRACT_AND_RUN': '1', 'MONOPRINT_SESSION': str(session),
        'XDG_CACHE_HOME': str(fixture / 'cache'), 'XDG_CONFIG_HOME': str(fixture / 'config'),
        'XDG_DATA_HOME': str(fixture / 'data'), 'XDG_STATE_HOME': str(fixture / 'state'),
    }
    environment.pop('WAYLAND_DISPLAY', None)
    for attempt in range(2):
        with (fixture / f'launch-{attempt}.log').open('w+') as log:
            process = subprocess.Popen([str(image)], env=environment, stdout=log, stderr=log)
            try:
                deadline = time.monotonic() + 90
                window = None
                while time.monotonic() < deadline and process.poll() is None:
                    found = subprocess.run(['xdotool', 'search', '--onlyvisible', '--name', '^Monoprint$'], capture_output=True, text=True)
                    if found.returncode == 0 and found.stdout.strip():
                        window = found.stdout.splitlines()[0]
                        break
                    time.sleep(0.5)
                if not window:
                    log.seek(0)
                    raise RuntimeError('Native Monoprint window did not open:\n' + log.read())
                # Graceful application Quit flushes the saved draft. Testing the
                # same session again catches startup and restart regressions.
                # Quit can destroy the window before xdotool sends its final
                # key-release event (BadWindow). The actual success criteria
                # are a clean application exit and a persisted draft below.
                quit_input = subprocess.run(
                    ['xdotool', 'key', '--window', window, 'ctrl+q'],
                    capture_output=True, text=True,
                )
                try:
                    exit_code = process.wait(timeout=30)
                except subprocess.TimeoutExpired as error:
                    raise RuntimeError('Application did not quit:\n' + quit_input.stderr) from error
                assert exit_code == 0, f'Application exited with {exit_code}: {quit_input.stderr}'
                assert session.is_file() and json.loads(session.read_text())
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=10)
print('Linux update discovery, download, byte-exact replacement, native launch, saved draft and restart passed.')
