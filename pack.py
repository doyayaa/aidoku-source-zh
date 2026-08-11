#!/usr/bin/env python3
"""Package the release build into package.aix (a zip Aidoku can unzip).

PowerShell's Compress-Archive writes zip entry names with backslash path
separators on Windows. On iOS/macOS, ZIPFoundation treats backslash as a
literal filename character, so "Payload/main.wasm" written with a backslash
becomes one file instead of a "Payload/" folder and the source fails to
load (解压/解析失败).

This script writes entry names with forward slashes so the archive extracts
to the `Payload/` directory Aidoku expects. Usage: `python pack.py` after
`cargo +nightly build --release`.
"""
import os
import sys
import zipfile

RELEASE = os.path.join("target", "wasm32-unknown-unknown", "release")
OUT = "package.aix"


def main() -> int:
    wasm = os.path.join(RELEASE, "hipmh.wasm")
    if not os.path.exists(wasm):
        print(f"error: {wasm} not found — run `cargo +nightly build --release` first", file=sys.stderr)
        return 1
    if not os.path.isdir("res"):
        print("error: res/ directory not found", file=sys.stderr)
        return 1

    with zipfile.ZipFile(OUT, "w", zipfile.ZIP_DEFLATED) as z:
        # main.wasm first, then res files, all under a forward-slash Payload/ prefix.
        z.write(wasm, "Payload/main.wasm")
        for name in sorted(os.listdir("res")):
            z.write(os.path.join("res", name), f"Payload/{name}")

    with zipfile.ZipFile(OUT) as z:
        for info in z.infolist():
            assert "/" in info.filename and "\\" not in info.filename, f"bad entry name: {info.filename!r}"
    print(f"Build complete: {OUT} ({os.path.getsize(OUT)} bytes, forward-slash entries)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
