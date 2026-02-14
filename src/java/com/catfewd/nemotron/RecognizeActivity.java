package com.catfewd.nemotron;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
import android.view.inputmethod.InputMethodManager;
import android.util.Log;
import android.provider.Settings;
import android.widget.Toast;

/**
 * Activity that handles android.speech.action.RECOGNIZE_SPEECH.
 */
public class RecognizeActivity extends Activity {
    private static final String TAG = "RecognizeActivity";

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        Log.d(TAG, "RecognizeActivity onCreate");

        try {
            InputMethodManager imm = (InputMethodManager) getSystemService(INPUT_METHOD_SERVICE);
            String id = Settings.Secure.getString(getContentResolver(), Settings.Secure.DEFAULT_INPUT_METHOD);
            String myId = getPackageName() + "/" + RustInputMethodService.class.getName();

            if (!myId.equals(id)) {
                imm.showInputMethodPicker();
                Toast.makeText(this, "Select Nemotron to start voice input", Toast.LENGTH_SHORT).show();
            } else {
                // If already active, just bring it up
                imm.showSoftInput(findViewById(android.R.id.content), 0);
            }
        } catch (Exception e) {
            Log.e(TAG, "Error switching IME", e);
        }

        finish();
    }
}
