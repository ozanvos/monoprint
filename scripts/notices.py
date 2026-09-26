"""Collect the locked Cargo dependency license declarations and packaged notices."""
import json
import subprocess
from pathlib import Path

metadata=json.loads(subprocess.check_output(['cargo','metadata','--offline','--locked','--format-version','1']))
inventory=[];notices=[]
for package in sorted(metadata['packages'],key=lambda p:(p['name'],p['version'])):
    if not package.get('source'):continue
    root=Path(package['manifest_path']).parent
    files=sorted(p for p in root.rglob('*') if p.is_file() and p.name.upper().startswith(('LICENSE','LICENCE','COPYING','NOTICE')))
    inventory.append({k:package[k] for k in ['name','version','license','repository']}|{'source':f'https://crates.io/api/v1/crates/{package["name"]}/{package["version"]}/download','notice_files':[str(p.relative_to(root)) for p in files]})
    for file in files:
        notices.append(f'\n{"="*72}\n{package["name"]} {package["version"]} — {file.name}\n{"="*72}\n'+file.read_text(errors='replace'))
Path('licenses/rust-dependencies.json').write_text(json.dumps(inventory,indent=2)+'\n')
Path('licenses/rust-dependency-notices.txt').write_text('Packaged license and notice files from the locked Cargo sources.\n'+''.join(notices))
print(f'Inventoried {len(inventory)} crates; preserved {len(notices)} notice files.')
