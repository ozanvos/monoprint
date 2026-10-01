"""Exercise signing order and post-stapling packaging without Apple credentials."""
from pathlib import Path
import tempfile
import plistlib
import unittest
from unittest.mock import patch
import macos_release as release


class MacOSReleaseTests(unittest.TestCase):
    def test_inside_out_signing(self):
        with tempfile.TemporaryDirectory() as folder:
            bundle = Path(folder) / 'Monoprint.app'
            binary = bundle / 'Contents/MacOS/monoprint-gui'
            binary.parent.mkdir(parents=True)
            (bundle / 'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleExecutable': 'monoprint-gui'}))
            binary.write_bytes(b'\xcf\xfa\xed\xfe' + b'native')
            helper = binary.parent / 'helper'
            helper.write_text('#!/bin/sh\n')
            cli = binary.parent / 'monoprint'
            cli.write_bytes(b'\xcf\xfa\xed\xfe' + b'native')
            with patch.object(release.subprocess, 'run') as run:
                release.sign_app(bundle, 'Developer ID Application: Test', '/tmp/test.keychain')
            calls = [c.args[0] for c in run.call_args_list]
            self.assertEqual([c[-1] for c in calls], [str(helper), str(cli), str(bundle), str(bundle)])
            self.assertTrue(all('--deep' not in c for c in calls[:-1]))
            self.assertIn('--deep', calls[-1])
            self.assertIn('--timestamp', calls[0])
            self.assertIn('--keychain', calls[0])

    def test_final_zip_after_stapling(self):
        with tempfile.TemporaryDirectory() as folder:
            bundle = Path(folder) / 'Monoprint.app'
            final = Path(folder) / 'final.zip'
            with patch.object(release.subprocess, 'run') as run, patch.object(release.subprocess, 'check_output', return_value='{"status":"Accepted","id":"submission"}'):
                release.notarize_app(bundle, 'profile')
                release.zip_app(bundle, final)
            calls = [c.args[0] for c in run.call_args_list]
            staple = next(i for i, c in enumerate(calls) if c[:3] == ['xcrun', 'stapler', 'staple'])
            final_zip = next(i for i, c in enumerate(calls) if c[-1] == str(final))
            self.assertLess(staple, final_zip)
            self.assertIn('--keepParent', calls[final_zip])

    def test_rejected_notarization_never_staples(self):
        with patch.object(release.subprocess, 'check_output', return_value='{"status":"Invalid","id":"bad"}'), patch.object(release.subprocess, 'run') as run:
            with self.assertRaisesRegex(RuntimeError, 'Invalid'):
                release.notarize_app('/tmp/app', 'profile')
            self.assertFalse(any(c.args[0][:2] == ['xcrun', 'stapler'] for c in run.call_args_list))

    def test_existing_archive_preserved(self):
        with tempfile.TemporaryDirectory() as folder:
            archive = Path(folder) / 'keep.zip'
            archive.write_bytes(b'keep')
            with self.assertRaises(FileExistsError):
                release.zip_app('/tmp/app', archive)
            self.assertEqual(archive.read_bytes(), b'keep')


if __name__ == '__main__':
    unittest.main()
