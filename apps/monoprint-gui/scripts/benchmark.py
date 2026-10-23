#!/usr/bin/env python3
"""Record release-mode Studio CPU frame benchmarks on macOS (requires Metal)."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import tempfile
import tomllib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True, help='New JSON result file')
    parser.add_argument('--label', required=True, help='Dependency stack being measured')
    parser.add_argument('--runs', type=int, default=3, help='Independent process runs (default: 3)')
    args = parser.parse_args()
    if platform.system() != 'Darwin':
        parser.error('This benchmark requires the macOS headless Metal renderer')
    if args.runs < 1:
        parser.error('--runs must be positive')
    if args.output.exists():
        parser.error('--output must be a new file')
    gui = Path(__file__).resolve().parents[1]
    root = gui.parents[1]
    command = ['cargo', 'test', '--locked', '--release', '--manifest-path',
               str(gui / 'Cargo.toml'), '--test', 'studio']
    measurements = []
    with tempfile.TemporaryDirectory(prefix='monoprint-benchmark-') as directory:
        for run in range(args.runs):
            raw = Path(directory) / f'{run}.jsonl'
            env = os.environ | {'MONOPRINT_GUI_BENCHMARK': str(raw), 'MONOPRINT_SESSION': 'off'}
            completed = subprocess.run(command, cwd=root, env=env, text=True,
                                       stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            if completed.returncode:
                print(completed.stdout)
                completed.check_returncode()
            print(f'{args.label}: process {run + 1}/{args.runs} complete', flush=True)
            records = [json.loads(line) for line in raw.read_text().splitlines()]
            if len(records) != 8:
                raise RuntimeError(f'Expected eight workload/size results, got {len(records)}')
            measurements.extend(record | {'run': run + 1} for record in records)
    result = {
        'label': args.label,
        'platform': platform.platform(),
        'machine': platform.machine(),
        'rustc': subprocess.check_output(['rustc', '--version'], text=True).strip(),
        'revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
        'dirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=root, text=True).strip()),
        'lock_sha256': hashlib.sha256((gui / 'Cargo.lock').read_bytes()).hexdigest(),
        'gpui_packages': [
            {'name': package['name'], 'version': package['version']}
            for package in tomllib.loads((gui / 'Cargo.lock').read_text())['package']
            if package['name'].startswith('gpui-')
        ],
        'scope': 'CPU scene construction and arena cleanup; excludes GPU present, VSync and preview worker',
        'measurements': measurements,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x') as output:
        output.write(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    main()
