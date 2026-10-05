"""Exercise Store profile validation and packaging without Apple credentials."""
from copy import deepcopy
from datetime import datetime, timedelta, timezone
import hashlib
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import app_store_release as store
import macos_release as release


class StoreReleaseTests(unittest.TestCase):
    def setUp(self):
        self.now = datetime(2026, 10, 5, tzinfo=timezone.utc)
        self.identity = hashlib.sha1(b'certificate').hexdigest().upper()
        self.profile = {
            'ExpirationDate': self.now + timedelta(days=90),
            'Platform': ['OSX'], 'TeamIdentifier': ['TEAM'],
            'ApplicationIdentifierPrefix': ['PREFIX'],
            'DeveloperCertificates': [b'certificate'],
            'Entitlements': {
                'com.apple.application-identifier': 'PREFIX.app.monoprint.studio',
                'com.apple.developer.team-identifier': 'TEAM',
            },
        }

    def test_valid_store_profile_and_legacy_prefix(self):
        self.assertEqual(store.validate_profile(self.profile, 'app.monoprint.studio', self.identity, self.now),
                         self.profile['Entitlements'])

    def test_rejects_wrong_app_team_certificate_expired_and_development_profiles(self):
        alterations = [
            {'ExpirationDate': self.now}, {'Platform': ['iOS']},
            {'ProvisionedDevices': ['device']}, {'ProvisionsAllDevices': True},
            {'DeveloperCertificates': [b'other certificate']},
            {'Entitlements': self.profile['Entitlements'] | {'get-task-allow': True}},
            {'Entitlements': self.profile['Entitlements'] | {'com.apple.application-identifier': 'PREFIX.*'}},
            {'Entitlements': self.profile['Entitlements'] | {'com.apple.developer.team-identifier': 'OTHER'}},
        ]
        for alteration in alterations:
            with self.subTest(alteration=alteration), self.assertRaises(ValueError):
                store.validate_profile(deepcopy(self.profile) | alteration, 'app.monoprint.studio', self.identity, self.now)

    def test_rejects_development_identity_and_uses_installer_policy(self):
        with patch.object(store.subprocess, 'check_output', return_value=f'1) {self.identity} "Apple Development: Test (TEAM)"'):
            with self.assertRaisesRegex(ValueError, 'Mac App Store application'):
                store.find_identity(self.identity, 'application')
        with patch.object(store.subprocess, 'check_output', return_value=f'1) {self.identity} "3rd Party Mac Developer Installer: Test (TEAM)"') as output:
            self.assertEqual(store.find_identity(self.identity, 'installer')[0], self.identity)
            self.assertEqual(output.call_args.args[0], ['security', 'find-identity', '-v', '-p', 'basic'])

    def test_entitlements_sign_helpers_with_inheritance_and_app_with_file_access(self):
        with tempfile.TemporaryDirectory() as folder:
            bundle = Path(folder) / 'Monoprint.app'
            macos = bundle / 'Contents/MacOS'
            macos.mkdir(parents=True)
            (bundle / 'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleExecutable': 'gui'}))
            (macos / 'gui').write_bytes(b'\xcf\xfa\xed\xfe')
            (macos / 'cli').write_bytes(b'\xcf\xfa\xed\xfe')
            with patch.object(release.subprocess, 'run') as run:
                release.sign_app(bundle, '-', entitlements='app.entitlements', helper_entitlements='helper.entitlements')
            calls = [c.args[0] for c in run.call_args_list]
            self.assertIn('helper.entitlements', calls[0])
            self.assertIn('app.entitlements', calls[1])
            self.assertIn('--generate-entitlement-der', calls[0])

    def test_package_signs_installer_and_checks_signature(self):
        with tempfile.TemporaryDirectory() as folder:
            destination = Path(folder) / 'Monoprint.pkg'
            with patch.object(store.subprocess, 'run') as run:
                store.package_app('/tmp/Monoprint.app', destination, 'installer', '/tmp/signing.keychain')
            calls = [c.args[0] for c in run.call_args_list]
            self.assertEqual(calls[0], ['productbuild', '--component', '/tmp/Monoprint.app', '/Applications',
                                       '--sign', 'installer', '--keychain', '/tmp/signing.keychain', str(destination)])
            self.assertEqual(calls[1], ['pkgutil', '--check-signature', str(destination)])
            destination.write_bytes(b'keep')
            with patch.object(store.subprocess, 'run') as run, self.assertRaises(FileExistsError):
                store.package_app('/tmp/Monoprint.app', destination)
            run.assert_not_called()
            self.assertEqual(destination.read_bytes(), b'keep')

    @unittest.skipUnless(sys.platform == 'darwin', 'macOS extended attributes')
    def test_store_bundle_removes_copied_quarantine_without_changing_source(self):
        with tempfile.TemporaryDirectory() as folder:
            source = Path(folder) / 'downloaded.provisionprofile'
            source.write_bytes(b'profile contents')
            subprocess.run(['xattr', '-w', 'com.apple.quarantine', '0081;download', str(source)], check=True)
            bundle = Path(folder) / 'Monoprint.app'
            contents = bundle / 'Contents'
            contents.mkdir(parents=True)
            embedded = contents / 'embedded.provisionprofile'
            shutil.copy2(source, embedded)
            for path in [bundle, contents]:
                subprocess.run(['xattr', '-w', 'com.apple.quarantine', '0081;download', str(path)], check=True)
            subprocess.run(['xattr', '-w', 'com.monoprint.test', 'preserve', str(embedded)], check=True)
            attributes = lambda path: subprocess.check_output(['xattr', str(path)], text=True)
            self.assertIn('com.apple.quarantine', attributes(embedded))
            store.clear_bundle_quarantine(bundle)
            store.clear_bundle_quarantine(bundle)  # Also handles clean payloads.
            for path in [bundle, *bundle.rglob('*')]:
                self.assertNotIn('com.apple.quarantine', attributes(path))
            self.assertEqual(embedded.read_bytes(), source.read_bytes())
            self.assertEqual(subprocess.check_output(['xattr', '-p', 'com.monoprint.test', str(embedded)], text=True).strip(), 'preserve')
            self.assertIn('com.apple.quarantine', attributes(source))


if __name__ == '__main__':
    unittest.main()
