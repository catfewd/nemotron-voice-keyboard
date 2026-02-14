#!/usr/bin/env bash
set -euo pipefail

MODEL_DIR="$(cd "$(dirname "$0")/.." && pwd)/assets/nemotron-model"
REPO="https://huggingface.co/lokkju/nemotron-speech-streaming-en-0.6b-int8/resolve/main"
FILES=(
  "encoder.onnx"
  "decoder_joint.onnx"
  "tokenizer.model"
)

mkdir -p "$MODEL_DIR"

echo "Downloading Nemotron 0.6B INT8 streaming model to $MODEL_DIR"
for file in "${FILES[@]}"; do
  url="$REPO/$file?download=1"
  dest="$MODEL_DIR/$file"
  echo "  - $file"
  curl -L "$url" -o "$dest"
  if [[ ! -s "$dest" ]]; then
    echo "Failed to download $file"
    exit 1
  fi
  sync
  chmod 644 "$dest"
done

echo "Download complete. Files ready in $MODEL_DIR"
