"""Round-trip producer View IR colors through the actual native shell decoders.

The Swift Color and Kotlin Compose Color stand-ins expose channels only; decoding
is extracted verbatim from each shell. This does not exercise a mobile UI host.
"""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
SWIFT_STUB = '''import Foundation
struct Color {
    enum Space { case sRGB }
    var r: Double; var g: Double; var b: Double; var a: Double
    init(_ space: Space, red: Double, green: Double, blue: Double, opacity: Double) {
        r = red; g = green; b = blue; a = opacity
    }
}
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crepus', default=str(ROOT / 'target/debug/crepus'))
    args = parser.parse_args()
    # Distinct channel bytes distinguish RGBA from ARGB and every channel swap.
    colors = ['12345678', '9abcdef0', '12345600', '123456ff', 'abcdef']
    source = '\n'.join(f'div bg-[#{color}]' for color in colors)
    result = subprocess.run([args.crepus, 'native', 'ir', '--stdin'], input=source,
                            text=True, capture_output=True, check=True)
    wire = [node['style']['backgroundColor'] for node in json.loads(result.stdout)['root']]
    assert wire == [f'#{color}' for color in colors], wire
    expected = [color if len(color) == 8 else color + 'ff' for color in colors]
    with tempfile.TemporaryDirectory(prefix='crepus-shell-colors-') as temporary:
        out = Path(temporary)
        checks = []
        for value, rgba in zip(wire, expected):
            channels = [int(rgba[i:i + 2], 16) for i in range(0, 8, 2)]
            checks.append(f'guard let c = Color(hex: "{value}") else {{ fatalError("valid color rejected") }}\n'
                          'precondition(' + ' && '.join(
                              f'abs(c.{channel} - {byte}.0 / 255) < 0.000001'
                              for channel, byte in zip('rgba', channels)) + ')')
        # Each check has its own scope so the production initializer is invoked anew.
        swift_checks = '\n'.join('do {\n' + check + '\n}' for check in checks)
        swift_checks += '''
for malformed in ["#ff000z", "#12345g", "#12345678junk", "#+12345", "##ff0000"] {
    precondition(Color(hex: malformed) == nil, "malformed color accepted: \\(malformed)")
}
'''
        for relative in ['crates/crepuscularity-cli/assets/ios/ViewIrTreeView.swift',
                         'examples/native-shells/ios/Sources/NativeShell/ViewIrTreeView.swift']:
            text = (ROOT / relative).read_text()
            decoder = text[text.index('extension Color {'):text.index('/// Loads bundled fixture')]
            swift = out / 'check.swift'
            swift.write_text(SWIFT_STUB + decoder + swift_checks)
            subprocess.run(['swiftc', '-warnings-as-errors', str(swift), '-o', str(out / 'check')], check=True)
            subprocess.run([str(out / 'check')], check=True)
            print(f'PASS {relative}: producer RGBA round-trip, opaque/transparent, malformed rejection')

        text = (ROOT / 'examples/native-shells/android/app/src/main/java/dev/crepuscularity/nativeshell/ViewIrTree.kt').read_text()
        decoder = text[text.index('private fun parseColor('):]
        kotlin_checks = '\n'.join(
            f'check(parseColor("{value}").argb == 0x{rgba[6:]}{rgba[:6]}L)'
            for value, rgba in zip(wire, expected))
        kotlin = out / 'Check.kt'
        kotlin.write_text('''data class Color(val argb: Long) {
    companion object { val Gray = Color(0xFF888888L) }
}
''' + decoder + '\nfun main() {\n' + kotlin_checks + '\n}\n')
        jar = out / 'check.jar'
        subprocess.run(['kotlinc', str(kotlin), '-include-runtime', '-d', str(jar)], check=True)
        subprocess.run(['java', '-jar', str(jar)], check=True)
        print('PASS Android shell: producer RGBA round-trip through Compose ARGB packing')


if __name__ == '__main__':
    main()
