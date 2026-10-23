"""Run one packaging command with temporary CI signing credentials. Never publishes."""
import base64
import os
from pathlib import Path
import secrets
import shlex
import shutil
import subprocess
import sys
import tempfile


def quiet(command):
    result = subprocess.run(command, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    if result.returncode:
        # Do not leak passwords/private key contents through CalledProcessError.
        raise RuntimeError(f'{command[0]} credential setup failed (exit {result.returncode})')


def main():
    check_only = sys.argv[1:] == ['--check-only']
    if sys.platform != 'darwin' or (not check_only and (len(sys.argv) < 3 or sys.argv[1] != '--')):
        raise SystemExit('Usage on macOS: python3 scripts/with-apple-signing.py --check-only | -- PACKAGING_COMMAND [ARGS]')
    names = ['MACOS_CERTIFICATE_BASE64', 'MACOS_CERTIFICATE_PASSWORD', 'MACOS_SIGN_IDENTITY',
             'APPLE_API_KEY_BASE64', 'APPLE_API_KEY_ID', 'APPLE_API_ISSUER_ID']
    values = {name: os.environ.get(name, '') for name in names}
    missing = [name for name, value in values.items() if not value]
    if missing:
        raise SystemExit('Missing signing credentials: ' + ', '.join(missing))
    if not values['MACOS_SIGN_IDENTITY'].startswith('Developer ID Application:'):
        raise SystemExit('MACOS_SIGN_IDENTITY must name a Developer ID Application certificate')
    with tempfile.TemporaryDirectory(prefix='monoprint-signing-') as temporary:
        folder = Path(temporary)
        keychain = folder / 'signing.keychain-db'
        certificate = folder / 'certificate.p12'
        api_key = folder / 'AuthKey.p8'
        for file, name in [(certificate, 'MACOS_CERTIFICATE_BASE64'), (api_key, 'APPLE_API_KEY_BASE64')]:
            file.write_bytes(base64.b64decode(values[name], validate=True))
            file.chmod(0o600)
        password = secrets.token_urlsafe(32)
        original_keychains = shlex.split(subprocess.check_output(
            ['security', 'list-keychains', '-d', 'user'], text=True))
        created = False
        try:
            quiet(['security', 'create-keychain', '-p', password, str(keychain)])
            created = True
            quiet(['security', 'set-keychain-settings', '-lut', '21600', str(keychain)])
            quiet(['security', 'unlock-keychain', '-p', password, str(keychain)])
            quiet(['security', 'list-keychains', '-d', 'user', '-s', str(keychain), *original_keychains])
            quiet(['security', 'import', str(certificate), '-k', str(keychain),
                   '-P', values['MACOS_CERTIFICATE_PASSWORD'], '-T', '/usr/bin/codesign'])
            quiet(['security', 'set-key-partition-list', '-S', 'apple-tool:,apple:,codesign:',
                   '-s', '-k', password, str(keychain)])
            identities = subprocess.check_output(
                ['security', 'find-identity', '-v', '-p', 'codesigning', str(keychain)], text=True)
            if values['MACOS_SIGN_IDENTITY'] not in identities:
                raise RuntimeError('Imported Developer ID identity is not valid; check the PKCS#12 private key and certificate chain')
            probe = folder / 'signing-probe'
            shutil.copyfile('/usr/bin/true', probe)
            quiet(['codesign', '--force', '--sign', values['MACOS_SIGN_IDENTITY'],
                   '--keychain', str(keychain), '--options', 'runtime', '--timestamp', str(probe)])
            quiet(['codesign', '--verify', '--strict', str(probe)])
            quiet(['xcrun', 'notarytool', 'store-credentials', 'monoprint-ci',
                   '--keychain', str(keychain), '--key', str(api_key),
                   '--key-id', values['APPLE_API_KEY_ID'], '--issuer', values['APPLE_API_ISSUER_ID']])
            if check_only:
                print('Developer ID signing and notarization authentication verified')
                return
            environment = {key: value for key, value in os.environ.items() if key not in names}
            subprocess.run([*sys.argv[2:], '--sign-identity', values['MACOS_SIGN_IDENTITY'],
                            '--sign-keychain', str(keychain), '--notary-profile', 'monoprint-ci',
                            '--notary-keychain', str(keychain)], env=environment, check=True)
        finally:
            if created:
                try:
                    quiet(['security', 'list-keychains', '-d', 'user', '-s', *original_keychains])
                finally:
                    quiet(['security', 'delete-keychain', str(keychain)])


if __name__ == '__main__':
    main()
