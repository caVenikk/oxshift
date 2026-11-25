#!/usr/bin/env python3
# The checked-in bundle is a shipped asset, generated only when dependencies change.
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "packaging/runtime"
BUNDLE = DATA / "THIRD-PARTY-LICENSES.txt"
MANIFEST = DATA / "manifest.json"
PRODUCTS = ("crates/desktop/oxshift-platform", "crates/python/oxshift-python",
            "crates/python/oxshift-cli", "crates/python/oxshift-db")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def relative_name(path, root):
    return path.relative_to(root).as_posix()


def external_package(package):
    return bool(package["source"] or "/vendor/" in package["manifest_path"].replace("\\", "/"))


def inputs():
    paths = [ROOT / "Cargo.lock", DATA / "about.toml", Path(__file__).resolve(), ROOT / ".gitattributes"]
    paths += sorted((ROOT / "crates").rglob("Cargo.toml"))
    paths += sorted((ROOT / "vendor").rglob("Cargo.toml"))
    paths.append(ROOT / "Cargo.toml")
    paths += sorted((DATA / "upstream").glob("*"))
    return {relative_name(p, ROOT): digest(p.read_bytes()) for p in paths}


def check():
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    if manifest["inputs"] != inputs():
        raise ValueError("runtime license inputs changed; regenerate with scripts/runtime-notices.py generate")
    data = BUNDLE.read_bytes()
    if not data or digest(data) != manifest["bundle_sha256"]:
        raise ValueError("runtime license bundle missing or modified")
    return data


def generate(executable, reports):
    version = subprocess.check_output([executable, "--version"], text=True, encoding="utf-8").strip()
    if version != "cargo-about 0.9.3":
        raise ValueError("install cargo-about 0.9.3 with --locked --features cli")
    with tempfile.TemporaryDirectory() as temporary:
        if not reports:
            reports = []
            for index, product in enumerate(PRODUCTS):
                output = Path(temporary) / f"{index}.json"
                subprocess.run([executable, "generate", "--locked", "--fail",
                    "-m", str(ROOT / product / "Cargo.toml"),
                    "-c", str(DATA / "about.toml"), "--format", "json", "-o", str(output)], check=True)
                reports.append(output)
        sections = {}
        packages = {}
        for report in reports:
            result = json.loads(Path(report).read_text(encoding="utf-8"))
            for entry in result["crates"]:
                package = entry["package"]
                if external_package(package):
                    packages[package["id"]] = package
            for license in result["licenses"]:
                used = [u["crate"] for u in license["used_by"]
                        if external_package(u["crate"])]
                if not used:
                    continue
                key = (license["id"], license["text"])
                sections.setdefault(key, set()).update(
                    f"{p['name']} {p['version']} | {p['license']} | {p['repository'] or ''} | Authors: {'; '.join(p['authors'])}" for p in used)
        text = ["Runtime licenses: Oxshift and Oxshift Python\n",
                "Union of both desktop products and companion CLIs on macOS arm64/x64, Linux arm64/x64 and Windows x64.\n",
                "Selected SPDX alternatives and every required conjunct are resolved by cargo-about.\n"]
        for (license, body), users in sorted(sections.items()):
            text.extend(["\n" + "=" * 78 + "\n", license + "\n", "\n".join(sorted(users)) + "\n\n", body + "\n"])
        # Preserve original copyright and NOTICE files, including bundled native libraries.
        originals = {}
        for package in sorted(packages.values(), key=lambda p: (p["name"], p["version"])):
            folder = Path(package["manifest_path"]).parent
            for path in sorted(folder.rglob("*"), key=lambda p: tuple(relative_name(p, folder).split("/"))):
                if not path.is_file() or not path.name.lower().startswith(("license", "licence", "copying", "copyright", "notice")):
                    continue
                if path.suffix in (".rs", ".c", ".h", ".py", ".toml", ".json"):
                    continue
                body = path.read_text(encoding="utf-8", errors="replace")
                if not body.strip():
                    continue
                label = f"{package['name']} {package['version']}/{relative_name(path, folder)}"
                originals.setdefault(body, []).append(label)
        for body, labels in sorted(originals.items(), key=lambda item: sorted(item[1])):
            text.extend(["\n" + "=" * 78 + "\nOriginal distribution texts: ", "\n".join(labels), "\n\n", body, "\n"])
        for source in json.loads((DATA / "upstream/sources.json").read_text(encoding="utf-8")):
            body = (DATA / "upstream" / source["file"]).read_bytes()
            if digest(body) != source["sha256"]:
                raise ValueError(f"upstream license checksum mismatch: {source['file']}")
            text.extend(["\n" + "=" * 78 + "\n", source["package"] + " " + source["version"] + "\n",
                         source["url"] + "\n\n", body.decode("utf-8"), "\n"])
        data = ("\n".join(line.rstrip() for line in "".join(text).splitlines()).rstrip() + "\n").encode("utf-8")
        BUNDLE.write_bytes(data)
        metadata = {"generator": version, "inputs": inputs(), "bundle_sha256": digest(data),
                    "packages": [{k: p.get(k) for k in ("name", "version", "license", "repository", "source")}
                                 for p in sorted(packages.values(), key=lambda p: (p["name"], p["version"]))]}
        MANIFEST.write_bytes((json.dumps(metadata, indent=2, sort_keys=True) + "\n").encode("utf-8"))
        print(f"{len(packages)} runtime packages, {len(data)} license bytes")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("check", "generate"))
    parser.add_argument("--cargo-about", default="cargo-about")
    parser.add_argument("--reports", nargs="+", type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.command == "generate":
        generate(args.cargo_about, args.reports)
    else:
        check()
        print("runtime license bundle and dependency inputs verified")


if __name__ == "__main__":
    main()
