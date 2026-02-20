import glob
import sys
import os

matches = glob.glob(os.path.expanduser(
    "~/.cargo/registry/src/*/ort-sys-*/build/main.rs"
))

if not matches:
    print("ort-sys main.rs not found, skipping patch")
    sys.exit(0)

path = matches[0]
print("Patching:", path)

with open(path, 'r') as f:
    lines = f.readlines()

lines[96] = '\t\tlet bin_extract_dir = std::path::PathBuf::from("/tmp/ort-cache")\n'
lines[97] = '\t\t\t// patched\n'

with open(path, 'w') as f:
    f.writelines(lines)

os.makedirs("/tmp/ort-cache", exist_ok=True)
print("Done")
