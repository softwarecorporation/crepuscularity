#!/usr/bin/env python3
"""Compile generator action queues with small platform stubs; no Cargo/dependencies.

Swift removes only unrelated model/keyboard collaborators; queue/perform remain
extracted verbatim. Kotlin stubs Handler's main-post queue and unrelated JSON/FFI
collaborators, not the executor. This does not emulate full native UI lifecycles.
Only generator queues are covered: current checked-in shell snapshots are
synchronous, and host routing/draft/picker APIs from the original PR are absent.
Bound edits here check queue delivery, not serialization or state-store behavior.
"""
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'scripts/fixtures/native-action-queue'


def run(*args):
    subprocess.run([str(arg) for arg in args], check=True)


def kotlin_object(source):
    obj = source[source.index('object CrepusActions {'):]
    return obj[:obj.index('\n}\n') + 3]


def main():
    for tool in ['swiftc', 'kotlinc', 'java']:
        if shutil.which(tool) is None:
            sys.exit(f'Missing installed {tool}; install/configure it before this gate.')
    with tempfile.TemporaryDirectory(prefix='crepus-native-action-queue-') as directory:
        out = Path(directory)
        run(sys.executable, FIXTURES / 'extract-swift.py', ROOT, out)
        for variant in ['generated']:
            run('swiftc', '-swift-version', '5', '-warnings-as-errors',
                '-parse-as-library', out / f'{variant}.swift', '-o', out / variant)
            run(out / variant)
        source = (ROOT / 'crates/crepuscularity-native/src/codegen/compose.rs').read_text()
        match = re.search(r'let source = format!\(\s*("(?:[^"\\]|\\.)*")', source)
        if match is None:
            raise RuntimeError('Cannot locate Compose generator source literal')
        generated = json.loads(match.group(1)).replace('{{', '{').replace('}}', '}')
        for variant, text in [('generator', generated)]:
            actual = out / f'{variant}.kt'
            actual.write_text(kotlin_object(text))
            jar = out / f'{variant}.jar'
            run('kotlinc', actual, FIXTURES / 'Harness.kt', FIXTURES / 'Stubs.kt',
                FIXTURES / 'Android.kt', '-include-runtime', '-d', jar)
            run('java', '-jar', jar)
    print('PASS generator action queues: Swift FIFO40 + Kotlin FIFO1001 and delivery failures (platform/FFI stubs)')


if __name__ == '__main__':
    main()
