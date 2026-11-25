#!/usr/bin/env python3
# Native build tool and embedded runtime, both pinned by digest. No execution.
import argparse
import hashlib
from pathlib import Path
import platform
import urllib.request

PINS = {
    "x86_64": ("ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0",
               "2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d"),
    "aarch64": ("f0837e7448a0c1e4e650a93bb3e85802546e60654ef287576f46c71c126a9158",
                "00cbdfcf917cc6c0ff6d3347d59e0ca1f7f45a6df1a428a0d6d8a78664d87444"),
}


def fetch(url, expected, destination):
    with urllib.request.urlopen(url, timeout=60) as response:
        data = response.read(128 * 1024 * 1024 + 1)
    if hashlib.sha256(data).hexdigest() != expected:
        raise ValueError(f"SHA-256 mismatch: {url}")
    destination.write_bytes(data)
    destination.chmod(0o755)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dest", type=Path, required=True)
    parser.add_argument("--arch", choices=PINS, default=platform.machine())
    args = parser.parse_args()
    if args.arch not in PINS:
        parser.error("unsupported architecture")
    args.dest.mkdir(parents=True, exist_ok=True)
    tool, runtime = PINS[args.arch]
    fetch(f"https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-{args.arch}.AppImage",
          tool, args.dest / "appimagetool")
    fetch(f"https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-{args.arch}",
          runtime, args.dest / "runtime")
    print("AppImage tool 1.9.1 and runtime 20251108: SHA-256 verified")


if __name__ == "__main__":
    main()
