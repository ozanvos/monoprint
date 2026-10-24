"""Release assembly rejects partial, mixed, unsigned and altered artifacts."""
import copy
import hashlib
import json
from pathlib import Path
import plistlib
import tempfile
import unittest
import zipfile

from desktop_release import assemble, SPARKLE


class DesktopReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.source = Path(self.temp.name) / 'inputs'
        self.output = Path(self.temp.name) / 'release'
        self.version = '0.1.0'
        self.commit = 'a' * 40
        self.base = 'https://github.com/ozanvos/monoprint/releases'
        self.latest = self.base + '/latest/download/'
        self.receipts = {}
        for folder in ['macos-app', 'macos-feed', 'desktop-win-x64', 'desktop-linux-x64', 'desktop-linux-arm64']:
            (self.source / folder).mkdir(parents=True)
        name = 'Monoprint-0.1.0-aarch64-apple-darwin.zip'
        archive = self.source / 'macos-app' / name
        with zipfile.ZipFile(archive, 'w') as output:
            output.writestr('Monoprint.app/Contents/Info.plist', plistlib.dumps({
                'CFBundleShortVersionString': self.version,
                'SUFeedURL': self.latest + 'appcast-macos-arm64.xml', 'SUPublicEDKey': 'key',
            }))
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        common = {'version': self.version, 'commit': self.commit, 'inputHash': 'inputs',
                  'release': True, 'dirty': False, 'updates': True, 'signed': True}
        self.receipts['macos-app'] = common | {
            'target': 'aarch64-apple-darwin', 'stapled': True, 'notarization': {'status': 'Accepted'},
            'archive': name, 'archiveSha256': digest, 'archiveBytes': archive.stat().st_size,
            'updateFeed': self.latest + 'appcast-macos-arm64.xml',
        }
        appcast = (f'<rss xmlns:sparkle="{SPARKLE[1:-1]}"><channel><item><enclosure '
                   f'url="{self.base}/download/v{self.version}/{name}" length="{archive.stat().st_size}" '
                   'sparkle:edSignature="test-signature"/></item></channel></rss>').encode()
        (self.source / 'macos-feed/appcast.xml').write_bytes(appcast)
        self.receipts['macos-feed'] = {key: self.receipts['macos-app'][key] for key in ['version', 'commit', 'archive', 'archiveSha256', 'updateFeed']}
        self.receipts['macos-feed'].update(appcast='appcast.xml', appcastSha256=hashlib.sha256(appcast).hexdigest(), publicKey='key')
        for runtime, suffix in [('win-x64', 'Setup.exe'), ('linux-x64', '.AppImage'), ('linux-arm64', '.AppImage')]:
            folder = f'desktop-{runtime}'
            self.receipts[folder] = common | {'runtime': runtime, 'channel': runtime,
                'verified': True, 'updateFeed': self.latest, 'artifacts': []}
            package = self.artifact(folder, f'Monoprint-0.1.0-{runtime}-full.nupkg', b'package')
            self.artifact(folder, f'Monoprint-{runtime}{suffix}', b'installer')
            feed = {'Assets': [{'FileName': package['file'], 'SHA256': package['sha256'],
                'Size': package['bytes'], 'Version': self.version, 'Type': 'Full'}]}
            self.artifact(folder, f'releases.{runtime}.json', json.dumps(feed).encode())

    def artifact(self, folder, name, data):
        (self.source / folder / name).write_bytes(data)
        entry = {'file': name, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}
        self.receipts[folder]['artifacts'].append(entry)
        return entry

    def build(self, distribution=True, **options):
        for folder, receipt in self.receipts.items():
            (self.source / folder / 'manifest.json').write_text(json.dumps(receipt))
        return assemble(self.source, self.output, version=self.version, commit=self.commit,
                        input_hash='inputs', repository='ozanvos/monoprint', public_key='key', distribution=distribution, **options)

    def test_complete_distribution_has_three_downloads_and_all_feeds(self):
        manifest = self.build()
        self.assertEqual(set(manifest['downloads']), {'macos-arm64', 'win-x64', 'linux-x64'})
        self.assertTrue((self.output / 'appcast-macos-arm64.xml').is_file())
        for line in (self.output / 'SHA256SUMS').read_text().splitlines():
            digest, name = line.split('  ')
            self.assertEqual(hashlib.sha256((self.output / name).read_bytes()).hexdigest(), digest)

    def test_deferred_windows_needs_no_windows_artifacts(self):
        del self.receipts['desktop-win-x64']
        manifest = self.build(platforms=['macos-arm64', 'linux-x64'])
        self.assertEqual(manifest['platforms'], ['macos-arm64', 'linux-x64'])
        self.assertEqual(set(manifest['downloads']), {'macos-arm64', 'linux-x64'})
        self.assertFalse((self.output / 'releases.win-x64.json').exists())

    def test_deferring_windows_does_not_relax_mac_signing(self):
        self.receipts['macos-app']['signed'] = False
        with self.assertRaisesRegex(ValueError, 'macOS requires signing'):
            self.build(platforms=['macos-arm64', 'linux-x64'])

    def test_linux_architectures_ship_distinct_installers_and_feeds(self):
        manifest = self.build(platforms=['macos-arm64', 'linux-x64', 'linux-arm64'])
        for runtime in ['linux-x64', 'linux-arm64']:
            self.assertEqual(manifest['downloads'][runtime]['file'], f'Monoprint-{runtime}.AppImage')
            self.assertTrue((self.output / f'releases.{runtime}.json').is_file())

    def test_arm64_cannot_use_x64_update_channel(self):
        self.receipts['desktop-linux-arm64']['channel'] = 'linux-x64'
        with self.assertRaisesRegex(ValueError, 'update configuration: linux-arm64'):
            self.build(platforms=['linux-x64', 'linux-arm64'])

    def test_selected_arm64_artifacts_cannot_be_missing(self):
        del self.receipts['desktop-linux-arm64']
        with self.assertRaises(FileNotFoundError):
            self.build(platforms=['linux-x64', 'linux-arm64'])

    def test_selected_linux_cannot_be_missing(self):
        del self.receipts['desktop-linux-x64']
        with self.assertRaises(FileNotFoundError):
            self.build(platforms=['macos-arm64', 'linux-x64'])

    def test_rejects_mixed_sources_local_builds_and_unsigned_packages(self):
        cases = [('desktop-win-x64', 'signed', False), ('desktop-linux-x64', 'release', False),
                 ('macos-app', 'stapled', False), ('macos-app', 'updates', False),
                 ('desktop-win-x64', 'commit', 'b' * 40), ('desktop-linux-x64', 'inputHash', 'other'),
                 ('desktop-linux-x64', 'dirty', True), ('desktop-win-x64', 'verified', False),
                 ('desktop-linux-x64', 'channel', 'win-x64'), ('macos-app', 'appStore', True)]
        original = copy.deepcopy(self.receipts)
        for folder, field, value in cases:
            with self.subTest(folder=folder, field=field):
                self.receipts = copy.deepcopy(original)
                self.receipts[folder][field] = value
                with self.assertRaises(ValueError):
                    self.build()
                self.assertFalse(self.output.exists())

    def test_rejects_tampered_archive(self):
        name = self.receipts['macos-app']['archive']
        (self.source / 'macos-app' / name).write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError, 'checksum'):
            self.build()

    def test_rejects_self_consistent_feed_pointing_to_other_version(self):
        folder = 'desktop-win-x64'
        name = 'releases.win-x64.json'
        data = json.loads((self.source / folder / name).read_text())
        data['Assets'][0]['Version'] = '0.2.0'
        self.receipts[folder]['artifacts'] = [entry for entry in self.receipts[folder]['artifacts'] if entry['file'] != name]
        self.artifact(folder, name, json.dumps(data).encode())
        with self.assertRaisesRegex(ValueError, 'shipping bytes'):
            self.build()

    def test_rejects_signed_feed_for_other_archive(self):
        file = self.source / 'macos-feed/appcast.xml'
        file.write_text(file.read_text().replace('/download/v0.1.0/', '/download/v0.2.0/'))
        self.receipts['macos-feed']['appcastSha256'] = hashlib.sha256(file.read_bytes()).hexdigest()
        with self.assertRaisesRegex(ValueError, 'enclosure'):
            self.build()

    def test_rejects_path_traversal(self):
        self.receipts['desktop-win-x64']['artifacts'][0]['file'] = '../escape'
        with self.assertRaisesRegex(ValueError, 'Unsafe'):
            self.build()

    def test_unsigned_candidate_is_never_a_distribution(self):
        self.receipts['macos-app']['signed'] = False
        self.receipts['macos-app']['updates'] = False
        self.receipts['desktop-win-x64']['signed'] = False
        self.receipts['desktop-win-x64']['release'] = False
        manifest = self.build(distribution=False)
        self.assertFalse(manifest['distribution'])
        self.assertFalse((self.output / 'appcast-macos-arm64.xml').exists())


if __name__ == '__main__':
    unittest.main()
