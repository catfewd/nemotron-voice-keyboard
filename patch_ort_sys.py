import glob
import sys
import os
import subprocess

subprocess.run(["cargo", "fetch", "--target", "aarch64-linux-android"],
               capture_output=True)

matches = glob.glob(os.path.expanduser(
    "~/.cargo/registry/src/*/ort-sys-*/build/main.rs"
))

if not matches:
    print("ort-sys main.rs not found")
    sys.exit(1)

path = matches[0]
print("Patching:", path)

with open(path, 'r') as f:
    content = f.read()

# Find and replace the cache_dir call regardless of line number
old = 'internal::dirs::cache_dir()\n\t\t\t\t.expect("could not determine cache directory")'
new = 'Ok::<std::path::PathBuf, ()>(std::path::PathBuf::from("/tmp/ort-cache")).unwrap_err(); let _unused = Ok::<std::path::PathBuf, ()>(std::path::PathBuf::from("/tmp/ort-cache"))'

if old not in content:
    # Try alternate indentation
    old = 'internal::dirs::cache_dir()\n\t\t\t.expect("could not determine cache directory")'

if old in content:
    content = content.replace(old, 'Ok::<std::path::PathBuf, Box<dyn std::error::Error>>(std::path::PathBuf::from("/tmp/ort-cache"))')
    with open(path, 'w') as f:
        f.write(content)
    os.makedirs("/tmp/ort-cache", exist_ok=True)
    print("Done - patched by content search")
else:
    print("Pattern not found, trying line-based search")
    with open(path, 'r') as f:
        lines = f.readlines()
    for i, line in enumerate(lines):
        if 'cache_dir()' in line:
            print(f"Found cache_dir at line {i+1}: {repr(line)}")
            print(f"Next line {i+2}: {repr(lines[i+1])}")
    sys.exit(1)
