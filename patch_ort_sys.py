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
    lines = f.readlines()

for i, line in enumerate(lines):
    if 'cache_dir()' in line:
        print(f"Found cache_dir at line {i+1}: {repr(line)}")
        print(f"Next line {i+2}: {repr(lines[i+1])}")
        lines[i] = '\t\t\tlet bin_extract_dir = std::path::PathBuf::from("/tmp/ort-cache")\n'
        lines[i+1] = '\t\t\t\t// patched\n'
        print("Patched successfully")
        break

with open(path, 'w') as f:
    f.writelines(lines)

os.makedirs("/tmp/ort-cache", exist_ok=True)
print("Done")
