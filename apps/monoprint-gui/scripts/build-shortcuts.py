#!/usr/bin/env python3
"""Generate reproducible macOS shortcuts; optionally sign importable files with Apple."""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import subprocess


# Locate the installed bundle, including ~/Applications and renamed bundles.
# No source code or file paths are interpolated into shell commands.
LOCATE = '''set -eu
app=""
for candidate in /Applications/Monoprint.app "$HOME/Applications/Monoprint.app"; do
  if [ -x "$candidate/Contents/MacOS/monoprint-shortcut" ]; then app="$candidate"; break; fi
done
if [ -z "$app" ]; then
  app=$(/usr/bin/mdfind 'kMDItemCFBundleIdentifier == "app.monoprint.studio"' | while IFS= read -r candidate; do
    if [ -x "$candidate/Contents/MacOS/monoprint-shortcut" ]; then printf '%s\\n' "$candidate"; break; fi
  done)
fi
if [ -z "$app" ]; then printf '%s\\n' 'Install Monoprint.app in Applications first.' >&2; exit 1; fi
'''


def workflow(clipboard: bool) -> dict:
    script = LOCATE + ('image=$("$app/Contents/MacOS/monoprint-shortcut" text typescript)\n' if clipboard
                     else 'image=$("$app/Contents/MacOS/monoprint-shortcut" files "$@")\n')
    # Shell actions output text. Decode base64 afterwards to return a real PNG
    # for subsequent Copy, Save, Quick Look or other Shortcuts actions.
    script += '/usr/bin/base64 -i "$image"\n'
    parameters = {
        # Run Shell Script uses unprefixed parameter keys (unlike most actions).
        # https://chutelang.dev/reference/stdlib/scripting.html#runshellscript
        "Script": script,
        "Shell": "/bin/sh",
        "InputMode": "to stdin" if clipboard else "as arguments",
        "RunAsRoot": False,
        "UUID": "E2FEA6B0-1940-41F7-9CC1-49D520CE1921",
    }
    if clipboard:
        parameters["Input"] = action_output("E2FEA6B0-1940-41F7-9CC1-49D520CE1920", "Clipboard")
    else:
        parameters["Input"] = {
            "Value": {"Type": "ExtensionInput"}, "WFSerializationType": "WFTextTokenAttachment",
        }
    return {
        "WFWorkflowClientVersion": "9999",
        "WFWorkflowMinimumClientVersion": 900,
        "WFWorkflowMinimumClientVersionString": "900",
        "WFWorkflowIcon": {"WFWorkflowIconStartColor": 4282601983, "WFWorkflowIconGlyphNumber": 59511},
        "WFWorkflowTypes": ["QuickActions"] if not clipboard else [],
        "WFQuickActionSurfaces": ["Finder"] if not clipboard else [],
        "WFWorkflowInputContentItemClasses": ["WFGenericFileContentItem"] if not clipboard else [],
        "WFWorkflowOutputContentItemClasses": ["WFImageContentItem", "WFGenericFileContentItem"],
        "WFWorkflowHasOutputFallback": False,
        "WFWorkflowHasShortcutInputVariables": not clipboard,
        "WFWorkflowName": "Monoprint Clipboard to PNG" if clipboard else "Monoprint Source File to PNG",
        "WFWorkflowImportQuestions": [],
        "WFWorkflowActions": ([{"WFWorkflowActionIdentifier": "is.workflow.actions.getclipboard", "WFWorkflowActionParameters": {
            "UUID": "E2FEA6B0-1940-41F7-9CC1-49D520CE1920",
        }}] if clipboard else []) + [
            {"WFWorkflowActionIdentifier": "is.workflow.actions.runshellscript", "WFWorkflowActionParameters": parameters},
            {"WFWorkflowActionIdentifier": "is.workflow.actions.base64encode", "WFWorkflowActionParameters": {
                "WFEncodeMode": "Decode", "UUID": "E2FEA6B0-1940-41F7-9CC1-49D520CE1922",
                "WFInput": action_output("E2FEA6B0-1940-41F7-9CC1-49D520CE1921", "Shell Script Result"),
            }},
            {"WFWorkflowActionIdentifier": "is.workflow.actions.setitemname", "WFWorkflowActionParameters": {
                "WFName": "Monoprint.png", "WFDontIncludeFileExtension": False,
                "WFInput": action_output("E2FEA6B0-1940-41F7-9CC1-49D520CE1922", "Base64 Encoded"),
            }},
        ],
    }


def action_output(uuid: str, name: str) -> dict:
    # Matches the attachments in Apple's bundled Gallery .wflow files.
    return {
        "Value": {"Type": "ActionOutput", "OutputUUID": uuid, "OutputName": name},
        "WFSerializationType": "WFTextTokenAttachment",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--sign", action="store_true", help="Produce distributable .shortcut files (requires macOS and Apple's signing service)")
    parser.add_argument("--check", action="store_true", help="Verify the packaged shortcuts match their source and signed hashes")
    args = parser.parse_args()
    if args.sign and args.check:
        parser.error("Choose --sign or --check")
    args.output.mkdir(parents=True, exist_ok=True)
    manifest = {} if args.sign else json.loads((args.output / "manifest.json").read_text()) if args.check else None
    for name, clipboard in [("Monoprint Clipboard to PNG", True), ("Monoprint Source File to PNG", False)]:
        unsigned = args.output / f"{name}.wflow"
        data = plistlib.dumps(workflow(clipboard), fmt=plistlib.FMT_BINARY)
        signed = args.output / f"{name}.shortcut"
        if args.check:
            if unsigned.read_bytes() != data or manifest[name] != {
                "sourceSHA256": hashlib.sha256(data).hexdigest(),
                "signedSHA256": hashlib.sha256(signed.read_bytes()).hexdigest(),
            }:
                raise RuntimeError(f"Regenerate and sign {name}; its packaged files are stale")
            continue
        unsigned.write_bytes(data)
        if args.sign:
            subprocess.run(["/usr/bin/shortcuts", "sign", "--mode", "anyone", "--input", str(unsigned),
                            "--output", str(signed)], check=True)
            signed.chmod(0o644)
            manifest[name] = {"sourceSHA256": hashlib.sha256(data).hexdigest(),
                              "signedSHA256": hashlib.sha256(signed.read_bytes()).hexdigest()}
    if args.sign:
        (args.output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(args.output)


if __name__ == "__main__":
    main()
