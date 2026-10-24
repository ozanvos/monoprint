"""Updater release-policy and signing-order regressions, without credentials."""
import base64
import json
from pathlib import Path
import plistlib
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from desktop_updates import https_url, sparkle_settings, velopack_settings, linuxdeploy_settings
import macos_release as release


class UpdateReleaseTests(unittest.TestCase):
    def test_linuxdeploy_is_pinned_per_architecture(self):
        x64_url, x64_hash = linuxdeploy_settings('linux-x64')
        arm_url, arm_hash = linuxdeploy_settings('linux-arm64')
        self.assertTrue(x64_url.endswith('/linuxdeploy-x86_64.AppImage'))
        self.assertTrue(arm_url.endswith('/linuxdeploy-aarch64.AppImage'))
        self.assertNotEqual(x64_hash, arm_hash)
        for digest in [x64_hash, arm_hash]:
            self.assertRegex(digest, r'^[a-f0-9]{64}$')
        with self.assertRaises(KeyError):
            linuxdeploy_settings('linux-armhf')

    def test_feed_generator_refuses_unsigned_local_artifacts_before_creating_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            folder = Path(temporary)
            (folder / 'manifest.json').write_text(json.dumps({
                'release': False, 'dirty': True, 'updates': True,
                'signed': False, 'stapled': False, 'notarization': None,
            }))
            script = Path(__file__).resolve().parents[1] / 'apps/monoprint-gui/scripts/generate-appcast.py'
            result = subprocess.run([sys.executable, str(script), '--from', str(folder),
                '--sparkle-sdk', str(folder), '--output', str(folder / 'feed'),
                '--download-url', 'https://updates.example/mac/'], capture_output=True, text=True)
            self.assertEqual(result.returncode, 2)
            self.assertIn('signed and notarized release', result.stderr)
            self.assertFalse((folder / 'feed').exists())

    @unittest.skipUnless(sys.platform == 'darwin', 'CryptoKit verification uses the macOS release host')
    def test_signature_verification_uses_the_archive_and_embedded_public_key(self):
        # RFC 8032 section 7.1: Ed25519 TEST 1, an empty message.
        key = bytes.fromhex('d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a')
        signature = bytes.fromhex('e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b')
        with tempfile.TemporaryDirectory() as temporary:
            archive = Path(temporary) / 'archive.zip'
            archive.write_bytes(b'')
            command = ['xcrun', 'swift', '-module-cache-path', str(Path(temporary) / 'module-cache'),
                       str(Path(__file__).with_name('verify-sparkle-signature.swift')),
                       str(archive), base64.b64encode(signature).decode(), base64.b64encode(key).decode()]
            subprocess.run(command, check=True, capture_output=True)
            archive.write_bytes(b'tampered')
            result = subprocess.run(command, capture_output=True)
            self.assertNotEqual(result.returncode, 0)

    def test_feed_and_public_key_are_required(self):
        key = base64.b64encode(bytes(range(32))).decode()
        settings = sparkle_settings('https://updates.example/mac/appcast.xml', key)
        self.assertEqual(settings['SUPublicEDKey'], key)
        self.assertFalse(settings['SUAutomaticallyUpdate'])
        for invalid in ['http://updates.example/', 'https://user:password@updates.example/', 'https://updates.example/#fragment']:
            with self.assertRaises(ValueError):
                sparkle_settings(invalid, key)
        with self.assertRaises(ValueError):
            sparkle_settings('https://updates.example/', 'aW52YWxpZA==')
        with patch('desktop_updates.configuration', return_value={
            'sparkle': {'feedUrl': None, 'publicKey': None},
            'velopack': {'version': '1.2.161', 'feeds': {'linux-x64': None}},
        }):
            with self.assertRaises(ValueError):
                sparkle_settings(None, key)
            with self.assertRaises(ValueError):
                velopack_settings('linux-x64')

    def test_sparkle_is_rejected_by_store_binary_audit(self):
        with tempfile.TemporaryDirectory() as temporary:
            binary = Path(temporary) / 'app'
            binary.write_bytes(b'SPUStandardUpdaterController')
            with self.assertRaisesRegex(RuntimeError, 'SPUStandardUpdaterController'):
                release.verify_app_store_apis(binary)

    def test_framework_helpers_are_signed_before_their_enclosing_bundles(self):
        with tempfile.TemporaryDirectory() as temporary:
            app = Path(temporary) / 'Monoprint.app'
            contents = app / 'Contents'
            (contents / 'MacOS').mkdir(parents=True)
            (contents / 'MacOS/monoprint-gui').write_bytes(b'\xcf\xfa\xed\xfeapp')
            (contents / 'Info.plist').write_bytes(plistlib.dumps({'CFBundleExecutable': 'monoprint-gui'}))
            framework = contents / 'Frameworks/Sparkle.framework'
            helper = framework / 'Versions/B/XPCServices/Installer.xpc'
            binary = helper / 'Contents/MacOS/Installer'
            binary.parent.mkdir(parents=True)
            binary.write_bytes(b'\xcf\xfa\xed\xfeinstaller')
            (framework / 'Versions/Current').symlink_to('B')
            with patch.object(release, 'sign_code') as sign, patch.object(release, 'verify_code'):
                release.sign_app(app, 'Developer ID: Test')
            paths = [call.args[0] for call in sign.call_args_list]
            self.assertLess(paths.index(binary), paths.index(helper))
            self.assertLess(paths.index(helper), paths.index(framework))
            self.assertLess(paths.index(framework), paths.index(app))
            self.assertNotIn(framework / 'Versions/Current', paths)


if __name__ == '__main__':
    unittest.main()
