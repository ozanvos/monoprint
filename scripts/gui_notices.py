"""Ship licenses and corresponding source for the isolated GUI workspace."""
import hashlib
import json
from pathlib import Path
import shutil
import tomllib


def package_notices(root, gui, resources, metadata):
    manifest = gui / "Cargo.toml"
    inventory = []
    notices = []
    lock = tomllib.loads((gui / 'Cargo.lock').read_text(encoding='utf-8'))
    checksums = {(p['name'], p['version']): p.get('checksum') for p in lock['package']}
    for package in sorted(metadata['packages'], key=lambda p: (p['name'], p['version'])):
        source = Path(package['manifest_path']).parent
        if not package.get('source') and not source.is_relative_to(root / 'vendor'):
            continue
        files = sorted(p for p in source.rglob('*') if p.is_file() and p.name.upper().startswith(('LICENSE', 'LICENCE', 'COPYING', 'NOTICE')))
        record = {k: package[k] for k in ['name', 'version', 'license', 'repository']} | {
            'source': (f"https://crates.io/api/v1/crates/{package['name']}/{package['version']}/download"
                       if package.get('source') else str(source.relative_to(root))),
            'notice_files': [str(p.relative_to(source)) for p in files],
        }
        if package.get('license') in ('EUPL-1.2', 'MPL-2.0'):
            archive_name = f"{package['name']}-{package['version']}.crate"
            archive = source.parents[2] / 'cache' / source.parent.name / archive_name
            source_bytes = archive.read_bytes()
            checksum = hashlib.sha256(source_bytes).hexdigest()
            if checksum != checksums[(package['name'], package['version'])]:
                raise RuntimeError(f'Corresponding source checksum mismatch: {archive_name}')
            destination = resources / 'licenses/sources' / archive_name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(source_bytes)
            record['bundled_source'] = f'sources/{archive_name}'
            record['sha256'] = checksum
        inventory.append(record)
        for path in files:
            notices.append(f"\n{'=' * 72}\n{package['name']} {package['version']} — {path.relative_to(source)}\n{'=' * 72}\n" + path.read_text(encoding='utf-8', errors='replace'))
    (resources / 'licenses/gui-dependencies.json').write_text(json.dumps(inventory, indent=2) + '\n', encoding='utf-8')
    (resources / 'licenses/gui-dependency-notices.txt').write_text('Packaged notices from the optional GUI lockfile, including host build/test dependencies.\n' + ''.join(notices), encoding='utf-8')
    (resources / 'licenses/sources/README.md').write_text(
        '# Corresponding dependency source\n\nUnmodified Giallo (EUPL-1.2) and MPL-2.0 dependency source archives accompany this app. '
        'Extract each `.crate` as a gzip-compressed tar archive. Versions, checksums and exact upstream source URLs are in '
        '`../gui-dependencies.json`; license texts are in `../gui-dependency-notices.txt`. MPL grammar sources are in '
        '`../grammar-sources/`. Original third-party licenses remain applicable.\n', encoding='utf-8')
    shutil.copy2(manifest.parent / 'Cargo.lock', resources / 'gui-Cargo.lock')
