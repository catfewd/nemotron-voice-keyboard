# Nemotron Voice Keyboard for Android

An offline, private, on-device voice transcription keyboard and live subtitle application powered by the NVIDIA Nemotron-3 0.6B INT8 model.

## Overview

This project provides a seamless voice typing experience for Android users who value privacy and performance. By running the NVIDIA Nemotron model locally on your device, transcription is fast, works without an internet connection, and ensures your voice data never leaves your hardware.

## Features

- Real-time streaming transcription.
- Automatic keyboard switching: Reverts to your previous keyboard (e.g., FUTO) immediately after you finish speaking.
- System-wide integration: Registered as a standard Android Voice Input provider.
- Forced Dark Mode support for high-visibility environments.
- Live Subtitles: Capture and caption any audio playing on your device in real-time.

## Credits and Acknowledgments

I am catfewd. I am not a programmer, and I do not have a background in software development. This project was made possible only through the incredible work of the following individuals and teams:

- **NoTune**: The creator of parakeet-rs and the original offline voice input application architecture. This project is built directly upon those foundations.
- **lokkju**: Responsible for the quantization of the Nemotron-3 0.6B model into the INT8 format used here.
- **Microsoft / pykeio**: The developers of the ONNX Runtime (ort) Rust bindings.
- **Rust Audio Team**: The developers of CPAL, used for cross-platform audio handling.

## Installation

Currently, the project is distributed as an APK for manual installation.

1. Download the latest `android_transcribe_app_release.apk`.
2. Enable "Install from Unknown Sources" in your Android settings.
3. Install the APK and follow the in-app instructions to enable the Nemotron Keyboard.
4. (Optional) Set Nemotron as your system-wide Voice Input app in: System Settings > Language & region > Speech > Voice input.

## License

This project is licensed under the MIT License - see the LICENSE file for details. Note that the underlying Nemotron model and various libraries used carry their own respective licenses.

---
Keywords: NVIDIA Nemotron, Android Voice Keyboard, Offline ASR, On-device Speech to Text, Private Dictation, Nemotron-3 0.6B, INT8 Quantization, Rust Android App.
