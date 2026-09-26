"""Maintenance-only font import. Records source URLs and checksums; CLI is offline."""
import concurrent.futures
import hashlib
import json
from pathlib import Path
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "assets/fonts"
OUT.mkdir(parents=True, exist_ok=True)

def get(url):
    with urlopen(Request(url, headers={"User-Agent": "monoprint-asset-import"}), timeout=60) as response:
        return response.read()

FONTS = {
    "jetbrains-mono": ("jetbrainsmono", "JetBrains Mono", 500),
    "geist-mono": ("geistmono", "Geist Mono", 400),
    "ibm-plex-mono": ("ibmplexmono", "IBM Plex Mono", 500),
    "fira-code": ("firacode", "Fira Code", 400),
    "roboto-mono": ("robotomono", "Roboto Mono", 400),
    "space-mono": ("spacemono", "Space Mono", 400),
    "source-code-pro": ("sourcecodepro", "Source Code Pro", 400),
    "google-sans-code": ("googlesanscode", "Google Sans Code", 400),
    "inter": ("inter", "Inter", 500),
}

def fetch_font(item):
    key, (directory, family, weight) = item
    listing = json.loads(get(f"https://api.github.com/repos/google/fonts/contents/ofl/{directory}"))
    fonts = [entry for entry in listing if entry["name"].endswith(".ttf") and "Italic" not in entry["name"]]
    preferred = next((entry for entry in fonts if "[" in entry["name"]), None)
    preferred = preferred or next((entry for entry in fonts if ("Medium" if weight == 500 else "Regular") in entry["name"]), fonts[0])
    license_entry = next(entry for entry in listing if entry["name"] == "OFL.txt")
    data = get(preferred["download_url"])
    license_data = get(license_entry["download_url"])
    (OUT / f"{key}.ttf").write_bytes(data)
    (OUT / f"{key}-OFL.txt").write_bytes(license_data)
    return key, dict(family=family, weight=weight, file=f"{key}.ttf", source=preferred["download_url"], blob=preferred["sha"], sha256=hashlib.sha256(data).hexdigest(), license="OFL-1.1")

with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
    manifest = dict(pool.map(fetch_font, FONTS.items()))

# Commit Mono is not in Google Fonts. Its author distributes font assets under OFL.
tree = json.loads(get("https://api.github.com/repos/eigilnikolajsen/commit-mono/git/trees/main?recursive=1"))["tree"]
candidates = [entry for entry in tree if entry["path"].endswith(".otf") and "400" in entry["path"] and "Regular" in entry["path"]]
if not candidates:
    candidates = [entry for entry in tree if entry["path"].endswith(".otf") and "Regular" in entry["path"]]
entry = sorted(candidates, key=lambda e: len(e["path"]))[0]
url = "https://raw.githubusercontent.com/eigilnikolajsen/commit-mono/main/" + entry["path"]
data = get(url)
(OUT / "commit-mono.otf").write_bytes(data)
(OUT / "commit-mono-OFL.txt").write_bytes(get("https://raw.githubusercontent.com/eigilnikolajsen/commit-mono/main/LICENSE-FONT"))
manifest["commit-mono"] = dict(family="CommitMono", weight=400, file="commit-mono.otf", source=url, blob=entry["sha"], sha256=hashlib.sha256(data).hexdigest(), license="OFL-1.1")
(OUT / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
print(f"Imported {len(manifest)} font families and their licenses.")
