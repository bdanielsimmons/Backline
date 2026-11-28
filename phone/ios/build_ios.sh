#!/bin/bash
# The iPhone app, built on a Mac (GitHub's: .github/workflows/ios.yml): phone/ios/build_ios.sh [sim]
#   sim: for the simulator, target/ios/Backline-sim.app; else for iPhones, target/ios/Backline.ipa, unsigned: the
#   sideloading app (SideStore, AltStore) signs it with the person's own Apple ID as it installs it.
set -euo pipefail
cd "$(dirname "$0")/.."
SIM=0; [ "${1:-}" = sim ] && SIM=1
OUT="$PWD/target/ios"; mkdir -p "$OUT"
BUILD="${BACKLINE_BUILD:-0}"
if [ $SIM = 1 ]; then
  T=aarch64-apple-ios-sim; SDK=iphonesimulator; PLAT=iPhoneSimulator; APP="$OUT/Backline-sim.app"; CT=arm64-apple-ios15.0-simulator
else
  T=aarch64-apple-ios; SDK=iphoneos; PLAT=iPhoneOS; APP="$OUT/Payload/Backline.app"; CT=arm64-apple-ios15.0
fi
export IPHONEOS_DEPLOYMENT_TARGET=15.0
SDKP="$(xcrun --sdk $SDK --show-sdk-path)"
# (YouTube's player-code engine has its bindings made for the phone, from the phone's own headers)
export LIBCLANG_PATH="$(xcode-select -p)/Toolchains/XcodeDefault.xctoolchain/usr/lib"
# (clang's own name for the target: it doesn't know Rust's "-sim")
export BINDGEN_EXTRA_CLANG_ARGS="--target=$CT --sysroot=$SDKP"
cargo build --release --target $T --example ios
BIN="target/$T/release/examples/ios"

rm -rf "$APP"; mkdir -p "$APP"
cp "$BIN" "$APP/Backline"
sed -e "s/@BUILD@/$BUILD/g" ios/Info.plist > "$APP/Info.plist"
/usr/libexec/PlistBuddy -c "Add :CFBundleSupportedPlatforms array" -c "Add :CFBundleSupportedPlatforms:0 string $PLAT" "$APP/Info.plist"
# the icon and the launch screen's colour (Studio's), put in by Xcode's own tool
A="$OUT/assets/Assets.xcassets"; rm -rf "$OUT/assets"; mkdir -p "$A/AppIcon.appiconset" "$A/LaunchBg.colorset"
cp ios/AppIcon.png "$A/AppIcon.appiconset/"
echo '{"info":{"author":"xcode","version":1}}' > "$A/Contents.json"
echo '{"images":[{"filename":"AppIcon.png","idiom":"universal","platform":"ios","size":"1024x1024"}],"info":{"author":"xcode","version":1}}' > "$A/AppIcon.appiconset/Contents.json"
echo '{"colors":[{"color":{"color-space":"srgb","components":{"red":"0.082","green":"0.086","blue":"0.067","alpha":"1.000"}},"idiom":"universal"}],"info":{"author":"xcode","version":1}}' > "$A/LaunchBg.colorset/Contents.json"
xcrun actool "$A" --compile "$APP" --platform $SDK --minimum-deployment-target 15.0 --app-icon AppIcon \
  --target-device iphone --output-partial-info-plist "$OUT/assets/icon.plist" --output-format human-readable-text | grep -iE "error|warning" || true
/usr/libexec/PlistBuddy -c "Merge $OUT/assets/icon.plist" "$APP/Info.plist"
# (signed for this Mac alone: the simulator wants it, and a sideloading app signs it again for a phone)
codesign --force --sign - --timestamp=none "$APP"
if [ $SIM = 0 ]; then
  (cd "$OUT" && rm -f Backline.ipa && zip -qry Backline.ipa Payload)
  ls -la "$OUT/Backline.ipa" | awk '{print "built: Backline.ipa (" int($5/1048576) " MB)"}'
else
  echo "built: $APP"
fi
