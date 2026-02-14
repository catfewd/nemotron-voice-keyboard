#!/bin/bash
set -e

# --- Configuration ---
if [ "$GITHUB_ACTIONS" = "true" ]; then
    # GitHub Actions environment
    AAPT2=$(find $ANDROID_HOME/build-tools -name aapt2 | sort -r | head -n 1)
    D8=$(find $ANDROID_HOME/build-tools -name d8 | sort -r | head -n 1)
    APKSIGNER=$(find $ANDROID_HOME/build-tools -name apksigner | sort -r | head -n 1)
    ZIPALIGN=$(find $ANDROID_HOME/build-tools -name zipalign | sort -r | head -n 1)
    PLATFORM=$(find $ANDROID_HOME/platforms -name android.jar | sort -r | head -n 1)
else
    # Local Termux environment
    AAPT2=$(which aapt2)
    D8=$(which d8)
    APKSIGNER=$(which apksigner)
    ZIPALIGN=$(which zipalign)
    PLATFORM="/data/data/com.termux/files/home/android-sdk/platforms/android-33/android.jar"
fi

# Keystore
KEYSTORE="$HOME/.android/debug.keystore"
STORE_PASS="android"
KEY_ALIAS="androiddebugkey"
KEY_PASS="android"

# --- Build Steps ---
rm -rf build_manual
mkdir -p build_manual/gen build_manual/obj build_manual/apk build_manual/lib/arm64-v8a

# 1. Build Rust
echo "--- Building Rust ---"
cargo build --target aarch64-linux-android --release

# 2. Compile Resources
echo "--- Compiling Resources ---"
$AAPT2 compile --dir res -o build_manual/resources.zip
$AAPT2 link -I "$PLATFORM" \
    --manifest AndroidManifest.xml \
    -o build_manual/apk/unaligned.apk \
    build_manual/resources.zip \
    --java build_manual/gen \
    --auto-add-overlay

# 3. Compile Java
echo "--- Compiling Java ---"
find src/java -name "*.java" > build_manual/sources.txt
find build_manual/gen -name "*.java" >> build_manual/sources.txt

javac -d build_manual/obj \
    -source 1.8 -target 1.8 \
    -classpath "$PLATFORM" \
    @build_manual/sources.txt

# 4. Dex
echo "--- Dexing ---"
find build_manual/obj -name "*.class" > build_manual/classes.txt
$D8 --output build_manual/apk \
    --lib "$PLATFORM" \
    @build_manual/classes.txt

# 5. Package
echo "--- Packaging ---"
cp target/aarch64-linux-android/release/libandroid_transcribe_app.so build_manual/lib/arm64-v8a/
cp jniLibs/arm64-v8a/libonnxruntime.so build_manual/lib/arm64-v8a/
cp /data/data/com.termux/files/usr/lib/libc++_shared.so build_manual/lib/arm64-v8a/

cd build_manual/apk
zip -g unaligned.apk classes.dex
cp -r ../lib .
zip -r0 unaligned.apk lib
if [ -d "../../assets" ]; then
    cp -r ../../assets .
    zip -r0 -g unaligned.apk assets
fi
cd ../..

# 6. Sign
echo "--- Aligning and Signing ---"
$ZIPALIGN -f -p -v 4 build_manual/apk/unaligned.apk build_manual/apk/aligned.apk
$APKSIGNER sign --ks "$KEYSTORE" \
    --ks-pass "pass:$STORE_PASS" \
    --key-pass "pass:$KEY_PASS" \
    --ks-key-alias "$KEY_ALIAS" \
    --out Nemotron_Voice_Keyboard.apk \
    build_manual/apk/aligned.apk

echo "SUCCESS: Nemotron_Voice_Keyboard.apk created"
