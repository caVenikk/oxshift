#!/usr/bin/env python3
# Native release packaging for both products (docs/release.md).
# Adapted from the archived release packager; standard library only.
# This tool creates local files only, never tags, uploads or publishes.
import argparse
import hashlib
import os
import platform
import plistlib
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path

from macos_icons import compile_icons, validate_icons

ROOT = Path(__file__).resolve().parent.parent
BRAND = ROOT / "crates" / "python" / "brand" / "src" / "lib.rs"
PACKAGING = ROOT / "packaging"
FORMULA_TEMPLATE = PACKAGING / "homebrew" / "formula.rb.in"
NOTES_TEMPLATE = PACKAGING / "release-notes.md.in"
INSTALL_SCRIPT = PACKAGING / "install.sh.in"
FONT_LICENSES = ROOT / "crates" / "platform" / "ui" / "assets" / "fonts"

# The CPUs a release is built for, by the names of the archives.
ARCHES = {"arm64": "aarch64", "aarch64": "aarch64", "amd64": "x86_64", "x86_64": "x86_64"}
# The macOS the bundle declares as its minimum (not tested on older systems).
MACOS_MINIMUM = "13.0"


def fail(message):
    print(f"package.py: {message}", file=sys.stderr)
    sys.exit(1)


def run(command, **kwargs):
    result = subprocess.run(command, text=True, **kwargs)
    if result.returncode != 0:
        fail(f"`{' '.join(str(c) for c in command)}` failed ({result.returncode})")
    return result


def python():
    return sys.executable


# ---------------------------------------------------------------- names


class Names:
    """Release identity read from the product's Rust brand module."""

    def __init__(self, product="python"):
        self.kind = product
        cargo = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
        version = re.search(r'^\[workspace\.package\][^\[]*?^version = "([^"]+)"', cargo, re.M | re.S)
        repository = re.search(r'^repository = "https://github\.com/([^"/]+/[^"/]+)"', cargo, re.M)
        if not version or not repository:
            fail("missing workspace version or repository")
        self.package_version = version.group(1)
        self.version = release_version(self.package_version)
        self.repository = repository.group(1)
        if product == "python":
            text = BRAND.read_text(encoding="utf-8")
            def const(name):
                found = re.search(rf'pub const {name}: &str = "([^"]+)";', text)
                if not found:
                    fail(f"missing brand constant {name}")
                return found.group(1)
            self.product = const("PRODUCT_NAME")
            self.full_name = const("PRODUCT_FULL_NAME")
            self.bundle_id = const("BUNDLE_ID")
            self.cli, self.db, self.ide = const("CLI_NAME"), const("DB_CLI_NAME"), const("IDE_NAME")
            self.slug = const("PROFILE_DIR")
            self.icon = "oxshift-python"
            self._binaries = [self.ide, self.db, self.cli]
        else:
            text = (ROOT / "crates/desktop/oxshift-platform/src/brand.rs").read_text(encoding="utf-8")
            def field(name):
                found = re.search(rf'^    {name}: "([^"]+)"', text, re.M)
                if not found:
                    fail(f"missing platform identity field {name}")
                return found.group(1)
            self.product, self.full_name = field("name"), field("full_name")
            self.bundle_id, self.slug = field("bundle_id"), field("slug")
            self.ide = self.slug
            self.cli = self.db = ""
            self.icon = "oxshift"
            self._binaries = [self.ide]

    @property
    def app(self):
        return f"{self.product}.app"

    @property
    def binaries(self):
        return self._binaries

    def table(self):
        return {"PRODUCT_NAME": self.product, "PRODUCT_FULL_NAME": self.full_name,
                "SLUG": self.slug, "CLASS": "".join(p.capitalize() for p in self.slug.split("-")),
                "BUNDLE_ID": self.bundle_id, "CLI": self.cli, "DB_CLI": self.db,
                "IDE": self.ide, "APP": self.app, "BINARIES": " ".join(self.binaries),
                "REPOSITORY": self.repository, "REPOSITORY_OWNER": self.repository.split("/")[0]}


def release_version(value):
    if not re.fullmatch(r"20[0-9]{2}\.[1-9][0-9]*(?:\.[0-9]+)?", value):
        fail(f"not a calendar release version: {value!r}")
    return value[:-2] if value.endswith(".0") else value


def requested_version(names, value):
    version = release_version(value or names.version)
    if version != names.version:
        fail(f"release {version} does not match workspace {names.version}")
    return version


def host_os():
    if sys.platform == "darwin":
        return "macos"
    if sys.platform == "win32":
        return "windows"
    if sys.platform.startswith("linux"):
        return "linux"
    fail(f"no release for {sys.platform}")


def host_arch(override):
    machine = (override or platform.machine()).lower()
    if machine not in ARCHES:
        fail(f"no release for the CPU {machine!r}")
    return ARCHES[machine]


def archive_name(names, version, os_name, arch):
    suffix = "zip" if os_name == "windows" else "tar.gz"
    return f"{names.slug}-{version}-{os_name}-{arch}.{suffix}"


def sha256_of(path):
    digest = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


# ---------------------------------------------------------------- build


def copy_binaries(names, bin_dir, dest, os_name):
    suffix = ".exe" if os_name == "windows" else ""
    dest.mkdir(parents=True, exist_ok=True)
    for binary in names.binaries:
        source = bin_dir / f"{binary}{suffix}"
        if not source.is_file():
            fail(f"{source} is missing; build with `cargo build --release` first")
        shutil.copy2(source, dest / source.name)


def put_debugpy(dest, given):
    """The debugger helper (ADR 0015), unmodified: copied from a folder that
    holds one, else fetched and checked by scripts/fetch-debugpy.py."""
    if given:
        for marker in ("debugpy/adapter/__main__.py", "debugpy-1.8.21.dist-info/RECORD",
                       "debugpy-1.8.21.dist-info/licenses/LICENSE"):
            if not (given / marker).is_file() or not (given / marker).stat().st_size:
                fail(f"{given} is not a debugpy helper folder ({marker} is missing)")
        # A folder that was used has bytecode in it, which is not the wheel's.
        shutil.copytree(given, dest, ignore=shutil.ignore_patterns("__pycache__"))
    else:
        run([python(), str(ROOT / "scripts" / "fetch-debugpy.py"), "--dest", str(dest)])
    if not (dest / "debugpy" / "ThirdPartyNotices.txt").is_file() or not (dest / "debugpy" / "ThirdPartyNotices.txt").stat().st_size:
        fail("the debugpy helper has no debugpy/ThirdPartyNotices.txt; its notices must ship")


def put_notices(folder):
    """NOTICE (the complete third-party text), the licences, the fonts' licences."""
    folder.mkdir(parents=True, exist_ok=True)
    run([python(), str(ROOT / "scripts/runtime-notices.py"), "check"], stdout=subprocess.DEVNULL)
    shutil.copy2(PACKAGING / "runtime/THIRD-PARTY-LICENSES.txt", folder / "THIRD-PARTY-LICENSES.txt")
    run([python(), str(ROOT / "scripts" / "notice.py"), "--out", str(folder / "NOTICE.txt")],
        stdout=subprocess.DEVNULL)
    for name in ("LICENSE-MIT", "LICENSE-APACHE"):
        shutil.copy2(ROOT / name, folder / name)
    licenses = folder / "licenses"
    licenses.mkdir(exist_ok=True)
    for license_file in sorted((PACKAGING / "licenses").glob("*.txt")):
        shutil.copy2(license_file, licenses / license_file.name)
    font_licenses = sorted(FONT_LICENSES.glob("LICENSE-*.txt"))
    if not font_licenses:
        fail("font license texts are missing")
    for license_file in font_licenses:
        if not license_file.stat().st_size:
            fail(f"empty font license: {license_file}")
        shutil.copy2(license_file, licenses / license_file.name)


def verify_runtime_notices(resources):
    expected = PACKAGING / "runtime/THIRD-PARTY-LICENSES.txt"
    actual = resources / "THIRD-PARTY-LICENSES.txt"
    if not actual.is_file() or actual.read_bytes() != expected.read_bytes():
        fail("runtime third-party license bytes do not match the reviewed bundle")


def put_conpty(dest, given, arch):
    """conpty.dll and OpenConsole.exe next to the executable (ADR 0014)."""
    if arch != "x86_64":
        fail("ConPTY files are pinned for x86_64 only")
    if given:
        for name in ("conpty.dll", "OpenConsole.exe"):
            if not (given / name).is_file():
                fail(f"{given / name} is missing")
            shutil.copy2(given / name, dest / name)
    else:
        run([python(), str(ROOT / "scripts" / "fetch-conpty.py"), "--dest", str(dest)])


def codesign(path, identifier, identity, keychain):
    command = ["codesign", "--force", "--sign", identity, "--identifier", identifier]
    if identity == "-":
        # Ad hoc: the default requirement is the hash of this build's code.
        command += [f"-r=designated => identifier \"{identifier}\""]
    elif keychain:
        command += ["--keychain", str(keychain)]
    run(command + [str(path)], stderr=subprocess.DEVNULL)


def sign_bundle(names, app, identity, keychain):
    macos = app / "Contents" / "MacOS"
    # Every binary on its own, with a fixed identifier (the name of the
    # program, which keychain access lists name), then the bundle.
    for binary in names.binaries[1:]:
        codesign(macos / binary, binary, identity, keychain)
    codesign(app, names.bundle_id, identity, keychain)
    run(["codesign", "--verify", "--strict", "--deep", str(app)])
    for target in [app, *(macos / b for b in names.binaries[1:])]:
        shown = subprocess.run(["codesign", "--display", "-r-", str(target)],
                               capture_output=True, text=True)
        requirement = next((line for line in (shown.stdout + shown.stderr).splitlines()
                            if line.startswith("designated")), "?")
        print(f"signed {target.relative_to(app.parent)}: {requirement}")


def make_app(names, version, bin_dir, root, args):
    app = root / names.app
    contents = app / "Contents"
    macos = contents / "MacOS"
    copy_binaries(names, bin_dir, macos, "macos")
    resources = contents / "Resources"
    if names.kind == "python":
        put_debugpy(resources / "helpers" / "debugpy", args.debugpy)
    put_notices(resources)
    info = {
        "CFBundleName": names.product,
        "CFBundleDisplayName": names.product,
        "CFBundleIdentifier": names.bundle_id,
        "CFBundleExecutable": names.ide,
        "CFBundlePackageType": "APPL",
        "CFBundleVersion": names.package_version,
        "CFBundleShortVersionString": version,
        "LSMinimumSystemVersion": MACOS_MINIMUM,
        "NSHighResolutionCapable": True,
        "LSApplicationCategoryType": "public.app-category.developer-tools",
    }
    info.update(compile_icons(ROOT, resources, names.kind, root, MACOS_MINIMUM))
    with open(contents / "Info.plist", "wb") as fh:
        plistlib.dump(info, fh)
    sign_bundle(names, app, args.sign_identity, args.sign_keychain)
    return app


def make_folder(names, bin_dir, root, os_name, arch, args):
    copy_binaries(names, bin_dir, root, os_name)
    if names.kind == "python":
        put_debugpy(root / "helpers" / "debugpy", args.debugpy)
    put_notices(root)
    shutil.copy2(ROOT / "assets/brand" / (names.icon + (".ico" if os_name == "windows" else ".png")), root)
    if os_name == "windows":
        put_conpty(root, args.conpty, arch)
    return root


def tar_filter(info):
    info.uid = info.gid = 0
    info.uname = info.gname = ""
    return info


def write_archive(root, top_name, archive):
    """The folder `root` as `archive`, its entries under `top_name`."""
    entries = sorted(root.rglob("*"))
    if archive.name.endswith(".zip"):
        with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as zf:
            for path in entries:
                if path.is_file():
                    zf.write(path, f"{top_name}/{path.relative_to(root).as_posix()}")
        return
    with tarfile.open(archive, "w:gz") as tf:
        tf.add(root, arcname=top_name, recursive=False, filter=tar_filter)
        for path in entries:
            tf.add(path, arcname=f"{top_name}/{path.relative_to(root).as_posix()}",
                   recursive=False, filter=tar_filter)


def checksum(path):
    path.with_name(path.name + ".sha256").write_text(
        f"{sha256_of(path)}  {path.name}\n", encoding="utf-8")


def make_dmg(names, app, out):
    with tempfile.TemporaryDirectory(prefix="dmg-") as temporary:
        stage = Path(temporary)
        shutil.copytree(app, stage / app.name, symlinks=True)
        (stage / "Applications").symlink_to("/Applications")
        run(["hdiutil", "create", "-volname", names.product, "-srcfolder", str(stage),
             "-format", "UDZO", str(out)])


def desktop_entry(names):
    return ("[Desktop Entry]\nType=Application\n" + f"Name={names.product}\n"
            + f"Exec={names.ide} %F\nIcon={names.slug}\n"
            + "Terminal=false\nCategories=Development;IDE;\n")


def make_deb(names, version, folder, out, arch):
    with tempfile.TemporaryDirectory(prefix="deb-") as temporary:
        root = Path(temporary)
        root.chmod(0o755)
        installed = root / "opt" / names.slug
        shutil.copytree(folder, installed)
        control = root / "DEBIAN"
        control.mkdir()
        dependencies = ("libc6 (>= 2.39), libgcc-s1, libstdc++6, libfontconfig1, "
                        "libfreetype6, libx11-6, libx11-xcb1, libxcb1, libxkbcommon0, "
                        "libxkbcommon-x11-0, libwayland-client0, libvulkan1")
        (control / "control").write_text(
            f"Package: {names.slug}\nVersion: {version}\n"
            f"Architecture: {'amd64' if arch == 'x86_64' else 'arm64'}\n"
            f"Maintainer: Oxshift contributors\nDescription: {names.full_name}\n"
            f"Section: devel\nPriority: optional\nDepends: {dependencies}\n", encoding="utf-8")
        launchers = root / "usr/bin"
        launchers.mkdir(parents=True)
        for binary in names.binaries:
            launcher = launchers / binary
            launcher.write_text(f'#!/bin/sh\nexec /opt/{names.slug}/{binary} "$@"\n', encoding="utf-8")
            launcher.chmod(0o755)
        applications = root / "usr/share/applications"
        applications.mkdir(parents=True)
        (applications / (names.slug + ".desktop")).write_text(desktop_entry(names), encoding="utf-8")
        icons = root / "usr/share/icons/hicolor/512x512/apps"
        icons.mkdir(parents=True)
        shutil.copy2(ROOT / "assets/brand" / (names.icon + ".png"), icons / (names.slug + ".png"))
        run(["dpkg-deb", "--root-owner-group", "--build", str(root), str(out)])


def make_appimage(names, folder, out, arch, tool, runtime):
    if not tool or not runtime:
        fail("AppImage needs --appimagetool and --appimage-runtime (verified native files)")
    with tempfile.TemporaryDirectory(prefix="appimage-") as temporary:
        appdir = Path(temporary) / (names.slug + ".AppDir")
        shutil.copytree(folder, appdir / "usr/bin")
        (appdir / (names.slug + ".desktop")).write_text(desktop_entry(names), encoding="utf-8")
        shutil.copy2(ROOT / "assets/brand" / (names.icon + ".png"), appdir / (names.slug + ".png"))
        launcher = appdir / "AppRun"
        launcher.write_text('#!/bin/sh\nHERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)\n'
                            f'exec "$HERE/usr/bin/{names.ide}" "$@"\n', encoding="utf-8")
        launcher.chmod(0o755)
        env = {**os.environ, "ARCH": arch}
        run([str(tool), "--appimage-extract-and-run", "--runtime-file", str(runtime),
             "--no-appstream", str(appdir), str(out)], env=env)


def make_installer(names, version, folder, out, iscc):
    if not iscc:
        fail("Windows installer needs --iscc (Inno Setup 6 compiler)")
    with tempfile.TemporaryDirectory(prefix="installer-") as temporary:
        script = Path(temporary) / "installer.iss"
        table = {**names.table(), "VERSION": version, "SOURCE": str(folder),
                 "OUTPUT": str(out.parent), "OUTPUT_NAME": out.stem,
                 "ICON": str(folder / (names.icon + ".ico")), "ICON_FILE": names.icon + ".ico"}
        script.write_text(render(PACKAGING / "windows/installer.iss.in", table), encoding="utf-8")
        run([str(iscc), str(script)])


def cmd_build(args):
    os_name, arch = host_os(), host_arch(args.arch)
    if os_name == "windows" and arch != "x86_64":
        fail("Windows ARM64 is not supported by the pinned ConPTY payload")
    formats = args.formats.split(",") if args.formats else {
        "macos": ["tar.gz", "dmg"], "linux": ["tar.gz"], "windows": ["zip"]}[os_name]
    allowed = {"macos": {"tar.gz", "dmg"}, "linux": {"tar.gz", "deb", "AppImage"},
               "windows": {"zip", "installer"}}[os_name]
    if not set(formats) <= allowed or len(set(formats)) != len(formats):
        fail(f"unsupported or repeated formats for {os_name}: {formats}")
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    for product in (["platform", "python"] if args.product == "all" else [args.product]):
        names = Names(product)
        version = requested_version(names, args.version)
        stem = f"{names.slug}-{version}-{os_name}-{arch}"
        with tempfile.TemporaryDirectory(prefix="package-") as temporary:
            stage = Path(temporary)
            if os_name == "macos":
                payload = make_app(names, version, args.bin_dir.resolve(), stage, args)
            else:
                payload = make_folder(names, args.bin_dir.resolve(), stage / stem, os_name, arch, args)
            for fmt in formats:
                suffix = "setup.exe" if fmt == "installer" else fmt
                artifact = out / f"{stem}.{suffix}"
                if artifact.exists():
                    fail(f"refusing to overwrite {artifact}")
                if fmt == "dmg":
                    make_dmg(names, payload, artifact)
                elif fmt == "deb":
                    make_deb(names, version, payload, artifact, arch)
                elif fmt == "AppImage":
                    make_appimage(names, payload, artifact, arch, args.appimagetool, args.appimage_runtime)
                elif fmt == "installer":
                    # Only the archive has the portable data marker, never the installer.
                    make_installer(names, version, payload, artifact, args.iscc)
                else:
                    if fmt == "zip":
                        data = payload / "data"
                        data.mkdir()
                        (data / ".oxshift-portable").write_text("", encoding="utf-8")
                        (data / "config").mkdir()
                        (data / "config/secrets.toml").write_text('store = "memory"\n', encoding="utf-8")
                        (data / "README.txt").write_text(
                            "Portable profile marker. See release notes for storage boundaries.\n", encoding="utf-8")
                    write_archive(payload, names.app if os_name == "macos" else stem, artifact)
                    if fmt == "zip":
                        shutil.rmtree(payload / "data")
                checksum(artifact)
                print(artifact)
    return 0


# ---------------------------------------------------------------- rendering


def render(template, table):
    text = template.read_text(encoding="utf-8")
    for key, value in table.items():
        text = text.replace(f"@{key}@", value)
    left = re.findall(r"@[A-Z0-9_]+@", text)
    if left:
        fail(f"{template.name}: no value for {sorted(set(left))}")
    return text


def read_sums(path):
    sums = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        found = re.fullmatch(r"([0-9a-f]{64}) [ *](\S+)", line.strip())
        if found:
            sums[found.group(2)] = found.group(1)
    return sums


def cmd_formula(args):
    names = Names(args.product)
    sums = read_sums(args.sums)
    table = names.table()
    table["VERSION"] = requested_version(names, args.version)
    for key, os_name, arch in (
        ("SHA_MACOS_AARCH64", "macos", "aarch64"),
        ("SHA_MACOS_X86_64", "macos", "x86_64"),
        ("SHA_LINUX_X86_64", "linux", "x86_64"),
        ("SHA_LINUX_AARCH64", "linux", "aarch64"),
    ):
        name = archive_name(names, requested_version(names, args.version), os_name, arch)
        if name not in sums:
            fail(f"{args.sums} has no checksum for {name}")
        table[key] = sums[name]
    args.out.write_text(render(FORMULA_TEMPLATE, table), encoding="utf-8")
    print(f"{args.out}: formula for {args.version}")
    return 0


def cmd_notes(args):
    names = Names(args.product)
    table = names.table()
    table["VERSION"] = requested_version(names, args.version)
    table["TAG"] = "v" + table["VERSION"]
    table["PRODUCT_SUMMARY"] = (
        "The standalone desktop includes the editor, project tree, Git, terminal, "
        "Shell run configurations and shared Python and SQL syntax highlighting."
        if names.kind == "platform" else
        "The Python IDE adds interpreter selection, Python code intelligence, "
        "run and debug configurations, test execution and database tools."
    )
    args.out.write_text(render(NOTES_TEMPLATE, table), encoding="utf-8")
    print(f"{args.out}: release notes header for {args.version}")
    return 0


def cmd_verify(args):
    """Inspect an archive without starting a GUI or modifying a user profile."""
    names = Names(args.product)
    archive = args.archive.resolve()
    os_name = args.os or host_os()
    suffix = ".exe" if os_name == "windows" else ""
    sidecar = archive.with_name(archive.name + ".sha256")
    if not sidecar.is_file() or read_sums(sidecar).get(archive.name) != sha256_of(archive):
        fail(f"{archive.name}: missing or mismatched checksum")
    with tempfile.TemporaryDirectory(prefix="verify-") as temporary:
        temporary = Path(temporary)
        if archive.suffix == ".zip":
            with zipfile.ZipFile(archive) as zf:
                for member in zf.namelist():
                    if "\\" in member or ":" in member or Path(member).is_absolute() or ".." in Path(member).parts:
                        fail(f"unsafe archive member: {member}")
                zf.extractall(temporary)
        else:
            with tarfile.open(archive) as tf:
                for member in tf.getmembers():
                    if (not (member.isfile() or member.isdir()) or "\\" in member.name
                            or ":" in member.name or Path(member.name).is_absolute()
                            or ".." in Path(member.name).parts):
                        fail(f"unsafe archive member: {member.name}")
                tf.extractall(temporary)
        tops = list(temporary.iterdir())
        if len(tops) != 1:
            fail("expected one top-level payload")
        top = tops[0]
        programs = top / "Contents/MacOS" if os_name == "macos" else top
        resources = top / "Contents/Resources" if os_name == "macos" else top
        verify_runtime_notices(resources)
        wanted = [programs / (binary + suffix) for binary in names.binaries]
        wanted += [resources / name for name in ("NOTICE.txt", "LICENSE-MIT", "LICENSE-APACHE")]
        if names.kind == "python":
            wanted += [resources / "helpers/debugpy" / name for name in
                       ("debugpy/adapter/__main__.py", "debugpy/ThirdPartyNotices.txt")]
        elif (resources / "helpers/debugpy").exists():
            fail("standalone payload must not ship Python tooling")
        if os_name == "windows":
            wanted += [top / name for name in ("conpty.dll", "OpenConsole.exe", "data/README.txt", "data/.oxshift-portable", "data/config/secrets.toml")]
        if any(not path.is_file() for path in wanted):
            fail(f"missing payload files: {[str(p.relative_to(top)) for p in wanted if not p.is_file()]}")
        if list(resources.rglob("__pycache__")):
            fail("bytecode cache in payload")
        if os_name == "macos":
            info = plistlib.loads((top / "Contents/Info.plist").read_bytes())
            validate_icons(resources, names.kind, info)
            if info["CFBundleIdentifier"] != names.bundle_id or info["CFBundleShortVersionString"] != requested_version(names, args.version):
                fail("bundle identity/version mismatch")
            if host_os() == "macos":
                run(["codesign", "--verify", "--strict", "--deep", str(top)])
        if args.smoke_cli:
            if not names.cli or os_name != host_os():
                fail("--smoke-cli requires the native Python product archive")
            shown = run([str(programs / (names.cli + suffix)), "--version"], capture_output=True)
            if requested_version(names, args.version) not in shown.stdout:
                fail("CLI version mismatch")
    print(f"{archive.name}: checksum and payload verified")
    return 0


def cmd_verify_dmg(args):
    names = Names(args.product)
    if host_os() != "macos":
        fail("DMG bundle verification requires macOS")
    digest = read_sums(args.archive.with_name(args.archive.name + ".sha256")).get(args.archive.name)
    if digest != sha256_of(args.archive):
        fail("DMG checksum mismatch")
    run(["hdiutil", "verify", str(args.archive)])
    with tempfile.TemporaryDirectory(prefix="verify-dmg-") as temporary:
        mount = Path(temporary) / "mounted"
        mount.mkdir()
        run(["hdiutil", "attach", "-readonly", "-nobrowse", "-mountpoint", str(mount), str(args.archive)])
        try:
            bundle = mount / names.app
            resources = bundle / "Contents/Resources"
            verify_runtime_notices(resources)
            info = plistlib.loads((bundle / "Contents/Info.plist").read_bytes())
            validate_icons(resources, names.kind, info)
            if (info["CFBundleIdentifier"] != names.bundle_id
                    or info["CFBundleExecutable"] != names.ide
                    or info["CFBundleShortVersionString"] != requested_version(names, args.version)
                    or info["CFBundleVersion"] != names.package_version
                    or info.get("LSMinimumSystemVersion") != MACOS_MINIMUM):
                fail("DMG application identity/version/minimum OS mismatch")
            for binary in names.binaries:
                if not (bundle / "Contents/MacOS" / binary).is_file():
                    fail(f"DMG missing {binary}")
            for name in ("NOTICE.txt", "LICENSE-MIT", "LICENSE-APACHE"):
                if not (resources / name).is_file():
                    fail(f"DMG missing {name}")
            if names.kind == "python" and not (resources / "helpers/debugpy/debugpy/adapter/__main__.py").is_file():
                fail("DMG missing Python debugger helper")
            run(["codesign", "--verify", "--strict", "--deep", str(bundle)])
            print(f"{args.archive.name}: signed app identity, Icon Composer assets and payload verified")
        finally:
            run(["hdiutil", "detach", str(mount)])
    return 0


def cmd_names(args):
    names = Names(args.product)
    for key, value in {**names.table(), "VERSION": names.version}.items():
        print(f"{key}={value}")
    return 0


def cmd_installer_script(args):
    names = Names(args.product)
    args.out.write_text(render(INSTALL_SCRIPT, names.table()), encoding="utf-8")
    return 0


def cmd_check(args):
    run([python(), str(ROOT / "scripts/runtime-notices.py"), "check"])
    for product in ("platform", "python"):
        names = Names(product)
        for suffix in (".icns", ".ico", ".png"):
            if not (ROOT / "assets/brand" / (names.icon + suffix)).is_file():
                fail(f"missing {product} icon {suffix}")
        print(f"{product}: {names.product}, {names.bundle_id}, {names.version}, {names.binaries}")
    print("packaging: product identities and artwork available")
    return 0


def cmd_sums(args):
    files = sorted(p for p in args.directory.iterdir() if p.is_file()
                   and p.name != "SHA256SUMS" and not p.name.endswith(".sha256"))
    if not files:
        fail("no release artifacts")
    (args.directory / "SHA256SUMS").write_text(
        "".join(f"{sha256_of(path)}  {path.name}\n" for path in files), encoding="utf-8")
    return 0


def main():
    parser = argparse.ArgumentParser(description="Build local release artifacts; never publishes or tags.")
    sub = parser.add_subparsers(dest="command", required=True)
    build = sub.add_parser("build")
    build.add_argument("--bin-dir", type=Path, default=ROOT / "target/release")
    build.add_argument("--out", type=Path, required=True)
    build.add_argument("--product", choices=["all", "platform", "python"], default="all")
    build.add_argument("--version")
    build.add_argument("--arch")
    build.add_argument("--formats", help="comma-separated native formats: tar.gz,dmg / zip,installer / tar.gz,deb,AppImage")
    build.add_argument("--debugpy", type=Path)
    build.add_argument("--conpty", type=Path)
    build.add_argument("--sign-identity", default="-")
    build.add_argument("--sign-keychain", type=Path)
    build.add_argument("--iscc", type=Path)
    build.add_argument("--appimagetool", type=Path)
    build.add_argument("--appimage-runtime", type=Path)
    build.set_defaults(run=cmd_build)
    for command, function in (("formula", cmd_formula), ("notes", cmd_notes), ("install-script", cmd_installer_script)):
        item = sub.add_parser(command)
        item.add_argument("--product", choices=["platform", "python"], default="python")
        item.add_argument("--version")
        item.add_argument("--out", type=Path, required=True)
        if command == "formula":
            item.add_argument("--sums", type=Path, required=True)
        item.set_defaults(run=function)
    verify = sub.add_parser("verify")
    verify.add_argument("archive", type=Path)
    verify.add_argument("--product", choices=["platform", "python"], required=True)
    verify.add_argument("--os", choices=["macos", "linux", "windows"])
    verify.add_argument("--version")
    verify.add_argument("--smoke-cli", action="store_true")
    verify.set_defaults(run=cmd_verify)
    dmg = sub.add_parser("verify-dmg")
    dmg.add_argument("archive", type=Path)
    dmg.add_argument("--product", choices=["platform", "python"], required=True)
    dmg.add_argument("--version")
    dmg.set_defaults(run=cmd_verify_dmg)
    names = sub.add_parser("names")
    names.add_argument("--product", choices=["platform", "python"], default="python")
    names.set_defaults(run=cmd_names)
    sub.add_parser("check").set_defaults(run=cmd_check)
    sums = sub.add_parser("sums")
    sums.add_argument("directory", type=Path)
    sums.set_defaults(run=cmd_sums)
    args = parser.parse_args()
    return args.run(args)


if __name__ == "__main__":
    sys.exit(main())
