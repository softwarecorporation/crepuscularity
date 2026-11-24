#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
platform=${1:-macos}
case ${2:-} in ''|--check) ;; *) echo 'Unknown build option' >&2; exit 1 ;; esac
arch=$(uname -m)
case $arch in arm64) rust_arch=aarch64 ;; x86_64) rust_arch=x86_64 ;; *) exit 1 ;; esac
out="$PWD/.amp/in/orb-backup/swiftui-motion/$platform${2:+-check}"
mkdir -p "$out/Motion.app/Frameworks"
case "$platform" in
  macos) target=$rust_arch-apple-darwin; sdk=macosx; swift_target=$arch-apple-macosx14.0 ;;
  simulator)
    target=$rust_arch-apple-ios
    [[ $arch == arm64 ]] && target=$target-sim
    sdk=iphonesimulator; swift_target=$arch-apple-ios17.0-simulator ;;
  *) echo 'Usage: build.sh [macos|simulator]' >&2; exit 1 ;;
esac
rustup target add "$target"
export SDKROOT="$(xcrun --sdk "$sdk" --show-sdk-path)"
cargo build --locked -p crepuscularity-abi --target "$target"
cp "target/$target/debug/libcrepuscularity_abi.dylib" "$out/Motion.app/Frameworks/"
install_name_tool -id '@rpath/libcrepuscularity_abi.dylib' "$out/Motion.app/Frameworks/libcrepuscularity_abi.dylib"
source=examples/animation-swiftui/MotionApp.swift
if [[ ${2:-} == --check ]]; then
  [[ $platform == macos ]] || { echo '--check requires macos' >&2; exit 1; }
  source=examples/animation-swiftui/HostChecks.swift
fi
swiftc -parse-as-library -sdk "$(xcrun --sdk "$sdk" --show-sdk-path)" -target "$swift_target" \
  -import-objc-header crates/crepuscularity-abi/include/crepuscularity_abi.h \
  examples/animation-swiftui/MotionHost.swift "$source" \
  -L "$out/Motion.app/Frameworks" -lcrepuscularity_abi \
  -Xlinker -rpath -Xlinker '@executable_path/Frameworks' -o "$out/Motion.app/Motion"
cat > "$out/Motion.app/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>dev.crepuscularity.motion</string>
<key>CFBundleExecutable</key><string>Motion</string>
<key>CFBundleName</key><string>Motion</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>1</string>
<key>CFBundleShortVersionString</key><string>1.0</string>
<key>UILaunchScreen</key><dict/>
</dict></plist>
PLIST
codesign --force --sign - "$out/Motion.app/Frameworks/libcrepuscularity_abi.dylib"
codesign --force --sign - "$out/Motion.app"
if [[ ${2:-} == --check ]]; then "$out/Motion.app/Motion"; fi
echo "$out/Motion.app"
