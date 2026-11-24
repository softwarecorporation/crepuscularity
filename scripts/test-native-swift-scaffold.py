"""Build a fresh Swift scaffold and run queued edits through its real Rust bridge."""
import argparse
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'scripts/fixtures/native-action-queue'


def run(*args):
    subprocess.run([str(arg) for arg in args], check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crepus', default=str(ROOT / 'target/debug/crepus'))
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='crepus-swift-scaffold-') as temporary:
        app = Path(temporary) / 'app'
        run(args.crepus, 'native', 'new', app)
        package = app / 'ios'
        generated = package / 'Sources/NativeShell/Generated'
        run('swift', 'build', '--package-path', package)
        run(args.crepus, 'native', 'codegen', FIXTURES / 'QueuedEdits.crepus',
            '--platform', 'swiftui', '--out', generated, '--view-name', 'CrepusGeneratedView')
        # Typecheck the real package with newly emitted controls, not extracted methods.
        run('swift', 'build', '--package-path', package)
        sdk = subprocess.check_output(['xcrun', '--sdk', 'iphonesimulator', '--show-sdk-path'], text=True).strip()
        run('swiftc', '-typecheck', '-swift-version', '5', '-sdk', sdk,
            '-target', 'arm64-apple-ios17.0-simulator',
            *sorted((package / 'Sources/NativeShell').rglob('*.swift')))
        run('cargo', 'test', '--manifest-path', app / 'rust/Cargo.toml')
        run('cargo', 'build', '--manifest-path', app / 'rust/Cargo.toml')
        bridge = app / 'rust/target/debug/libcrepus_mobile_actions.dylib'
        manifest = package / 'Package.swift'
        content = manifest.read_text()
        # Add only the external regression target to the fresh package.
        content = content.replace('\n    targets: [', '\n    targets: [\n        .testTarget(name: "NativeShellTests", dependencies: ["NativeShell"]),', 1)
        manifest.write_text(content)
        tests = package / 'Tests/NativeShellTests'
        tests.mkdir(parents=True)
        shutil.copyfile(FIXTURES / 'QueuedEditsTests.swift', tests / 'QueuedEditsTests.swift')
        run('swift', 'test', '--package-path', package, '-Xlinker', bridge)
        print('PASS fresh Swift scaffold builds and executes queued edits with the real Rust bridge')


if __name__ == '__main__':
    main()
