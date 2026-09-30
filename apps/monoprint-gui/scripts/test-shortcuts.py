#!/usr/bin/env python3
"""Exercise the shortcut bridge against a real CLI, without changing the clipboard."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
HELPER = Path(__file__).with_name("monoprint-shortcut")
CLI = Path(os.environ.get("MONOPRINT_CLI", ROOT / "target/release/monoprint")).resolve()


class ShortcutBridgeTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="monoprint-shortcut-test-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.output = self.root / "Pictures with spaces"
        self.environment = os.environ | {"MONOPRINT_CLI": str(CLI), "MONOPRINT_OUTPUT_DIR": str(self.output)}

    def bridge(self, *args, source=None):
        return subprocess.run(["/bin/sh", str(HELPER), *map(str, args)], input=source,
                              env=self.environment, capture_output=True, check=False)

    def test_file_input_handles_spaces_unicode_and_shell_characters(self):
        source = self.root / "示例 #1 $(touch SHOULD_NOT_EXIST).ts"
        source.write_text("const message = 'Hello 👋';\n")
        result = self.bridge("files", source)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        image = Path(result.stdout.decode().strip())
        expected = self.root / "expected.png"
        subprocess.run([str(CLI), str(source), "--title", source.name, "--output", str(expected),
                        "--scale", "2", "--png-compression", "balanced"], check=True, capture_output=True)
        self.assertEqual(image.read_bytes(), expected.read_bytes())
        self.assertFalse((self.root / "SHOULD_NOT_EXIST").exists())
        self.assertTrue(image.name.endswith(".png"))

    def test_text_input_keeps_unicode_and_final_newlines(self):
        source = "// 中文 👩🏽‍💻\nconst n = 1;\n\n".encode()
        result = self.bridge("text", "typescript", source=source)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        expected = self.root / "expected.png"
        subprocess.run([str(CLI), "-", "--language", "typescript", "--output", str(expected),
                        "--scale", "2", "--png-compression", "balanced"], input=source, check=True, capture_output=True)
        self.assertEqual(Path(result.stdout.decode().strip()).read_bytes(), expected.read_bytes())

    def test_repeated_exports_never_replace_previous_images(self):
        first = self.bridge("text", "typescript", source=b"const n = 1;")
        second = self.bridge("text", "typescript", source=b"const n = 2;")
        self.assertEqual(first.returncode, 0, first.stderr.decode())
        self.assertEqual(second.returncode, 0, second.stderr.decode())
        first_path, second_path = map(lambda r: Path(r.stdout.decode().strip()), (first, second))
        self.assertNotEqual(first_path, second_path)
        self.assertNotEqual(first_path.read_bytes(), second_path.read_bytes())

    def test_failed_inputs_leave_no_partial_exports(self):
        invalid = self.root / "binary.rs"
        invalid.write_bytes(b"\xff\xfe")
        for args, data in [(("text", "typescript"), b""), (("text", "typescript"), b"\xff"),
                           (("text", "typescript"), b"x" * (1024 * 1024 + 10)),
                           (("files", invalid), None), (("files", invalid, invalid), None)]:
            result = self.bridge(*args, source=data)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(result.stdout, b"")
            self.assertEqual(list(self.output.iterdir()) if self.output.exists() else [], [])


if __name__ == "__main__":
    unittest.main()
