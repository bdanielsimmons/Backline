#!/bin/bash
# Build the small phone app: phone/build_apk.sh [build] [arm]
#   arm: for real phones (arm64) into $S/backline-phone.apk; else for the emulator (x86_64), installed there with MUTE.
#   build: build only (no install).
# The app gets Python, QuickJS and yt-dlp added (phone/android_libs, from phone/fetch_libs.sh), then is aligned and
# signed with the lasting key in ~/.android (updates install over earlier copies).
export MSYS_NO_PATHCONV=1
export PATH="$HOME/.cargo/bin:$PATH"
S="${PHONE_OUT:-$(cygpath -m /c/Users/bdani/Documents/Backline/phone/target)}"; mkdir -p "$S"
SDK="$(cygpath -m "$LOCALAPPDATA")/Android/Sdk"
export ANDROID_HOME="$SDK"
export ANDROID_NDK_ROOT="$SDK/ndk/30.0.16248370"
export LIBCLANG_PATH="C:/Program Files/LLVM/bin"
sysroot="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/windows-x86_64/sysroot"
export BINDGEN_EXTRA_CLANG_ARGS_x86_64_linux_android="--target=x86_64-linux-android26 --sysroot=$sysroot -I\"C:/Program Files/LLVM/lib/clang/23/include\""
export BINDGEN_EXTRA_CLANG_ARGS_aarch64_linux_android="--target=aarch64-linux-android26 --sysroot=$sysroot -I\"C:/Program Files/LLVM/lib/clang/23/include\""
JBR="C:/Program Files/Android/Android Studio/jbr"
export JAVA_HOME="$JBR"
KS="$(cygpath -m "$HOME")/.android/backline-phone.jks"
# (its password beside it, never in the repository; a new key gets a new one)
KPF="$HOME/.android/backline-phone.pass"
[ -f "$KPF" ] || { [ -f "$KS" ] && { echo "the signing key's password ($KPF) is missing"; exit 1; }; head -c 18 /dev/urandom | base64 | tr -d "/+=" > "$KPF"; }
KP="$(cat "$KPF")"
[ -f "$KS" ] || "$JBR/bin/keytool.exe" -genkeypair -keystore "$KS" -alias backline -storepass $KP -keypass $KP -keyalg RSA -keysize 2048 -validity 20000 -dname "CN=Backline phone" >/dev/null 2>&1
export CARGO_APK_RELEASE_KEYSTORE="$KS"
export CARGO_APK_RELEASE_KEYSTORE_PASSWORD=$KP
export RUST_MIN_STACK=134217728
export CARGO_TARGET_DIR="$S/phoneapk"
BUILD=0; ARM=0
for a in "$@"; do [ "$a" = build ] && BUILD=1; [ "$a" = arm ] && ARM=1; done
if [ $ARM = 1 ]; then TARGET=aarch64-linux-android; ABI=arm64-v8a; OUT="$S/backline-phone.apk"; else TARGET=x86_64-linux-android; ABI=x86_64; OUT="$S/phone_local.apk"; fi
P=/c/Users/bdani/Documents/Backline/phone
cd $P
[ -f "android_libs/lib/$ABI/libpython.zip.so" ] || bash fetch_libs.sh >/dev/null || { echo "couldn't fetch yt-dlp's parts"; exit 1; }
nice cargo apk build --lib --release --target $TARGET > "$S/phone_apk.log" 2>&1
grep -E "^error" -A8 "$S/phone_apk.log" | head -30
apk="$CARGO_TARGET_DIR/release/apk/backline-practice.apk"
[ -f "$apk" ] && [ "$apk" -nt "$S/phone_apk.log" -o "$(grep -c '^error' "$S/phone_apk.log")" = 0 ] || { echo "no apk"; exit 1; }
BT="$SDK/build-tools/36.0.0"
AJAR="$SDK/platforms/android-34/android.jar"
APK_DIR="$CARGO_TARGET_DIR/release/apk"
# the Java side (audio focus, the media session, the playback service: phone/java), compiled and turned into dex
JC="$S/phone_java"; rm -rf "$JC"; mkdir -p "$JC/classes" "$JC/dex"
(cd "$P/java" && "$JBR/bin/javac.exe" -nowarn -source 8 -target 8 -bootclasspath "$AJAR" -d "$JC/classes" $(find . -name "*.java") 2>&1 | grep -vE "^(warning|Note|[0-9]+ warnings?)" ) 
(cd "$JC/classes" && "$JBR/bin/jar.exe" cf "$JC/classes.jar" .) || { echo "couldn't build the Java side"; exit 1; }
powershell.exe -NoProfile -Command "\$env:JAVA_HOME='$JBR'; & '$BT/d8.bat' --release --min-api 26 --lib '$AJAR' --output '$JC/dex' '$JC/classes.jar'" 2>&1 | grep -v "^$"
[ -f "$JC/dex/classes.dex" ] || { echo "no classes.dex"; exit 1; }
# cargo-apk's manifest with the Java side on and the playback service in, packaged with the app's pictures
sed -e 's/android:hasCode="false"/android:hasCode="true"/'     -e 's#</application>#<service android:name="com.v9studio.backline.practice.Playback" android:exported="false" android:foregroundServiceType="mediaPlayback"/></application>#'     "$APK_DIR/AndroidManifest.xml" > "$JC/AndroidManifest.xml"
rm -f "$S/phone_unsigned.apk"
"$BT/aapt.exe" package -f -M "$JC/AndroidManifest.xml" -S "$(cygpath -m "$P/res")" -I "$AJAR" -F "$S/phone_unsigned.apk" || { echo "couldn't package the app"; exit 1; }
(cd "$APK_DIR" && "$BT/aapt.exe" add "$S/phone_unsigned.apk" lib/$ABI/libbackline_phone.so >/dev/null) || { echo "couldn't add the app's library"; exit 1; }
(cd "$JC/dex" && "$BT/aapt.exe" add "$S/phone_unsigned.apk" classes.dex >/dev/null) || { echo "couldn't add the Java side"; exit 1; }
# Python, QuickJS and yt-dlp in beside the app's own library
(cd "$P/android_libs" && "$BT/aapt.exe" add "$S/phone_unsigned.apk" lib/$ABI/libpython.so lib/$ABI/libpython.zip.so lib/$ABI/libqjs.so lib/$ABI/libytdlp.so >/dev/null) || { echo "couldn't add yt-dlp's parts"; exit 1; }
"$BT/zipalign.exe" -f -p 4 "$S/phone_unsigned.apk" "$S/phone_aligned.apk"
powershell.exe -NoProfile -Command "\$env:JAVA_HOME='$JBR'; & '$BT/apksigner.bat' sign --ks '$KS' --ks-pass pass:$KP --key-pass pass:$KP --out '$OUT' '$S/phone_aligned.apk'" 2>&1 | grep -v "^WARNING" | grep -v "^$"
ls -la "$OUT" | awk '{print "built: " $NF " (" int($5/1048576) " MB)"}'
[ $BUILD = 1 -o $ARM = 1 ] && exit 0
A="$SDK/platform-tools/adb.exe"
"$A" install -r "$OUT" 2>&1 | tail -1
# my test runs play silently (the app reads this file at start)
"$A" shell mkdir -p /sdcard/Android/data/com.v9studio.backline.practice/files
"$A" shell touch /sdcard/Android/data/com.v9studio.backline.practice/files/MUTE
