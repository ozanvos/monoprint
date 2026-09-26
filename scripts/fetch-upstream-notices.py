"""Fill missing packaged notices from each crate's exact recorded Git revision."""
import concurrent.futures
import hashlib
import json
import subprocess
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request, urlopen

metadata=json.loads(subprocess.check_output(['cargo','metadata','--offline','--locked','--format-version','1']))
out=Path('licenses/upstream');out.mkdir(parents=True,exist_ok=True)
tasks={}
for package in metadata['packages']:
    if not package.get('source'):continue
    root=Path(package['manifest_path']).parent
    if any(p.is_file() and p.name.upper().startswith(('LICENSE','LICENCE','COPYING')) for p in root.iterdir()):continue
    vcs=root/'.cargo_vcs_info.json'
    repo=(package.get('repository') or '').removesuffix('/').removesuffix('.git')
    if not vcs.exists() or not repo.startswith('https://github.com/'):continue
    sha=json.loads(vcs.read_text())['git']['sha1']
    tasks.setdefault((repo,sha),[]).append(package['name'])

def collect(item):
    (repo,sha),packages=item
    results=[]
    for name in ['LICENSE','LICENSE.md','LICENSE.txt','LICENSE-MIT','LICENSE-APACHE','COPYING']:
        url=f'https://raw.githubusercontent.com/{repo.removeprefix("https://github.com/")}/{sha}/{name}'
        try:
            with urlopen(Request(url,headers={'User-Agent':'monoprint-notice-import'}),timeout=30) as response: data=response.read()
        except HTTPError as e:
            if e.code==404:continue
            raise
        file=out/f'{packages[0]}-{name}.txt';file.write_bytes(data)
        results.append({'packages':packages,'file':str(file),'source':url,'sha256':hashlib.sha256(data).hexdigest()})
    return results

with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
    files=[item for group in pool.map(collect,tasks.items()) for item in group]
(out/'manifest.json').write_text(json.dumps(files,indent=2)+'\n')
print(f'Preserved {len(files)} notices from exact upstream revisions.')
