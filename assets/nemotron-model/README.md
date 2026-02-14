# Nemotron Model Placeholder

This directory will be populated with the Nemotron speech streaming model files by running:

```
./scripts/download_nemotron_model.sh
```

Required files:
- `encoder.onnx`
- `encoder.onnx.data`
- `decoder_joint.onnx`
- `tokenizer.model`

The download script pulls the int8 quantized ONNX export from https://huggingface.co/lokkju/nemotron-speech-streaming-en-0.6b-int8.
