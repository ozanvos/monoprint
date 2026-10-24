"""Release inputs shared by desktop packaging; no publishing side effects."""
import base64
import json
from pathlib import Path
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]


def configuration():
    return json.loads((ROOT / 'release/updates.json').read_text())


def linuxdeploy_settings(runtime):
    config = configuration()['linuxdeploy']
    archive = config['archives'][runtime]
    return (f"https://github.com/linuxdeploy/linuxdeploy/releases/download/{config['version']}/{archive['file']}",
            archive['sha256'])


def https_url(value):
    parsed = urlsplit(value or '')
    if parsed.scheme != 'https' or not parsed.hostname or parsed.username or parsed.password or parsed.fragment:
        raise ValueError('An HTTPS update URL without credentials or fragments is required')
    return value


def sparkle_settings(feed=None, public_key=None):
    config = configuration()['sparkle']
    feed = https_url(feed or config['feedUrl'])
    public_key = public_key or config['publicKey']
    try:
        key = base64.b64decode(public_key or '', validate=True)
    except ValueError as error:
        raise ValueError('Sparkle requires a base64 Ed25519 public key') from error
    if len(key) != 32:
        raise ValueError('Sparkle requires a 32-byte Ed25519 public key')
    return {'SUFeedURL': feed, 'SUPublicEDKey': public_key,
            'SUEnableAutomaticChecks': True, 'SUAutomaticallyUpdate': False}


def velopack_settings(runtime, feed=None):
    config = configuration()['velopack']
    return https_url(feed or config['feeds'][runtime]), config['version']
