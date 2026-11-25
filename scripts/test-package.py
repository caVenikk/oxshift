#!/usr/bin/env python3
# Isolated packaging regressions: fake executables, no downloads or GUI launches.
import argparse
import contextlib
import importlib.util
import io
import os
from pathlib import Path
import plistlib
import re
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile

sys.dont_write_bytecode = True

spec = importlib.util.spec_from_file_location('release_package', Path(__file__).with_name('package.py'))
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class PackagingTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='oxshift-packaging-test-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.binaries = self.root / 'binaries'
        self.binaries.mkdir()
        for product in ('platform', 'python'):
            for binary in package.Names(product).binaries:
                for suffix in ('', '.exe'):
                    target = self.binaries / (binary + suffix)
                    target.write_bytes(b'fixture executable, never run\n')
                    target.chmod(0o755)
        self.helper = self.root / 'debugpy'
        for name in ('debugpy/adapter/__main__.py', 'debugpy/ThirdPartyNotices.txt',
                     'debugpy-1.8.21.dist-info/RECORD', 'debugpy-1.8.21.dist-info/licenses/LICENSE',
                     'debugpy/__pycache__/old.pyc'):
            path = self.helper / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text('fixture')
        self.conpty = self.root / 'conpty'
        self.conpty.mkdir()
        for name in ('conpty.dll', 'OpenConsole.exe'):
            (self.conpty / name).write_bytes(b'fixture')
        self.args = argparse.Namespace(product='all', version='2026.1', arch='x86_64',
            bin_dir=self.binaries, out=self.root / 'dist', formats='tar.gz',
            debugpy=self.helper, conpty=self.conpty, sign_identity='-', sign_keychain=None,
            iscc=Path('fixture-iscc'), appimagetool=Path('fixture-appimagetool'),
            appimage_runtime=Path('fixture-runtime'))

    def build(self, os_name, formats):
        self.args.formats = formats
        with patch.object(package, 'host_os', return_value=os_name):
            package.cmd_build(self.args)
        return sorted(self.args.out.iterdir())

    def verify(self, archive, product, os_name):
        args = argparse.Namespace(archive=archive, product=product, os=os_name,
                                  version='2026.1', smoke_cli=False)
        with patch.object(package, 'host_os', return_value='linux'):
            self.assertEqual(package.cmd_verify(args), 0)

    def test_local_and_release_use_same_icon_composer_function(self):
        spec = importlib.util.spec_from_file_location('local_package', Path(__file__).with_name('package-macos.py'))
        local = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(local)
        self.assertIs(local.compile_icons, package.compile_icons)
        resources = self.root / 'resources'
        resources.mkdir()
        (resources / 'Oxshift.icns').write_bytes(b'only-static-icon')
        with self.assertRaises(ValueError):
            package.validate_icons(resources, 'platform', {'CFBundleIconName': 'Oxshift', 'CFBundleIconFile': 'Oxshift.icns'})

    def test_runtime_license_bytes_are_required(self):
        resources = self.root / 'licenses-test'
        package.put_notices(resources)
        package.verify_runtime_notices(resources)
        bundle = resources / 'THIRD-PARTY-LICENSES.txt'
        self.assertIn(b'aws-lc-sys', bundle.read_bytes())
        self.assertIn(b'option-ext', bundle.read_bytes())
        bundle.write_bytes(bundle.read_bytes() + b'changed')
        with self.assertRaises(SystemExit):
            package.verify_runtime_notices(resources)
        bundle.unlink()
        with self.assertRaises(SystemExit):
            package.verify_runtime_notices(resources)

    def test_calendar_versions_and_distinct_products(self):
        platform, python = package.Names('platform'), package.Names('python')
        self.assertEqual(platform.version, '2026.1')
        self.assertEqual(python.package_version, '2026.1.0')
        self.assertNotEqual(platform.bundle_id, python.bundle_id)
        self.assertNotEqual(platform.icon, python.icon)
        with self.assertRaises(SystemExit):
            package.requested_version(platform, '2026.2')

    def test_linux_archives_helpers_notices_and_no_pycache(self):
        paths = self.build('linux', 'tar.gz')
        archives = [p for p in paths if p.name.endswith('.tar.gz')]
        self.assertEqual(len(archives), 2)
        for product in ('platform', 'python'):
            names = package.Names(product)
            archive = next(p for p in archives if p.name.startswith(names.slug + '-2026'))
            self.verify(archive, product, 'linux')
            with tarfile.open(archive) as tf:
                members = tf.getnames()
            self.assertEqual(any('/helpers/debugpy/' in name for name in members), product == 'python')
            self.assertFalse(any('__pycache__' in name for name in members))

    def test_windows_archive_has_data_but_installer_does_not(self):
        seen = []
        real_run = package.run
        def native(command, **kwargs):
            if str(command[0]) != 'fixture-iscc':
                return real_run(command, **kwargs)
            script = Path(command[1]).read_text()
            self.assertIn('PrivilegesRequiredOverridesAllowed=dialog', script)
            self.assertIn('PrivilegesRequired=lowest', script)
            source = Path(re.search(r'Source: "(.*?)\\\*"', script)[1])
            self.assertFalse((source / 'data').exists())
            output = Path(re.search(r'^OutputDir=(.+)$', script, re.M)[1])
            name = re.search(r'^OutputBaseFilename=(.+)$', script, re.M)[1]
            (output / (name + '.exe')).write_bytes(b'fixture installer')
            seen.append(script)
        with patch.object(package, 'run', side_effect=native):
            paths = self.build('windows', 'zip,installer')
        self.assertEqual(len(seen), 2)
        for product in ('platform', 'python'):
            archive = next(p for p in paths if p.name.startswith(package.Names(product).slug + '-2026') and p.suffix == '.zip')
            self.verify(archive, product, 'windows')
            with zipfile.ZipFile(archive) as zf:
                self.assertTrue(any(name.endswith('/data/.oxshift-portable') for name in zf.namelist()))
                secret = next(name for name in zf.namelist() if name.endswith('/data/config/secrets.toml'))
                self.assertEqual(zf.read(secret), b'store = "memory"\n')

    def test_macos_bundle_identity_icon_and_disk_image(self):
        real_run = package.run
        signed = []
        def native(command, **kwargs):
            if command[0] == 'codesign':
                signed.append(command)
                return subprocess.CompletedProcess(command, 0)
            if command[0] == 'hdiutil':
                stage = Path(command[command.index('-srcfolder') + 1])
                self.assertEqual((stage / 'Applications').readlink(), Path('/Applications'))
                self.assertEqual(len(list(stage.glob('*.app'))), 1)
                Path(command[-1]).write_bytes(b'fixture dmg')
                return subprocess.CompletedProcess(command, 0)
            return real_run(command, **kwargs)
        original = subprocess.run
        with patch.object(package, 'run', side_effect=native), patch.object(
                package.subprocess, 'run', wraps=subprocess.run) as process:
            # codesign's informational display call is also a native boundary.
            def process_call(command, **kw):
                if command[0] == 'codesign':
                    return subprocess.CompletedProcess(command, 0, '', '')
                if command[:2] == ['xcrun', 'actool']:
                    resources = Path(command[command.index('--compile') + 1])
                    name = command[command.index('--app-icon') + 1]
                    (resources / 'Assets.car').write_bytes(b'fixture car')
                    (resources / (name + '.icns')).write_bytes(b'fixture icon')
                    return subprocess.CompletedProcess(command, 0)
                return original(command, **kw)
            process.side_effect = process_call
            paths = self.build('macos', 'tar.gz,dmg')
        for product in ('platform', 'python'):
            names = package.Names(product)
            archive = next(p for p in paths if p.name.startswith(names.slug + '-2026') and p.name.endswith('.tar.gz'))
            self.verify(archive, product, 'macos')
            with tarfile.open(archive) as tf:
                info = plistlib.loads(tf.extractfile(names.app + '/Contents/Info.plist').read())
                self.assertEqual(info['CFBundleVersion'], '2026.1.0')
                self.assertEqual(info['CFBundleIconFile'], ('Oxshift' if product == 'platform' else 'OxshiftPython') + '.icns')
                self.assertEqual(info['CFBundleIconName'], 'Oxshift' if product == 'platform' else 'OxshiftPython')
                self.assertTrue(tf.getmember(names.app + '/Contents/Resources/Assets.car').size)
            self.assertTrue(any(names.bundle_id in command for command in signed))

    def test_native_linux_layouts_and_explicit_runtime(self):
        real_run = package.run
        kinds = []
        def native(command, **kwargs):
            if command[0] == 'dpkg-deb':
                root = Path(command[-2])
                self.assertIn('Architecture: amd64', (root / 'DEBIAN/control').read_text())
                self.assertTrue(list((root / 'usr/bin').iterdir()))
                kinds.append('deb')
            elif command[0] == 'fixture-appimagetool':
                self.assertIn('--runtime-file', command)
                self.assertIn('fixture-runtime', command)
                root = Path(command[-2])
                self.assertTrue((root / 'AppRun').is_file())
                self.assertTrue(list(root.glob('*.desktop')))
                kinds.append('AppImage')
            else:
                return real_run(command, **kwargs)
            Path(command[-1]).write_bytes(b'fixture native package')
        with patch.object(package, 'run', side_effect=native):
            self.build('linux', 'deb,AppImage')
        self.assertEqual(kinds, ['deb', 'AppImage', 'deb', 'AppImage'])

    def test_checksum_corruption_is_rejected(self):
        archive = next(p for p in self.build('linux', 'tar.gz') if p.name.endswith('.tar.gz'))
        archive.write_bytes(archive.read_bytes() + b'changed')
        with self.assertRaises(SystemExit):
            self.verify(archive, 'platform', 'linux')

    def test_unsafe_tar_link_is_rejected_before_extraction(self):
        archive = self.root / 'unsafe.tar.gz'
        with tarfile.open(archive, 'w:gz') as tf:
            member = tarfile.TarInfo('payload/escape')
            member.type = tarfile.SYMTYPE
            member.linkname = '../../outside'
            tf.addfile(member)
        package.checksum(archive)
        with self.assertRaises(SystemExit):
            self.verify(archive, 'platform', 'linux')
        self.assertFalse((self.root / 'outside').exists())

    def test_missing_binary_and_unsupported_arch_fail(self):
        (self.binaries / 'oxshift-platform').unlink()
        with self.assertRaises(SystemExit):
            self.build('linux', 'tar.gz')
        self.args.arch = 'aarch64'
        with self.assertRaises(SystemExit):
            self.build('windows', 'zip')

    def test_metadata_templates_for_both_products(self):
        sums = self.root / 'SHA256SUMS'
        sums.write_text(''.join('0' * 64 + '  ' + package.archive_name(package.Names(product), '2026.1', os_name, arch) + '\n'
                               for product in ('platform', 'python') for os_name in ('linux', 'macos') for arch in ('x86_64', 'aarch64')))
        for product in ('platform', 'python'):
            names = package.Names(product)
            args = argparse.Namespace(product=product, version='2026.1', sums=sums, out=self.root / 'metadata')
            for command in (package.cmd_formula, package.cmd_notes, package.cmd_installer_script):
                command(args)
                value = args.out.read_text()
                self.assertNotRegex(value, r'@[A-Z0-9_]+@')
                self.assertNotIn('pyoxide', value.lower())
            subprocess.run(['sh', '-n', str(args.out)], check=True)
            if product == 'platform':
                package.cmd_formula(args)
                self.assertIn('%w[oxshift-platform]', args.out.read_text())


@unittest.skipUnless(os.name == 'posix', 'shell installer targets macOS and Linux')
class ShellInstallerTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='installer-test-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.names = package.Names('platform')
        self.prefix = self.root / "apps with ' and $ dollars"
        self.bin = self.root / 'bin'
        self.bin.mkdir()
        self.script = self.root / 'install.sh'
        self.script.write_text(package.render(package.INSTALL_SCRIPT, self.names.table()))
        self.env = {**os.environ, 'OXSHIFT_VERSION': '2026.1', 'OXSHIFT_OS': 'linux',
                    'OXSHIFT_ARCH': 'x86_64', 'OXSHIFT_BASE_URL': self.root.as_uri()}
        self.stage = self.root / 'stage'
        self.stage.mkdir()
        self.binary = b'#!/bin/sh\nprintf "fixture output\\n"\n'
        executable = self.stage / self.names.ide
        executable.write_bytes(self.binary)
        executable.chmod(0o755)
        self.archive('linux')

    def archive(self, os_name):
        name = package.archive_name(self.names, '2026.1', os_name, 'x86_64')
        package.write_archive(self.stage, self.names.app if os_name == 'macos' else name[:-7], self.root / name)
        (self.root / 'SHA256SUMS').write_text(package.sha256_of(self.root / name) + '  ' + name + '\n')

    def run_installer(self, *args, success=True):
        result = subprocess.run(['sh', str(self.script), '--prefix', str(self.prefix),
                                 '--bin-dir', str(self.bin), *args], env=self.env,
                                capture_output=True, text=True)
        self.assertEqual(result.returncode == 0, success, result.stdout + result.stderr)
        return result

    def test_install_reinstall_symlink_and_literal_launcher_path(self):
        self.run_installer()
        installed = self.prefix / self.names.slug / self.names.ide
        launcher = self.bin / self.names.ide
        launcher.unlink()
        launcher.symlink_to(installed)
        self.run_installer()
        self.assertFalse(launcher.is_symlink())
        self.assertEqual(installed.read_bytes(), self.binary)
        self.assertEqual(subprocess.check_output([str(launcher)], text=True), 'fixture output\n')
        self.run_installer('--uninstall')
        self.assertFalse(installed.exists())
        self.assertFalse(launcher.exists())

    def test_foreign_destination_and_launcher_are_preserved(self):
        app = self.prefix / self.names.slug
        app.mkdir(parents=True)
        personal = app / 'keep.txt'
        personal.write_text('keep')
        self.run_installer(success=False)
        self.run_installer('--uninstall', success=False)
        self.assertEqual(personal.read_text(), 'keep')
        personal.unlink()
        app.rmdir()
        outside = self.root / 'foreign-program'
        outside.write_text('keep foreign program')
        launcher = self.bin / self.names.ide
        launcher.symlink_to(outside)
        self.run_installer(success=False)
        self.assertTrue(launcher.is_symlink())
        self.assertEqual(outside.read_text(), 'keep foreign program')

    def test_uninstall_preserves_replaced_launcher_even_with_matching_path(self):
        self.run_installer()
        launcher = self.bin / self.names.ide
        custom = '#!/bin/sh\n# custom: ' + str(self.prefix / self.names.slug) + '\n'
        launcher.write_text(custom)
        self.run_installer('--uninstall')
        self.assertEqual(launcher.read_text(), custom)

    def test_uninstall_preserves_foreign_symlink_and_rejects_unowned_app_link(self):
        self.run_installer()
        launcher = self.bin / self.names.ide
        outside = self.root / 'foreign-launcher'
        outside.write_text('keep')
        launcher.unlink()
        launcher.symlink_to(outside)
        self.run_installer('--uninstall')
        self.assertTrue(launcher.is_symlink())
        self.assertEqual(outside.read_text(), 'keep')
        app = self.prefix / self.names.slug
        other = self.root / 'foreign-directory'
        other.mkdir()
        app.symlink_to(other, target_is_directory=True)
        self.run_installer('--uninstall', success=False)
        self.assertTrue(app.is_symlink())
        self.assertTrue(other.is_dir())

    def test_macos_never_calls_xattr_and_keeps_receipt_outside_bundle(self):
        executable = self.stage / self.names.ide
        programs = self.stage / 'Contents/MacOS'
        programs.mkdir(parents=True)
        executable.rename(programs / self.names.ide)
        self.archive('macos')
        self.env['OXSHIFT_OS'] = 'macos'
        tools = self.root / 'tools'
        tools.mkdir()
        xattr = tools / 'xattr'
        called = self.root / 'xattr-called'
        xattr.write_text('#!/bin/sh\ntouch "' + str(called) + '"\nexit 1\n')
        xattr.chmod(0o755)
        self.env['PATH'] = str(tools) + os.pathsep + self.env['PATH']
        self.run_installer()
        self.assertFalse(called.exists())
        bundle = self.prefix / self.names.app
        self.assertEqual(sorted(str(p.relative_to(bundle)) for p in bundle.rglob('*')),
                         ['Contents', 'Contents/MacOS', 'Contents/MacOS/' + self.names.ide])
        self.run_installer('--uninstall')


class ReleaseGateTests(unittest.TestCase):
    def test_only_exact_full_main_push_accepts(self):
        spec = importlib.util.spec_from_file_location('preflight', Path(__file__).with_name('release-preflight.py'))
        gate = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(gate)
        good = dict(head_sha='abc', head_branch='main', event='push',
                    path='.github/workflows/ci.yml', head_repository={'full_name': 'owner/repo'},
                    run_number=1, run_attempt=1, status='completed', conclusion='success')
        self.assertEqual(gate.select_run([good], 'abc', 'owner/repo'), good)
        for field, value in [('event', 'pull_request'), ('head_sha', 'other'),
                             ('path', '.github/workflows/preview.yml'), ('conclusion', 'failure')]:
            with self.assertRaises(ValueError):
                gate.select_run([{**good, field: value}], 'abc', 'owner/repo')
        jobs = [dict(name='check (' + os + ')', conclusion='success',
                     steps=[dict(name='Record completed platform checks', conclusion='success')])
                for os in ['ubuntu-latest', 'macos-latest', 'windows-latest']]
        jobs += [dict(name='integration', conclusion='success',
                      steps=[dict(name='Record completed database checks', conclusion='success')]),
                 dict(name='CI required', conclusion='success',
                      steps=[dict(name='Validate exact results and live PR identity', conclusion='success')])]
        gate.validate_jobs(jobs)
        self.assertEqual(gate.accept_full_or_reuse(jobs, '', lambda: self.fail('full run must not need reuse')), 'full')
        reused = [jobs[-1]]
        called = []
        self.assertEqual(gate.accept_full_or_reuse(reused, '2026-11-25 CI required: reuse passed', lambda: called.append(True)), 'verified-main')
        self.assertEqual(called, [True])
        with self.assertRaises(ValueError):
            gate.accept_full_or_reuse(reused, 'CI required: draft passed', lambda: None)
        def invalid_proof():
            raise ValueError('proof tree differs')
        with self.assertRaises(ValueError):
            gate.accept_full_or_reuse(reused, 'CI required: reuse passed', invalid_proof)
        jobs[0]['steps'][0]['conclusion'] = 'skipped'
        with self.assertRaises(ValueError):
            gate.validate_jobs(jobs)


if __name__ == '__main__':
    with contextlib.redirect_stdout(io.StringIO()):
        unittest.main(verbosity=2)
