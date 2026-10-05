"""Validate Mac App Store credentials, sign the sandboxed app, and package it."""
from datetime import datetime, timezone
import hashlib
from pathlib import Path
import plistlib
import re
import subprocess


def validate_profile(profile, bundle_id, identity, now=None):
    """Reject development, expired, wrong-app and wrong-certificate profiles."""
    now = now or datetime.now(timezone.utc)
    expiry = profile['ExpirationDate'].replace(tzinfo=timezone.utc)
    if expiry <= now:
        raise ValueError('Provisioning profile has expired')
    if profile.get('Platform') != ['OSX']:
        raise ValueError('Use a macOS provisioning profile')
    entitlements = profile.get('Entitlements', {})
    if profile.get('ProvisionedDevices') or profile.get('ProvisionsAllDevices') or entitlements.get('get-task-allow') or entitlements.get('com.apple.security.get-task-allow'):
        raise ValueError('Use a Mac App Store distribution profile, not development or Developer ID')
    team = profile['TeamIdentifier'][0]
    application = entitlements.get('com.apple.application-identifier', '')
    prefixes = profile.get('ApplicationIdentifierPrefix', [team])
    if application not in [f'{prefix}.{bundle_id}' for prefix in prefixes]:
        raise ValueError('Provisioning profile does not match this bundle identifier')
    if entitlements.get('com.apple.developer.team-identifier') != team:
        raise ValueError('Provisioning profile team identifier does not match')
    if identity.upper() not in {hashlib.sha1(cert).hexdigest().upper() for cert in profile['DeveloperCertificates']}:
        raise ValueError('Application signing identity is not authorized by the provisioning profile')
    return {'com.apple.application-identifier': application,
            'com.apple.developer.team-identifier': team}


def find_identity(value, kind, keychain=None):
    """Resolve a named identity to its fingerprint; installer identities use basic policy."""
    command = ['security', 'find-identity', '-v', '-p', 'codesigning' if kind == 'application' else 'basic']
    if keychain:
        command.append(str(keychain))
    output = subprocess.check_output(command, text=True)
    candidates = re.findall(r'\b([A-Fa-f0-9]{40}) "([^"]+)"', output)
    matches = [(fingerprint, name) for fingerprint, name in candidates
               if value.upper() == fingerprint.upper() or value == name]
    if len(matches) != 1:
        raise ValueError(f'Missing or ambiguous {kind} signing identity: {value}')
    fingerprint, name = matches[0]
    allowed = ('Apple Distribution:', '3rd Party Mac Developer Application:') if kind == 'application' else ('3rd Party Mac Developer Installer:', 'Mac Installer Distribution:')
    if not name.startswith(allowed):
        raise ValueError(f'Use a Mac App Store {kind} certificate, not {name}')
    return fingerprint, name


def prepare_credentials(profile_path, bundle_id, application_identity, installer_identity, keychain=None):
    profile = plistlib.loads(subprocess.check_output(['security', 'cms', '-D', '-i', str(profile_path)]))
    application, application_name = find_identity(application_identity, 'application', keychain)
    installer, installer_name = find_identity(installer_identity, 'installer', keychain)
    identifiers = validate_profile(profile, bundle_id, application)
    team = identifiers['com.apple.developer.team-identifier']
    if not installer_name.endswith(f'({team})') or not application_name.endswith(f'({team})'):
        raise ValueError('Application and installer certificates must belong to the provisioning team')
    return {'application': application, 'installer': installer, 'identifiers': identifiers,
            'profileUUID': profile['UUID'], 'profileExpiration': profile['ExpirationDate'].isoformat()}


def signed_entitlements(template, credentials, destination):
    entitlements = plistlib.loads(Path(template).read_bytes())
    entitlements.update(credentials['identifiers'])
    Path(destination).write_bytes(plistlib.dumps(entitlements))
    return Path(destination)


def clear_bundle_quarantine(bundle):
    """Remove download quarantine from assembled Store payloads before signing.

    copy2/copytree preserve extended attributes, including those on downloaded
    provisioning profiles. Keep source files and all other attributes intact.
    """
    subprocess.run(['xattr', '-drs', 'com.apple.quarantine', str(bundle)], check=True)


def package_app(bundle, destination, installer_identity=None, keychain=None):
    destination = Path(destination)
    if destination.exists():
        raise FileExistsError(f'Refusing to overwrite package: {destination}')
    command = ['productbuild', '--component', str(bundle), '/Applications']
    if installer_identity:
        command += ['--sign', installer_identity]
        if keychain:
            command += ['--keychain', str(keychain)]
    subprocess.run([*command, str(destination)], check=True)
    if installer_identity:
        subprocess.run(['pkgutil', '--check-signature', str(destination)], check=True)
