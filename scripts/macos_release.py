"""macOS signing and notarization shared by CLI and app packaging."""
import argparse
import hashlib
import json
import plistlib
from pathlib import Path
import subprocess
import tempfile


def source_digest(root):
    """Bind app receipts to source, packaging recipes, assets and notices."""
    root = Path(root)
    inputs = ['Cargo.toml', 'Cargo.lock', 'bindings/node/Cargo.toml',
              'apps/monoprint-gui/Cargo.toml', 'apps/monoprint-gui/Cargo.lock',
              'LICENSE', 'THIRD_PARTY_NOTICES.md']
    inputs += ['vendor/takumi-raster/Cargo.toml', 'vendor/takumi-svg/Cargo.toml']
    inputs += ['apps/monoprint-gui/assets/app-store.entitlements',
               'apps/monoprint-gui/assets/app-store-helper.entitlements']
    directories = ['src', 'assets', 'licenses', 'vendor/takumi-raster/src', 'vendor/takumi-svg/src',
                   'vendor/gpui-component', 'vendor/gpui-pre-macos',
                   'apps/monoprint-gui/src', 'apps/monoprint-gui/scripts',
                   'apps/monoprint-gui/assets/Monoprint.icon', 'apps/monoprint-gui/assets/shortcuts',
                   'apps/monoprint-gui/assets/themes', 'scripts']
    for directory in directories:
        inputs += [str(p.relative_to(root)) for p in (root / directory).rglob('*')
                   if p.is_file() and '__pycache__' not in p.parts and 'target' not in p.parts]
    digest = hashlib.sha256()
    for name in sorted(set(inputs)):
        digest.update(name.encode())
        digest.update(b'\0')
        digest.update((root / name).read_bytes())
        digest.update(b'\0')
    return digest.hexdigest()


def verify_app_store_apis(binary):
    """Fail packaging if any audited private macOS API names survived compilation."""
    forbidden = (
        b'_opaqueRectForWindowMoveWhenInTitlebar', b'_zoomFill:',
        b'_windowRestorationOptions', b'NSWindowRestorationOptions',
        b'_windowResizeNorthWestSouthEastCursor', b'_windowResizeNorthEastSouthWestCursor',
        b'CAChameleonLayer', b'NSWindowRestoresWorkspaceAtLaunch',
    )
    data = Path(binary).read_bytes()
    found = [name.decode() for name in forbidden if name in data]
    if found:
        raise RuntimeError(f'App Store build contains private macOS API names: {", ".join(found)}')


def sign_code(path, identity='-', keychain=None, entitlements=None):
    command = ['codesign', '--force', '--sign', identity]
    if keychain:
        command += ['--keychain', str(keychain)]
    if identity != '-':
        command += ['--options', 'runtime', '--timestamp']
    if entitlements:
        command += ['--entitlements', str(entitlements), '--generate-entitlement-der']
    # Sign each Mach-O explicitly. --deep is only appropriate for verification.
    subprocess.run([*command, str(path)], check=True)


def verify_code(path):
    subprocess.run(['codesign', '--verify', '--deep', '--strict', str(path)], check=True)


def notarize(archive, profile, keychain=None):
    command = ['xcrun', 'notarytool', 'submit', str(archive),
               '--keychain-profile', profile, '--wait', '--timeout', '30m',
               '--output-format', 'json']
    if keychain:
        command += ['--keychain', str(keychain)]
    result = json.loads(subprocess.check_output(command, text=True))
    if result.get('status') != 'Accepted':
        raise RuntimeError(f"Notarization failed: {result.get('status')}; submission {result.get('id')}")
    return {key: result[key] for key in ['id', 'status']}


def zip_app(bundle, archive):
    archive = Path(archive)
    if archive.exists():
        raise FileExistsError(f'Refusing to overwrite archive: {archive}')
    subprocess.run(['ditto', '-c', '-k', '--sequesterRsrc', '--keepParent',
                    str(bundle), str(archive)], check=True)


def sign_app(bundle, identity='-', keychain=None, entitlements=None, helper_entitlements=None):
    bundle = Path(bundle)
    info = plistlib.loads((bundle / 'Contents/Info.plist').read_bytes())
    main = bundle / 'Contents/MacOS' / info['CFBundleExecutable']
    # Sign helpers first. Signing the enclosing app signs its main executable;
    # codesign can interpret the main executable's path as the enclosing bundle.
    # The shell helper in MacOS also requires signing (stored in extended attrs).
    magic = {b'\xfe\xed\xfa\xce', b'\xce\xfa\xed\xfe', b'\xfe\xed\xfa\xcf',
             b'\xcf\xfa\xed\xfe', b'\xca\xfe\xba\xbe', b'\xbe\xba\xfe\xca',
             b'\xca\xfe\xba\xbf', b'\xbf\xba\xfe\xca'}
    for path in sorted(bundle.rglob('*'), key=lambda p: (-len(p.parts), str(p))):
        if not path.is_file() or path.is_symlink() or path == main:
            continue
        with path.open('rb') as stream:
            if stream.read(4) not in magic and path.parent != bundle / 'Contents/MacOS':
                continue
        sign_code(path, identity, keychain, helper_entitlements)
    sign_code(bundle, identity, keychain, entitlements)
    verify_code(bundle)


def notarize_app(bundle, profile, keychain=None):
    with tempfile.TemporaryDirectory(prefix='monoprint-notary-') as temporary:
        archive = Path(temporary) / 'submission.zip'
        zip_app(bundle, archive)
        result = notarize(archive, profile, keychain)
    subprocess.run(['xcrun', 'stapler', 'staple', str(bundle)], check=True)
    subprocess.run(['xcrun', 'stapler', 'validate', str(bundle)], check=True)
    verify_code(bundle)
    subprocess.run(['spctl', '--assess', '--type', 'execute', '--verbose=2', str(bundle)], check=True)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--sign-identity', required=True)
    parser.add_argument('--sign-keychain', type=Path)
    parser.add_argument('--notary-profile')
    parser.add_argument('--notary-keychain', type=Path)
    parser.add_argument('--receipt', required=True, type=Path)
    args = parser.parse_args()
    if args.sign_identity == '-':
        parser.error('Distribution CLI signing requires a Developer ID Application identity')
    sign_code(args.binary, args.sign_identity, args.sign_keychain)
    verify_code(args.binary)
    result = None
    if args.notary_profile:
        with tempfile.TemporaryDirectory(prefix='monoprint-cli-notary-') as temporary:
            archive = Path(temporary) / 'submission.zip'
            subprocess.run(['ditto', '-c', '-k', '--keepParent', str(args.binary), str(archive)], check=True)
            result = notarize(archive, args.notary_profile, args.notary_keychain)
    # A standalone executable cannot carry a stapled ticket. Gatekeeper fetches it
    # online; the app distribution can additionally be stapled for offline use.
    args.receipt.write_text(json.dumps({'signed': True, 'notarization': result,
        'stapled': False, 'sha256': hashlib.sha256(args.binary.read_bytes()).hexdigest()}, indent=2) + '\n')


if __name__ == '__main__':
    main()
