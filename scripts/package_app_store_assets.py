#!/usr/bin/env python3
"""Validate and package native screenshots, icon references and listing drafts.

Requires Pillow. Only normalizes PNG color/alpha; does not alter screenshot UI.
"""
import argparse
import hashlib
import html
import io
import json
from pathlib import Path
import plistlib
import subprocess
import zipfile

from PIL import Image, ImageCms, ImageDraw


def main():
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--captures', type=Path,
                        default=root / 'output/app-store-capture/native')
    args = parser.parse_args()
    srgb = ImageCms.ImageCmsProfile(ImageCms.createProfile('sRGB'))
    srgb_bytes = srgb.tobytes()

    def normalized_rgba(image):
        rgba = image.convert('RGBA')
        if profile := image.info.get('icc_profile'):
            rgba = ImageCms.profileToProfile(
                rgba, ImageCms.ImageCmsProfile(io.BytesIO(profile)), srgb, outputMode='RGBA')
        return rgba

    assets = root / 'release/app-store'
    metadata = assets / 'metadata/en-US'
    limits = {'name': 30, 'subtitle': 30, 'promotional-text': 170, 'description': 4000}
    counts = {}
    for name, limit in limits.items():
        value = (metadata / f'{name}.txt').read_text().rstrip('\n')
        assert value and len(value) <= limit, f'{name} exceeds {limit} characters'
        counts[name] = {'characters': len(value), 'limit': limit}
    keywords = (metadata / 'keywords.txt').read_text().rstrip('\n')
    assert len(keywords.encode()) <= 100, 'Keywords exceed 100 bytes'
    assert all(len(k) >= 3 for k in keywords.split(',')), 'Keywords must exceed 2 characters'
    counts['keywords'] = {'bytes': len(keywords.encode()), 'limit': 100}

    labels = ['TypeScript · dark interface', 'Rust · light interface and theme',
              'Python · line highlights', 'Hem · Ground',
              'Transparent SVG · light interface']
    screenshots = []
    for fixture, label in zip(sorted((assets / 'fixtures').glob('*.json')), labels, strict=True):
        source = args.captures / f'{fixture.stem}.png'
        capture = json.loads(source.with_suffix('.json').read_text())
        appearance = capture['interfaceAppearance']
        with Image.open(source) as image:
            assert image.size == (2880, 1800), f'{source}: unexpected window dimensions'
            rgba = normalized_rgba(image)
            # An empty area in the source gutter verifies the actual interface mode.
            gutter = rgba.getpixel((10, 100))[:3]
            assert (min(gutter) > 200 if appearance == 'light' else max(gutter) < 80), (
                f'{source}: interface appearance disagrees with capture provenance')
            background = Image.new('RGBA', image.size,
                                   '#f7f7f5' if appearance == 'light' else '#212120')
            background.alpha_composite(rgba)
            destination = assets / 'screenshots' / source.name
            background.convert('RGB').save(destination, optimize=True, icc_profile=srgb_bytes)
            # Apple's smaller accepted size, from the same Retina capture.
            small = assets / 'screenshots/1440x900' / source.name
            small.parent.mkdir(exist_ok=True)
            background.convert('RGB').resize((1440, 900), Image.Resampling.LANCZOS).save(
                small, optimize=True, icc_profile=srgb_bytes)
        screenshots.append({'file': str(destination.relative_to(assets)), 'label': label,
                            'size': [2880, 1800], 'colorMode': 'RGB', 'alpha': False,
                            'captureSHA256': hashlib.sha256(source.read_bytes()).hexdigest(),
                            **capture})

    icon = assets / 'icon/Monoprint-1024.png'
    with Image.open(icon) as image:
        assert image.size == (1024, 1024)
        reference = Image.new('RGBA', image.size, '#212120')
        reference.alpha_composite(normalized_rgba(image))
        reference.convert('RGB').save(assets / 'icon/Monoprint-1024-opaque.png',
                                     optimize=True, icc_profile=srgb_bytes)

    sheet = Image.new('RGB', (1504, 1540), '#f7f7f5')
    draw = ImageDraw.Draw(sheet)
    for index, shot in enumerate(screenshots):
        x, y = 24 + (index % 2) * 752, 28 + (index // 2) * 502
        draw.text((x, y), f'{index + 1}. {shot["label"]}', fill='#2a2a28')
        with Image.open(assets / shot['file']) as image:
            sheet.paste(image.resize((720, 450), Image.Resampling.LANCZOS), (x, y + 28))
    sheet.save(assets / 'contact-sheet.jpg', quality=92)

    cards = ''.join(f'<figure><a href="{shot["file"]}"><img src="{shot["file"]}" '
                    f'alt="{html.escape(shot["label"])}"></a><figcaption>{i + 1}. '
                    f'{html.escape(shot["label"])}</figcaption></figure>'
                    for i, shot in enumerate(screenshots))
    description = html.escape((metadata / 'description.txt').read_text())
    (assets / 'preview.html').write_text('''<!doctype html><html lang="en"><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Monoprint App Store assets</title><style>
body{background:#f7f7f5;color:#2a2a28;font:16px/1.6 -apple-system,BlinkMacSystemFont,sans-serif;margin:36px}
main{max-width:1500px;margin:auto}header{display:flex;gap:24px;align-items:center}header img{width:100px}
h1{margin:0;font-size:34px}a{color:#526fd2}.gallery{display:grid;grid-template-columns:repeat(auto-fit,minmax(480px,1fr));gap:24px}
figure{margin:0}figure img{width:100%;display:block;border-radius:12px}figcaption{padding:8px 0}
pre{white-space:pre-wrap;font:inherit;max-width:760px}small{color:#7c7c77}@media(max-width:560px){body{margin:16px}.gallery{display:block}}
</style></head><body><main><header><img src="icon/Monoprint-1024.png" alt="Monoprint icon">
<div><h1>Monoprint</h1><p>Code images, made on your Mac</p></div></header>
<p>App Store screenshots and listing text (en-US).</p>
<p><a href="README.md">Submission guide</a> · <a href="pages/support.html">Support page</a> · <a href="pages/privacy.html">Privacy policy</a></p>
<div class="gallery">''' + cards + '</div><h2>Listing description</h2><pre>' + description + '</pre></main></body></html>\n')

    bundle = root / 'target/gui-app-store/Monoprint.app'
    info = plistlib.loads((bundle / 'Contents/Info.plist').read_bytes())
    manifest = {
        'generatedOn': '2026-10-05', 'locale': 'en-US',
        'sourceCommit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
        'sourceHasUncommittedChanges': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=root)),
        'bundleIdentifier': info['CFBundleIdentifier'], 'version': info['CFBundleShortVersionString'],
        'appStoreFeature': True, 'guiBinarySHA256': hashlib.sha256((bundle / 'Contents/MacOS/monoprint-gui').read_bytes()).hexdigest(),
        'screenshots': screenshots, 'metadataValidation': counts,
        'screenshotProcessing': 'Native window capture without shadow; ICC-managed conversion to sRGB; flatten corner alpha; RGB PNG; half-size copies use Lanczos downsampling. No UI repainting or marketing overlays.',
        'icon': 'Existing Icon Composer source and compiled assets. Mac App Store icon is carried in the uploaded build. Standalone PNGs are references.',
        'publication': 'Submission assets local; website public; App Store release pending',
    }
    manifest['files'] = {str(path.relative_to(assets)): hashlib.sha256(path.read_bytes()).hexdigest()
                         for path in sorted(assets.rglob('*')) if path.is_file() and path.name != 'manifest.json'}
    (assets / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    output = root / 'output/app-store/Monoprint-App-Store-assets.zip'
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(assets.rglob('*')):
            if path.is_file():
                archive.write(path, 'Monoprint-App-Store-assets/' + str(path.relative_to(assets)))
    print(json.dumps({'archive': str(output), 'screenshots': len(screenshots), 'metadata': counts}, indent=2))


if __name__ == '__main__':
    main()
