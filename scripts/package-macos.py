#!/usr/bin/env python3
HELP = """Package built desktop products as signed macOS application bundles.

Requires macOS and Xcode with Icon Composer support. Build the binaries first:
  cargo build --release -p oxshift-platform -p oxshift-python -p oxshift-cli -p oxshift-db
  python3 scripts/package-macos.py --binaries target/release --output /tmp/oxshift-apps

Local bundles use ad hoc signing by default; --sign accepts a signing identity.
This does not notarize the applications or produce a distributable installer.
"""

import argparse
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import sys
import tempfile

PRODUCTS = {
    "platform": ("Oxshift", "Oxshift", "dev.oxshift.platform", ("oxshift-platform",)),
    "python": ("Oxshift Python", "OxshiftPython", "dev.oxshift.python",
               ("oxshift-python", "oxshift", "oxshift-db")),
}


def sign(path: Path, identifier: str, identity: str) -> None:
    subprocess.run(["codesign", "--force", "--sign", identity, "--identifier", identifier,
                    "-r=designated => identifier \"" + identifier + "\"", str(path)], check=True)


def package_product(source: Path, binaries: Path, output: Path, product: str,
                    identity: str = "-") -> Path:
    display, icon_name, bundle_id, programs = PRODUCTS[product]
    destination = output / (display + ".app")
    if destination.exists():
        raise FileExistsError(f"Bundle already exists: {destination}; choose an empty output directory")
    for program in programs:
        path = binaries / program
        if not path.is_file() or not os.access(path, os.X_OK):
            raise FileNotFoundError(f"Missing executable: {path}")
    manifest = (source / "Cargo.toml").read_text()
    package = re.search(r"(?ms)^\[workspace\.package\]\s*\n(.*?)(?=^\[|\Z)", manifest)
    version_match = re.search(r'^version\s*=\s*"([^"\n]+)"', package[1], re.M) if package else None
    if not version_match:
        raise ValueError("Missing literal workspace.package.version in Cargo.toml")
    version = version_match[1]
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".package-", dir=output) as temporary:
        stage = Path(temporary)
        bundle = stage / destination.name
        macos = bundle / "Contents" / "MacOS"
        resources = bundle / "Contents" / "Resources"
        macos.mkdir(parents=True)
        resources.mkdir()
        icon = stage / (icon_name + ".icon")
        # Resolve artwork symlinks into the temporary Icon Composer document.
        shutil.copytree(source / "assets" / "brand" / icon.name, icon)
        env = os.environ.copy()
        xcode = Path("/Applications/Xcode.app/Contents/Developer")
        if xcode.is_dir():
            env.setdefault("DEVELOPER_DIR", str(xcode))
        subprocess.run(["xcrun", "actool", "--compile", str(resources), "--app-icon", icon_name,
                        "--platform", "macosx", "--minimum-deployment-target", "11.0",
                        "--output-partial-info-plist", str(stage / "icon.plist"), str(icon)],
                       env=env, stdout=subprocess.DEVNULL, check=True)
        for name in ("Assets.car", icon_name + ".icns"):
            if not (resources / name).stat().st_size:
                raise ValueError(f"Empty icon resource: {name}")
        info = dict(CFBundleName=display, CFBundleDisplayName=display,
                    CFBundleExecutable=programs[0], CFBundleIdentifier=bundle_id,
                    CFBundlePackageType="APPL", CFBundleVersion=version,
                    CFBundleShortVersionString=version.removesuffix(".0") if version.startswith("20") else version,
                    CFBundleInfoDictionaryVersion="6.0", CFBundleIconFile=icon_name + ".icns",
                    CFBundleIconName=icon_name, NSHighResolutionCapable=True,
                    LSApplicationCategoryType="public.app-category.developer-tools")
        (bundle / "Contents" / "Info.plist").write_bytes(plistlib.dumps(info))
        for program in programs:
            shutil.copy2(binaries / program, macos / program)
            if program != programs[0]:
                sign(macos / program, program, identity)
        sign(bundle, bundle_id, identity)
        subprocess.run(["codesign", "--verify", "--strict", "--deep", str(bundle)], check=True)
        bundle.rename(destination)
    return destination


def main() -> None:
    parser = argparse.ArgumentParser(description=HELP, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--binaries", type=Path, default=Path("target/release"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--product", choices=["all", *PRODUCTS], default="all")
    parser.add_argument("--sign", default="-", help="codesign identity (default: local ad hoc)")
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("macOS with Xcode is required")
    source = Path(__file__).resolve().parent.parent
    products = PRODUCTS if args.product == "all" else [args.product]
    for product in products:
        print(package_product(source, args.binaries.resolve(), args.output.resolve(), product, args.sign))


if __name__ == "__main__":
    main()
