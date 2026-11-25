import importlib.util
import json
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch
import zipfile

SPEC = importlib.util.spec_from_file_location('smoke', Path(__file__).with_name('smoke-windows-desktop.py'))
smoke = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(smoke)


def align(data):
    data.extend(b'\0' * (-len(data) % 4))


def version_block(key, value=b'', children=(), text=False):
    result = bytearray(struct.pack('<HHH', 0, len(value) // 2 if text else len(value), int(text)))
    result.extend((key + '\0').encode('utf-16le'))
    align(result)
    result.extend(value)
    for child in children:
        align(result)
        result.extend(child)
    struct.pack_into('<H', result, 0, len(result))
    return bytes(result)


def fixture_resources(icon, description='Oxshift', version='2026.1'):
    strings = [version_block(key, (value + '\0').encode('utf-16le'), text=True)
               for key, value in [('FileDescription', description), ('ProductVersion', version)]]
    version = version_block('VS_VERSION_INFO', b'\0' * 52, [version_block('StringFileInfo', children=[version_block('040904B0', children=strings)])])
    count, = struct.unpack_from('<H', icon, 4)
    resources = {(16, 1, 1033): version}
    group = bytearray(icon[:6])
    for index in range(count):
        entry = icon[6 + index * 16:22 + index * 16]
        length, offset = struct.unpack_from('<II', entry, 8)
        identity = index + 20
        group.extend(entry[:12] + struct.pack('<H', identity))
        resources[(3, identity, 1033)] = icon[offset:offset + length]
    resources[(14, 1, 1033)] = bytes(group)
    return resources


def make_pe(resources):
    tree = {}
    for (kind, identity, language), payload in resources.items():
        tree.setdefault(kind, {}).setdefault(identity, {})[language] = payload
    section = bytearray()

    def directory(values):
        align(section)
        at = len(section)
        section.extend(b'\0' * (16 + len(values) * 8))
        struct.pack_into('<HH', section, at + 12, 0, len(values))
        for index, (identity, value) in enumerate(sorted(values.items())):
            if isinstance(value, dict):
                target = directory(value) | 0x80000000
            else:
                align(section)
                target = len(section)
                section.extend(struct.pack('<IIII', 0x1000 + target + 16, len(value), 0, 0))
                section.extend(value)
            struct.pack_into('<II', section, at + 16 + index * 8, identity, target)
        return at
    directory(tree)
    pe = bytearray(512)
    pe[:2] = b'MZ'
    struct.pack_into('<I', pe, 60, 128)
    pe[128:132] = b'PE\0\0'
    struct.pack_into('<HH', pe, 132, 0x8664, 1)
    struct.pack_into('<H', pe, 148, 240)
    struct.pack_into('<H', pe, 152, 0x20b)
    struct.pack_into('<II', pe, 152 + 112 + 16, 0x1000, len(section))
    pe[392:400] = b'.rsrc\0\0\0'
    struct.pack_into('<IIII', pe, 400, len(section), 0x1000, len(section), 512)
    return bytes(pe + section)


class WindowsSmokeFixtures(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.icon = smoke.ROOT / 'assets/brand/oxshift.ico'
        self.resources = fixture_resources(self.icon.read_bytes())
        self.exe = self.root / 'oxshift-platform.exe'

    def verify(self):
        self.exe.write_bytes(make_pe(self.resources))
        return smoke.verify_executable(self.exe, self.icon, 'Oxshift', '2026.1')

    def test_real_approved_ico_payloads_and_sizes_are_extracted_from_x64_fixture(self):
        result = self.verify()
        self.assertEqual(result['status'], 'passed')
        self.assertIn([256, 256], result['icon_dimensions'])
        self.assertEqual(result['group_icon_id'], 1)

    def test_python_product_uses_its_distinct_approved_icon(self):
        icon = smoke.ROOT / 'assets/brand/oxshift-python.ico'
        self.exe.write_bytes(make_pe(fixture_resources(icon.read_bytes(), description='Oxshift Python')))
        result = smoke.verify_executable(self.exe, icon, 'Oxshift Python', '2026.1')
        self.assertEqual(result['description'], 'Oxshift Python')
        with self.assertRaisesRegex(ValueError, 'icon'):
            smoke.verify_executable(self.exe, self.icon, 'Oxshift Python', '2026.1')

    def test_product_description_mismatch_fails(self):
        self.resources = fixture_resources(self.icon.read_bytes(), description='Wrong IDE')
        with self.assertRaisesRegex(ValueError, 'FileDescription'):
            self.verify()

    def test_market_version_mismatch_fails(self):
        self.resources = fixture_resources(self.icon.read_bytes(), version='2026.1.0')
        with self.assertRaisesRegex(ValueError, 'ProductVersion'):
            self.verify()

    def test_group_id_one_is_required(self):
        self.resources[(14, 2, 1033)] = self.resources.pop((14, 1, 1033))
        with self.assertRaisesRegex(ValueError, 'ID 1'):
            self.verify()

    def test_changed_embedded_image_fails_even_when_dimensions_match(self):
        key = next(key for key in self.resources if key[0] == 3)
        original = self.resources[key]
        self.resources[key] = bytes([original[0] ^ 1]) + original[1:]
        with self.assertRaisesRegex(ValueError, 'icon data'):
            self.verify()

    def test_truncated_pe_and_non_x64_fail(self):
        data = bytearray(make_pe(self.resources))
        with self.assertRaises(ValueError):
            smoke.pe_resources(data[:-20])
        struct.pack_into('<H', data, 132, 0x14c)
        with self.assertRaisesRegex(ValueError, 'x64'):
            smoke.pe_resources(data)

    def test_portable_package_static_check_is_cross_platform(self):
        self.exe.write_bytes(make_pe(self.resources))
        (self.root / 'oxshift.ico').write_bytes(self.icon.read_bytes())
        (self.root / 'data/config').mkdir(parents=True)
        (self.root / 'data/.oxshift-portable').touch()
        (self.root / 'data/config/secrets.toml').write_text('store = "memory"\n')
        result = smoke.inspect_package(self.root, '2026.1', 'skip')
        self.assertEqual(result['static']['status'], 'passed')
        self.assertEqual(result['runtime']['status'], 'not-run')
        (self.root / 'data/.oxshift-portable').unlink()
        with self.assertRaisesRegex(ValueError, 'marker'):
            smoke.inspect_package(self.root, '2026.1', 'skip')

    def test_portable_runtime_reads_reports_from_data_not_config(self):
        class Process:
            returncode = 0

            def communicate(inner, timeout):
                return (b'OXSHIFT run help.local-bug-report-state {"status":"partial"}\n'
                        b'OXSHIFT run terminal.state {"started":true,"lines":["OXSHIFT_TERMINAL_SMOKE"]}\n', None)

        def launch(args, **kwargs):
            data = Path(args[0]).parent / 'data'
            config = data / 'config'
            config.mkdir(parents=True)
            (config / 'settings.toml').write_text('idle-seconds = 17\n')
            reports = data / 'bug-reports'
            reports.mkdir()
            for count in (1, 2):
                metadata = {
                    'version': '2026.1', 'window_count': count, 'status': 'partial',
                    'recent_command_ids': ['settings.open'],
                    'screenshots': [{'status': 'unavailable', 'reason': 'No capture device'}] * count,
                }
                with zipfile.ZipFile(reports / f'report-{count}.zip', 'w') as report:
                    report.writestr('report.json', json.dumps(metadata))
            return Process()

        with patch.object(smoke, 'desktop_available', return_value=True), \
             patch.object(smoke.subprocess, 'CREATE_NEW_PROCESS_GROUP', 512, create=True), \
             patch.object(smoke.subprocess, 'Popen', side_effect=launch):
            result = smoke.runtime_smoke(self.root, self.exe.name, '2026.1', 'auto')
        self.assertEqual(result['status'], 'partial')
        self.assertEqual(result['portable_settings'], 'passed')
        self.assertEqual(result['terminal'], 'passed')
        self.assertEqual([len(group) for group in result['report_captures']], [1, 2])

    def test_explicit_required_mode_never_succeeds_without_desktop(self):
        original = smoke.desktop_available
        smoke.desktop_available = lambda: False
        try:
            with self.assertRaisesRegex(ValueError, 'desktop'):
                smoke.runtime_smoke(self.root, self.exe.name, '2026.1', 'required')
            self.assertEqual(smoke.runtime_smoke(self.root, self.exe.name, '2026.1', 'auto')['status'], 'not-run')
        finally:
            smoke.desktop_available = original


if __name__ == '__main__':
    unittest.main()
