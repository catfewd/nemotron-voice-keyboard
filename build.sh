#!/bin/bash
set -e

# --- 1. Cache & Environment Setup ---
# Must be first so all subsequent commands inherit these paths.
PROJECT_ROOT="$(pwd)"
export XDG_CACHE_HOME="$PROJECT_ROOT/.cache"
export XDG_DATA_HOME="$(pwd)/.local/share"
export HOME="/data/data/com.termux/files/home"
mkdir -p "$XDG_CACHE_HOME" "$XDG_DATA_HOME"

# --- 2. Automatic Permission Recovery ---
if [ -d "target" ] || [ -d "build_manual" ]; then
    echo "--- Checking file permissions ---"
    chown -R $(id -u):$(id -g) .
fi

# --- 3. Configuration & Paths ---
AAPT2=$(command -v aapt2)
D8=$(command -v d8)
APKSIGNER=$(command -v apksigner)
ZIPALIGN=$(command -v zipalign)
PLATFORM="$HOME/android-sdk/platforms/android-33/android.jar"

KEYSTORE="$HOME/.android/debug.keystore"
STORE_PASS="android"
KEY_ALIAS="androiddebugkey"
KEY_PASS="android"

ORT_LIBS="$(pwd)/jniLibs/arm64-v8a"
ORT_HEADERS="$(pwd)/libs/onnxruntime/headers"

# --- 4. ort-sys v2 Environment ---
export ORT_STRATEGY="system"
export ORT_LIB_LOCATION="$ORT_LIBS"
export ORT_INCLUDE_DIR="$ORT_HEADERS"
export ORT_SKIP_DOWNLOAD="1"
export LIBRARY_PATH="$ORT_LIBS:${LIBRARY_PATH:-}"
export LD_LIBRARY_PATH="$ORT_LIBS:${LD_LIBRARY_PATH:-}"

# --- 5. Cargo Configuration ---
echo "--- Configuring Cargo ---"
mkdir -p .cargo
cat > .cargo/config.toml << EOF
[target.aarch64-linux-android]
linker = "clang"
rustflags = [
    "-C", "link-arg=-L$ORT_LIBS",
    "-C", "link-arg=-lonnxruntime",
    "-C", "link-arg=-llog",
    "-C", "link-arg=-lc++_shared"
]
EOF

# --- 6. Clean & Build ---
echo "--- Starting Build ---"
rm -rf build_manual
mkdir -p build_manual/gen build_manual/obj build_manual/apk build_manual/lib/arm64-v8a

echo "--- Compiling Rust Code ---"
SO="target/aarch64-linux-android/release/libandroid_transcribe_app.so"
if [ -f "$SO" ] && [ "$SO" -nt "src/lib.rs" ] && [ "$SO" -nt "Cargo.toml" ]; then
    echo "--- Rust .so is up to date, skipping recompile ---"
else
    CC=clang cargo build --target aarch64-linux-android --release
fi

echo "--- Compiling Android Resources ---"
$AAPT2 compile --dir res -o build_manual/resources.zip
$AAPT2 link -I "$PLATFORM" --manifest AndroidManifest.xml \
    -o build_manual/apk/unaligned.apk build_manual/resources.zip \
    --java build_manual/gen --auto-add-overlay

echo "--- Compiling Java Source ---"
find src/java -name "*.java" > build_manual/sources.txt
[ -d "build_manual/gen" ] && find build_manual/gen -name "*.java" >> build_manual/sources.txt
javac -d build_manual/obj -source 1.8 -target 1.8 -classpath "$PLATFORM" @build_manual/sources.txt

echo "--- Converting to DEX ---"
find build_manual/obj -name "*.class" > build_manual/classes.txt
$D8 --output build_manual/apk --lib "$PLATFORM" @build_manual/classes.txt

echo "--- Packaging APK ---"
cp jniLibs/arm64-v8a/libandroid_transcribe_app.so build_manual/lib/arm64-v8a/
cp jniLibs/arm64-v8a/libonnxruntime.so build_manual/lib/arm64-v8a/
find .cache/ort -name "*.so" -exec cp {} build_manual/lib/arm64-v8a/ \; 2>/dev/null || true

cd build_manual/apk
zip -g unaligned.apk classes.dex
mkdir -p lib/arm64-v8a
cp ../lib/arm64-v8a/*.so lib/arm64-v8a/
zip -r0 unaligned.apk lib
if [ -d "$PROJECT_ROOT/assets" ]; then
    (cd "$PROJECT_ROOT" && zip -r0 "$PROJECT_ROOT/build_manual/apk/unaligned.apk" assets/)
fi
cd ../..

echo "--- Aligning and Signing APK ---"
$ZIPALIGN -f -p -v 4 build_manual/apk/unaligned.apk build_manual/apk/aligned.apk
$APKSIGNER sign --ks "$KEYSTORE" --ks-pass "pass:$STORE_PASS" \
    --key-pass "pass:$KEY_PASS" --ks-key-alias "$KEY_ALIAS" \
    --out Nemotron_Voice_Keyboard.apk build_manual/apk/aligned.apk

echo "----------------------------------------------"
echo "SUCCESS: Nemotron_Voice_Keyboard.apk created!"
echo "----------------------------------------------"