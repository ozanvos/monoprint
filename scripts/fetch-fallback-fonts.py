"""Fetch the reviewed fallback fonts byte-for-byte, or verify them offline.

Sources, revisions, checksums and license URLs are reviewed in the separate
assets/fonts/fallback/manifest.json. Never discovers or upgrades versions.
"""
import argparse
import concurrent.futures
import hashlib
import json
from pathlib import Path
import re
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets/fonts/fallback"
PINNED_URL = re.compile(r"https://raw\.githubusercontent\.com/[^/]+/[^/]+/[0-9a-f]{40}/.+")


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def local_file(name):
    if Path(name).name != name or name in (".", ".."):
        raise ValueError(f"Not a local asset filename: {name!r}")
    return ASSETS / name


def notice_text(manifest):
    lines = ["MONOPRINT FALLBACK FONT NOTICES",
             "Fonts are unmodified and remain under SIL OFL-1.1. See the referenced full license files.",
             "Original font metadata, including copyright and license statements, remains embedded.", ""]
    for font in manifest.values():
        lines.extend([f"{font['family']} — {font['file']}", font["copyright"],
                      f"License: {font['license_file']}", f"Source: {font['source']}", ""])
    return "\n".join(lines)


def validate_manifest(manifest):
    if len(manifest) != 8:
        raise ValueError("Expected the eight reviewed fallback faces; review inventory changes explicitly")
    sources = {}
    for font in manifest.values():
        if font["license"] != "OFL-1.1" or font["modified"]:
            raise ValueError(f"Unexpected font license or modification: {font['file']}")
        for field in ("source", "license_source"):
            if not PINNED_URL.fullmatch(font[field]):
                raise ValueError(f"Source must use an immutable commit: {font[field]}")
        expected_prefix = f"https://raw.githubusercontent.com/{font['repository']}/{font['revision']}/"
        if not font["source"].startswith(expected_prefix):
            raise ValueError(f"Source and revision disagree: {font['file']}")
        for name, url, digest in [(font["file"], font["source"], font["sha256"]),
                                  (font["license_file"], font["license_source"], font["license_sha256"])]:
            local_file(name)
            item = (url, digest)
            if name in sources and sources[name] != item:
                raise ValueError(f"Conflicting sources for {name}")
            sources[name] = item
    return sources


def download(item):
    filename, (url, expected_hash) = item
    with urlopen(Request(url, headers={"User-Agent": "monoprint-fallback-font-import"}), timeout=90) as response:
        data = response.read()
    if sha256(data) != expected_hash:
        raise ValueError(f"Upstream checksum mismatch: {filename}; retained local asset unchanged")
    destination = local_file(filename)
    temporary = destination.with_suffix(destination.suffix + ".download")
    temporary.write_bytes(data)
    temporary.replace(destination)


def check(manifest, sources):
    for filename, (_, expected_hash) in sources.items():
        if sha256(local_file(filename).read_bytes()) != expected_hash:
            raise ValueError(f"Local checksum mismatch: {filename}")
    for font in manifest.values():
        data = local_file(font["file"]).read_bytes()
        if len(data) != font["bytes"]:
            raise ValueError(f"Byte count mismatch: {font['file']}")
        git_blob = hashlib.sha1(f"blob {len(data)}\0".encode() + data).hexdigest()
        if git_blob != font["blob"]:
            raise ValueError(f"Upstream Git blob mismatch: {font['file']}")
    if (ASSETS / "NOTICE.txt").read_text() != notice_text(manifest):
        raise ValueError("Fallback font NOTICE.txt differs from the reviewed manifest")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Validate local font/license/provenance bytes without network access")
    args = parser.parse_args()
    manifest = json.loads((ASSETS / "manifest.json").read_text())
    sources = validate_manifest(manifest)
    if not args.check:
        with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
            list(pool.map(download, sources.items()))
        (ASSETS / "NOTICE.txt").write_text(notice_text(manifest))
    check(manifest, sources)
    print(f"Verified {len(manifest)} unmodified OFL fallback fonts ({sum(font['bytes'] for font in manifest.values()):,} bytes), licenses and provenance.")


if __name__ == "__main__":
    main()
