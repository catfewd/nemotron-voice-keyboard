package com.catfewd.nemotron;

import android.content.Intent;
import android.speech.RecognitionService;
import android.util.Log;

/**
 * Minimal implementation to register as a system voice recognition service.
 */
public class NemotronRecognitionService extends RecognitionService {
    private static final String TAG = "NemotronRecService";

    @Override
    protected void onStartListening(Intent recognizerIntent, Callback listener) {
        Log.d(TAG, "onStartListening");
        // For now, system-level voice input can just be handled via the IME.
        // This service exists mostly for the OS registration.
    }

    @Override
    protected void onCancel(Callback listener) {
        Log.d(TAG, "onCancel");
    }

    @Override
    protected void onStopListening(Callback listener) {
        Log.d(TAG, "onStopListening");
    }
}
