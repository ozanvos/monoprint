#!/usr/bin/env python3
"""Package Windows installers or Linux AppImages and their Velopack feed locally."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib

gui = Path(__file__).resolve().parents[1]
root = gui.parents[1]
sys.path.insert(0, str(root / 'scripts'))
from desktop_updates import velopack_settings, configuration
from gui_notices import package_notices
from macos_release import source_digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime', required=True, choices=['win-x64', 'linux-x64'])
    parser.add_argument('--output', required=True, type=Path, help='Fresh output directory')
    parser.add_argument('--update-feed', help='HTTPS directory containing releases.CHANNEL.json and packages')
    parser.add_argument('--linuxdeploy', type=Path, help='Pinned linuxdeploy AppImage (Linux only)')
    parser.add_argument('--sign-template', help='Windows signing command template accepted by vpk; use {{file}}')
    parser.add_argument('--local', action='store_true', help='Allow dirty source; mark artifacts verification-only')
    args = parser.parse_args()
    is_windows = args.runtime == 'win-x64'
    if (is_windows and sys.platform != 'win32') or (not is_windows and sys.platform != 'linux'):
        parser.error('Package on the matching operating system')
    if platform.machine().lower() not in ('amd64', 'x86_64'):
        parser.error('This package targets x64; use an x64 build host')
    if args.output.exists():
        parser.error('--output must be a fresh directory')
    feed, tool_version = velopack_settings(args.runtime, args.update_feed)
    installed = subprocess.check_output(['vpk', '--version'], text=True).strip()
    if not re.search(rf'(?<![\d.]){re.escape(tool_version)}(?![\d.])', installed):
        parser.error(f'Install vpk {tool_version}; found {installed}')
    if is_windows and not args.local and not args.sign_template:
        parser.error('Windows distribution requires --sign-template; use --local for unsigned verification')
    if not is_windows and (not args.linuxdeploy or not args.linuxdeploy.is_file()):
        parser.error('Linux packaging requires --linuxdeploy from prepare-linuxdeploy.py')
    if not is_windows and hashlib.sha256(args.linuxdeploy.read_bytes()).hexdigest() != configuration()['linuxdeploy']['archiveSha256']:
        parser.error('linuxdeploy checksum differs from release/updates.json')
    commit = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
    dirty = bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=root, text=True).strip())
    if dirty and not args.local:
        parser.error('Distribution requires clean source; use --local for verification')
    input_hash = source_digest(root)
    version = tomllib.loads((gui / 'Cargo.toml').read_text())['package']['version']
    channel = args.runtime
    target = root / 'target' / f'gui-{args.runtime}'
    environment = os.environ | {'MONOPRINT_UPDATE_URL': feed, 'MONOPRINT_UPDATE_CHANNEL': channel}
    subprocess.run(['cargo', 'build', '--locked', '--release', '--manifest-path', str(gui / 'Cargo.toml'), '--target-dir', str(target)], env=environment, check=True)
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--offline', '--manifest-path', str(gui / 'Cargo.toml'), '--format-version', '1'], env=environment))
    with tempfile.TemporaryDirectory(prefix='monoprint-desktop-') as temporary:
        stage = Path(temporary) / ('app' if is_windows else 'Monoprint.AppDir')
        binaries = stage if is_windows else stage / 'usr/bin'
        binaries.mkdir(parents=True)
        exe = 'monoprint-gui.exe' if is_windows else 'monoprint-gui'
        shutil.copy2(target / 'release' / exe, binaries / exe)
        resources = binaries / 'resources'
        resources.mkdir()
        for name in ['LICENSE', 'THIRD_PARTY_NOTICES.md']:
            shutil.copy2(root / name, resources / name)
        shutil.copytree(root / 'licenses', resources / 'licenses')
        fonts = resources / 'licenses/fonts'
        fonts.mkdir(exist_ok=True)
        for path in (root / 'assets/fonts').glob('*-OFL.txt'):
            shutil.copy2(path, fonts / path.name)
        fallback = root / 'assets/fonts/fallback'
        for path in [fallback / 'manifest.json', fallback / 'NOTICE.txt', *fallback.rglob('*-OFL.txt')]:
            destination = fonts / 'fallback' / path.relative_to(fallback)
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, destination)
        package_notices(root, gui, resources, metadata)
        if not is_windows:
            desktop = stage / 'app.monoprint.studio.desktop'
            desktop.write_text('[Desktop Entry]\nType=Application\nName=Monoprint\nExec=monoprint-gui\nIcon=monoprint\nCategories=Development;Graphics;\n')
            icon = stage / 'monoprint.svg'
            shutil.copy2(gui / 'assets/monoprint.svg', icon)
            shutil.copy2(icon, stage / '.DirIcon')
            # linuxdeploy resolves non-system shared libraries into usr/lib.
            subprocess.run([str(args.linuxdeploy.resolve()), '--appimage-extract-and-run', '--appdir', str(stage), '--executable', str(binaries / exe), '--desktop-file', str(desktop), '--icon-file', str(icon)], check=True)
            app_run = stage / 'AppRun'
            if app_run.is_symlink():
                app_run.unlink()
            app_run.write_text('#!/bin/sh\nHERE="$(dirname "$(readlink -f "$0")")"\nexport LD_LIBRARY_PATH="$HERE/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"\nexec "$HERE/usr/bin/monoprint-gui" "$@"\n')
            app_run.chmod(0o755)
            subprocess.run(['desktop-file-validate', str(desktop)], check=True)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.mkdir()
        command = ['vpk', 'pack', '--packId', 'Monoprint', '--packTitle', 'Monoprint', '--packVersion', version, '--packDir', str(stage), '--mainExe', exe, '--runtime', args.runtime, '--channel', channel, '--outputDir', str(args.output.resolve())]
        if args.sign_template:
            command += ['--signTemplate', args.sign_template]
        subprocess.run(command, check=True)
    if source_digest(root) != input_hash or subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip() != commit:
        raise RuntimeError('Source changed while packaging; rebuild')
    artifacts = []
    for path in sorted(args.output.iterdir()):
        if path.is_file():
            artifacts.append({'file': path.name, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'bytes': path.stat().st_size})
    feed_path = args.output / f'releases.{channel}.json'
    if not feed_path.is_file() or not artifacts:
        raise RuntimeError('Velopack did not create the expected release feed')
    final_dirty = bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=root, text=True).strip())
    if final_dirty and not args.local:
        raise RuntimeError('Checkout changed while packaging')
    receipt = {
        'version': version, 'commit': commit, 'runtime': args.runtime, 'channel': channel,
        'release': False, 'dirty': final_dirty, 'inputHash': input_hash,
        'updates': True, 'updateFeed': feed, 'velopackVersion': tool_version,
        'signed': bool(is_windows and args.sign_template), 'artifacts': artifacts,
    }
    manifest = args.output / 'manifest.json'
    manifest.write_text(json.dumps(receipt, indent=2) + '\n')
    (args.output / 'SHA256SUMS').write_text(''.join(f"{entry['sha256']}  {entry['file']}\n" for entry in artifacts))
    # An ineffective signing hook or incomplete AppDir cannot leave a receipt
    # that claims distribution eligibility.
    try:
        subprocess.run([sys.executable, str(root / 'scripts/test-desktop-package.py'), '--from', str(args.output)], check=True)
    except subprocess.CalledProcessError:
        receipt['signed'] = False
        receipt['verificationFailed'] = True
        manifest.write_text(json.dumps(receipt, indent=2) + '\n')
        raise
    receipt['release'] = not args.local
    receipt['verified'] = True
    manifest.write_text(json.dumps(receipt, indent=2) + '\n')
    print(args.output)


if __name__ == '__main__':
    main()
