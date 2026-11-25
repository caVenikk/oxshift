# One Icon Composer pipeline shared by local app bundles and release packages.
import os
from pathlib import Path
import shutil
import subprocess

ICON_NAMES = {"platform": "Oxshift", "python": "OxshiftPython"}


def compile_icons(source: Path, resources: Path, product: str, stage: Path,
                  minimum: str = "13.0") -> dict:
    name = ICON_NAMES[product]
    icon = stage / (name + ".icon")
    # Resolve artwork symlinks into an isolated Icon Composer document.
    shutil.copytree(source / "assets/brand" / icon.name, icon)
    env = os.environ.copy()
    xcode = Path("/Applications/Xcode.app/Contents/Developer")
    if xcode.is_dir():
        env.setdefault("DEVELOPER_DIR", str(xcode))
    subprocess.run(["xcrun", "actool", "--compile", str(resources), "--app-icon", name,
                    "--platform", "macosx", "--minimum-deployment-target", minimum,
                    "--output-partial-info-plist", str(stage / "icon.plist"), str(icon)],
                   env=env, stdout=subprocess.DEVNULL, check=True)
    info = {"CFBundleIconFile": name + ".icns", "CFBundleIconName": name}
    validate_icons(resources, product, info)
    return info


def validate_icons(resources: Path, product: str, info: dict) -> None:
    name = ICON_NAMES[product]
    if info.get("CFBundleIconName") != name or info.get("CFBundleIconFile") != name + ".icns":
        raise ValueError(f"Icon Composer bundle metadata mismatch for {product}")
    for filename in ("Assets.car", name + ".icns"):
        path = resources / filename
        if not path.is_file() or not path.stat().st_size:
            raise ValueError(f"Missing or empty Icon Composer resource: {filename}")
