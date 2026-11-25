import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
PRODUCTS = {
    'oxshift-platform.exe': ('Oxshift', 'oxshift.ico'),
    'oxshift-python.exe': ('Oxshift Python', 'oxshift-python.ico'),
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def pe_resources(data):
    def read(fmt, at):
        return struct.unpack_from(fmt, data, at)

    require(data[:2] == b'MZ', 'missing DOS header')
    pe, = read('<I', 60)
    require(data[pe:pe + 4] == b'PE\0\0', 'missing PE signature')
    machine, count = read('<HH', pe + 4)
    optional_size, = read('<H', pe + 20)
    optional = pe + 24
    require(machine == 0x8664 and read('<H', optional)[0] == 0x20b, 'not Windows x64 PE32+')
    sections = []
    for index in range(count):
        size, va, raw_size, raw = read('<IIII', optional + optional_size + index * 40 + 8)
        sections.append((va, raw_size, raw))

    def offset(rva, length):
        for va, size, raw in sections:
            if va <= rva and rva + length <= va + size:
                require(raw + rva - va + length <= len(data), 'truncated PE section')
                return raw + rva - va
        raise ValueError('resource RVA outside file-backed sections')

    resource_rva, resource_size = read('<II', optional + 112 + 2 * 8)
    base = offset(resource_rva, resource_size)
    result = {}

    def walk(relative, keys):
        require(len(keys) < 3 and relative + 16 <= resource_size, 'invalid resource tree')
        named, ids = read('<HH', base + relative + 12)
        require(relative + 16 + (named + ids) * 8 <= resource_size, 'truncated resource directory')
        for index in range(named + ids):
            key, child = read('<II', base + relative + 16 + index * 8)
            if key & 0x80000000:
                continue
            path = keys + (key,)
            if child & 0x80000000:
                walk(child & 0x7fffffff, path)
            else:
                require(len(path) == 3 and child + 16 <= resource_size, 'invalid resource leaf')
                rva, length = read('<II', base + child)
                at = offset(rva, length)
                require(path not in result, 'duplicate resource identity')
                result[path] = data[at:at + length]
    walk(0, ())
    return result


def version_strings(blob):
    result = {}

    def block(at, limit):
        require(at + 6 <= limit, 'truncated VERSIONINFO')
        size, value_length, kind = struct.unpack_from('<HHH', blob, at)
        end = at + size
        require(size >= 6 and end <= limit, 'invalid VERSIONINFO block')
        cursor = at + 6
        while cursor + 2 <= end and blob[cursor:cursor + 2] != b'\0\0':
            cursor += 2
        require(cursor + 2 <= end, 'unterminated VERSIONINFO key')
        key = blob[at + 6:cursor].decode('utf-16le')
        value_at = (cursor + 5) & ~3
        value_end = value_at + value_length * (2 if kind == 1 else 1)
        require(value_end <= end, 'truncated VERSIONINFO value')
        if kind == 1 and value_length:
            result[key] = blob[value_at:value_end].decode('utf-16le').rstrip('\0')
        cursor = (value_end + 3) & ~3
        while cursor + 6 <= end:
            used = block(cursor, end)
            cursor = (cursor + used + 3) & ~3
        return size
    block(0, len(blob))
    return result


def verify_executable(executable, approved_icon, description, version):
    resources = pe_resources(executable.read_bytes())
    versions = [version_strings(value) for (kind, _, _), value in resources.items() if kind == 16]
    require(versions, 'missing VERSIONINFO')
    for values in versions:
        require(values.get('ProductVersion') == version, 'marketed ProductVersion mismatch')
        require(values.get('FileDescription') == description, 'FileDescription mismatch')
    ico = approved_icon.read_bytes()
    reserved, kind, count = struct.unpack_from('<HHH', ico)
    require((reserved, kind) == (0, 1) and count > 0, 'invalid approved ICO')
    groups = [(language, value) for (kind, identity, language), value in resources.items()
              if kind == 14 and identity == 1]
    require(groups, 'missing RT_GROUP_ICON ID 1')
    dimensions = []
    for language, group in groups:
        require(group[:6] == ico[:6] and len(group) == 6 + count * 14, 'group icon table mismatch')
        dimensions = []
        for index in range(count):
            entry = ico[6 + index * 16:22 + index * 16]
            grouped = group[6 + index * 14:20 + index * 14]
            require(grouped[:12] == entry[:12], 'icon dimensions/format/size mismatch')
            length, start = struct.unpack_from('<II', entry, 8)
            identity, = struct.unpack_from('<H', grouped, 12)
            payload = resources.get((3, identity, language))
            require(payload is not None and len(payload) == length, 'missing/truncated RT_ICON')
            require(start + length <= len(ico) and payload == ico[start:start + length], 'embedded icon data differs from approved ICO')
            dimensions.append([entry[0] or 256, entry[1] or 256])
    return {'status': 'passed', 'description': description, 'version': version,
            'group_icon_id': 1, 'icon_dimensions': dimensions,
            'approved_icon_sha256': hashlib.sha256(ico).hexdigest()}


def desktop_available():
    if os.name != 'nt':
        return False
    user32 = ctypes.WinDLL('user32', use_last_error=True)
    user32.OpenInputDesktop.argtypes = [ctypes.c_uint32, ctypes.c_int, ctypes.c_uint32]
    user32.OpenInputDesktop.restype = ctypes.c_void_p
    user32.CloseDesktop.argtypes = [ctypes.c_void_p]
    desktop = user32.OpenInputDesktop(0, False, 1)
    if desktop:
        user32.CloseDesktop(desktop)
    return bool(desktop)


def command_results(log, command):
    marker = 'OXSHIFT run ' + command + ' '
    return [json.loads(line.split(marker, 1)[1]) for line in log.splitlines() if marker in line]


def validate_reports(folder, version):
    archives = sorted(folder.glob('*.zip'))
    require(len(archives) == 2, 'expected main and main+Settings report archives')
    captures = []
    for index, archive in enumerate(archives):
        with zipfile.ZipFile(archive) as report:
            require(report.testzip() is None, 'corrupt report ZIP')
            require(all(re.fullmatch(r'report\.json|window-\d+\.png', n) for n in report.namelist()), 'unexpected report archive entry')
            metadata = json.loads(report.read('report.json'))
            require(metadata['version'] == version, 'runtime report version mismatch')
            require(metadata['window_count'] == index + 1, 'owned window capture count mismatch')
            require(metadata['status'] in ('saved', 'partial'), 'invalid report completion state')
            ids = metadata['recent_command_ids']
            require(len(ids) <= 20 and all(isinstance(item, str) for item in ids), 'invalid command history')
            require(not any(item.startswith('help.') or item.endswith('.state') for item in ids), 'report/polling IDs leaked into history')
            serialized = json.dumps(metadata)
            require('PACKAGE_SMOKE_PRIVATE' not in serialized and 'package-project' not in serialized, 'project text/path or arguments leaked into metadata')
            images = metadata['screenshots']
            require(len(images) == index + 1, 'missing capture outcomes')
            outcomes = []
            for image in images:
                if image['status'] == 'saved':
                    png = report.read(image['file'])
                    require(png[:8] == b'\x89PNG\r\n\x1a\n', 'not a PNG capture')
                    dimensions = list(struct.unpack_from('>II', png, 16))
                    require(dimensions == [image['width'], image['height']] and min(dimensions) > 100, 'invalid PNG dimensions')
                    outcomes.append({'status': 'saved', 'dimensions': dimensions})
                else:
                    require(image['status'] == 'unavailable' and image.get('reason'), 'capture failure lacks reason')
                    outcomes.append({'status': 'unavailable', 'reason': image['reason'][:180]})
            require((metadata['status'] == 'saved') == all(i['status'] == 'saved' for i in outcomes), 'false screenshot success')
            captures.append(outcomes)
    require(len(list(folder.iterdir())) == 2, 'temporary report files were not cleaned up')
    return captures


def runtime_smoke(package, executable, version, mode):
    if mode == 'skip' or not desktop_available():
        require(mode != 'required', 'Windows input desktop is unavailable')
        return {'status': 'not-run', 'reason': 'explicit skip' if mode == 'skip' else 'Windows input desktop unavailable'}
    # Only this throwaway copy can receive screenshots, ZIPs and settings.
    # No profile override: the portable marker itself must route the writes.
    with tempfile.TemporaryDirectory(prefix='desktop-smoke-') as temporary:
        root = Path(temporary)
        copied = root / 'package'
        shutil.copytree(package, copied)
        project = root / 'package-project'
        project.mkdir()
        source = project / 'sample.txt'
        source.write_text('PACKAGE_SMOKE_PRIVATE\n', encoding='utf-8')
        poll = [['help.local-bug-report-state', {}] for _ in range(25)]
        commands = [
            ['project.open-file', {'path': str(source)}],
            ['settings.set', {'key': 'system.files.idle-seconds', 'value': 17}],
            ['help.create-local-bug-report', {}], *poll,
            ['settings.open', {'page': 'editor.font', 'filter': 'PACKAGE_SMOKE_PRIVATE'}],
            ['settings.open', {'page': 'editor.font', 'filter': ''}],
            ['help.create-local-bug-report', {}], *poll,
            ['settings.escape', {}],
            ['terminal.open', {'shell': os.environ.get('COMSPEC', 'cmd.exe'),
                               'input': 'echo OXSHIFT_TERMINAL_SMOKE\r\nexit\r\n'}],
            *[['terminal.state', {}] for _ in range(10)], ['app.quit', {}],
        ]
        env = {key: value for key, value in os.environ.items() if not key.startswith('OXSHIFT_')}
        env.update(OXSHIFT_BACKGROUND='1', OXSHIFT_FIRST_RUN='0', OXSHIFT_RUN=json.dumps(commands))
        process = subprocess.Popen([str(copied / executable), str(project)], cwd=project,
                                   env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                   creationflags=subprocess.CREATE_NEW_PROCESS_GROUP)
        try:
            output, _ = process.communicate(timeout=75)
        except subprocess.TimeoutExpired:
            subprocess.run(['taskkill', '/PID', str(process.pid), '/T', '/F'], capture_output=True, check=False)
            process.communicate(timeout=10)
            raise ValueError('desktop smoke timed out; only its process tree was terminated')
        log = output.decode('utf-8', errors='replace')
        if process.returncode != 0 and not command_results(log, 'help.local-bug-report-state'):
            gpu_failure = 'Creating DirectX devices' in log and any(text in log for text in ('Creating Direct3D device', 'Creating DXGI factory', 'Finding DXGI adapter'))
            if gpu_failure and mode == 'auto':
                return {'status': 'not-run', 'reason': 'DirectX initialization unavailable', 'exit_code': process.returncode}
        require(process.returncode == 0, 'desktop smoke process failed (exit {})'.format(process.returncode))
        states = command_results(log, 'help.local-bug-report-state')
        require(states and states[-1]['status'] in ('saved', 'partial'), 'report command did not complete')
        config = copied / 'data/config'
        require(re.search(r'^idle-seconds\s*=\s*17\s*$', (config / 'settings.toml').read_text(encoding='utf-8'), re.M), 'settings were not persisted in portable profile')
        captures = validate_reports(config / 'bug-reports', version)
        terminal = command_results(log, 'terminal.state')
        require(any(any(line.strip() == 'OXSHIFT_TERMINAL_SMOKE' for line in state.get('lines', [])) and state.get('started') for state in terminal), 'terminal command produced no verified output')
        complete = all(image['status'] == 'saved' for group in captures for image in group)
        require(mode != 'required' or complete, 'window capture unavailable on required desktop runner')
        return {'status': 'passed' if complete else 'partial', 'portable_settings': 'passed',
                'terminal': 'passed', 'report_captures': captures}


def inspect_package(package, version, mode):
    executables = [name for name in PRODUCTS if (package / name).is_file()]
    require(len(executables) == 1, 'expected one desktop product in the extracted package root')
    executable = executables[0]
    description, icon = PRODUCTS[executable]
    approved = ROOT / 'assets/brand' / icon
    require((package / icon).read_bytes() == approved.read_bytes(), 'packaged ICO differs from approved asset')
    require((package / 'data/.oxshift-portable').is_file(), 'portable marker missing')
    secrets = (package / 'data/config/secrets.toml').read_text(encoding='utf-8')
    require(re.search(r'^store\s*=\s*"memory"\s*$', secrets.strip()), 'portable package must use isolated memory secrets')
    pe = verify_executable(package / executable, approved, description, version)
    return {'product': executable, 'static': pe, 'portable_marker': 'passed',
            'runtime': runtime_smoke(package, executable, version, mode)}


def main():
    parser = argparse.ArgumentParser(description='Check an extracted Windows desktop package; never upload runtime captures.')
    parser.add_argument('--package', required=True, type=Path)
    parser.add_argument('--version', required=True)
    parser.add_argument('--runtime', choices=('auto', 'required', 'skip'), default='auto')
    parser.add_argument('--report', required=True, type=Path, help='Safe JSON summary only; no screenshots, logs or report ZIPs')
    args = parser.parse_args()
    try:
        result = inspect_package(args.package.resolve(), args.version, args.runtime)
        code = 0
    except (OSError, ValueError, KeyError, struct.error, zipfile.BadZipFile) as error:
        result = {'status': 'failed', 'reason': str(error)}
        code = 1
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(result))
    return code


if __name__ == '__main__':
    sys.exit(main())
