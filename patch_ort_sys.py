import glob
import sys
import os
import subprocess

# Force fetch ort-sys first
subprocess.run(["cargo", "fetch", "--target", "aarch64-linux-android"], 
               capture_output=True)

matches = glob.glob(os.path.expanduser(
    "~/.cargo/registry/src/*/ort-sys-*/build/main.rs"
))

if not matches:
    print("ort-sys main.rs not found")
    # List what we have for debugging
    for p in glob.glob(os.path.expanduser("~/.cargo/registry/src/*/*")):
        print(p)
    sys.exit(1)

path = matches[0]
print("Patching:", path)

with open(path, 'r') as f:
    lines = f.readlines()

print("Line 97:", repr(lines[96]))
print("Line 98:", repr(lines[97]))

lines[96] = '\t\tlet bin_extract_dir = std::path::PathBuf::from("/tmp/ort-cache")\n'
lines[97] = '\t\t\t// patched\n'

with open(path, 'w') as f:
    f.writelines(lines)

os.makedirs("/tmp/ort-cache", exist_ok=True)
print("Done")
