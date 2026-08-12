#!/usr/bin/env python3
"""Package a source crate's release build into package.aix (a zip Aidoku can unzip).

Usage: `python pack.py <src_dir>` — <src_dir> is a source crate directory, e.g.
`sources/zh.hipmh` or `sources/zh.bilimanga`. The wasm is located by globbing
`<src_dir>/target/wasm32-unknown-unknown/release/*.wasm` (one per crate); `res/`
must sit in the same directory. Output goes to `<src_dir>/package.aix`.

PowerShell's Compress-Archive writes zip entry names with backslash path
separators on Windows. On iOS/macOS, ZIPFoundation treats backslash as a
literal filename character, so "Payload/main.wasm" written with a backslash
becomes one file instead of a "Payload/" folder and the source fails to
load (解压/解析失败).

This script writes entry names with forward slashes so the archive extracts
to the `Payload/` directory Aidoku expects. Usage: `python pack.py sources/zh.hipmh`
after `cargo +nightly build --release` in that crate.
"""
import glob
import os
import sys
import zipfile


def main() -> int:
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} <src_dir>", file=sys.stderr)
        return 2
    src_dir = sys.argv[1]

    release = os.path.join(src_dir, "target", "wasm32-unknown-unknown", "release")
    wasms = glob.glob(os.path.join(release, "*.wasm"))
    if not wasms:
        print(
            f"error: no .wasm found in {release} — run `cargo +nightly build --release` "
            f"in {src_dir} first",
            file=sys.stderr,
        )
        return 1
    if len(wasms) > 1:
        print(
            f"error: multiple .wasm files in {release}: {wasms} — expected exactly one",
            file=sys.stderr,
        )
        return 1
    wasm = wasms[0]

    res_dir = os.path.join(src_dir, "res")
    if not os.path.isdir(res_dir):
        print(f"error: {res_dir} directory not found", file=sys.stderr)
        return 1

    out = os.path.join(src_dir, "package.aix")
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        # main.wasm first, then res files, all under a forward-slash Payload/ prefix.
        z.write(wasm, "Payload/main.wasm")
        for name in sorted(os.listdir(res_dir)):
            z.write(os.path.join(res_dir, name), f"Payload/{name}")

    with zipfile.ZipFile(out) as z:
        for info in z.infolist():
            assert "/" in info.filename and "\\" not in info.filename, f"bad entry name: {info.filename!r}"
    print(f"Build complete: {out} ({os.path.getsize(out)} bytes, forward-slash entries)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
