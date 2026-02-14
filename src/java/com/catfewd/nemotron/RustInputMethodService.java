package com.catfewd.nemotron;

import android.inputmethodservice.InputMethodService;
import android.view.View;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;
import android.widget.ProgressBar;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;
import android.content.Context;
import android.content.pm.PackageManager;
import android.view.MotionEvent;

public class RustInputMethodService extends InputMethodService {
    
    private static final String TAG = "OfflineVoiceInput";

    static {
        try {
            System.loadLibrary("c++_shared");
            System.loadLibrary("onnxruntime");
            System.loadLibrary("android_transcribe_app");
        } catch (UnsatisfiedLinkError e) {
            Log.e(TAG, "Failed to load native libraries", e);
        }
    }

    private TextView statusView;
    private TextView hintView;
    private TextView previewView;
    private android.widget.ScrollView previewScroll;
    private View recordContainer;
    private android.widget.ImageView micIcon;
    private ProgressBar progressBar;
    private Handler mainHandler;
    private boolean isRecording = false;
    private String lastStatus = "Initializing...";

    @Override
    public void onCreate() {
        super.onCreate();
        mainHandler = new Handler(Looper.getMainLooper());
        Log.d(TAG, "Service onCreate");
        try {
            initNative(this);
        } catch (Exception e) {
            Log.e(TAG, "Error in initNative", e);
        }
    }

    @Override
    public View onCreateInputView() {
        Log.d(TAG, "onCreateInputView");
        try {
            View view = getLayoutInflater().inflate(R.layout.ime_layout, null);
            
            // Handle window insets for avoiding navigation bar overlap
            view.setOnApplyWindowInsetsListener((v, insets) -> {
                int paddingBottom = insets.getSystemWindowInsetBottom();
                int originalPaddingBottom = v.getPaddingTop();
                v.setPadding(v.getPaddingLeft(), v.getPaddingTop(), v.getPaddingRight(), originalPaddingBottom + paddingBottom);
                return insets;
            });

            statusView = view.findViewById(R.id.ime_status_text);
            progressBar = view.findViewById(R.id.ime_progress);
            recordContainer = view.findViewById(R.id.ime_record_container);
            micIcon = view.findViewById(R.id.ime_mic_icon);
            hintView = view.findViewById(R.id.ime_hint);
            previewView = view.findViewById(R.id.ime_preview_text);
            previewScroll = view.findViewById(R.id.ime_preview_scroll);

            View.OnClickListener stopListener = v -> {
                if (isRecording) {
                    stopRecording();
                    updateRecordButtonUI(false);
                }
            };
            statusView.setOnClickListener(stopListener);
            previewView.setOnClickListener(stopListener);

            recordContainer.setOnClickListener(v -> {
                if (!recordContainer.isEnabled()) return;

                // Check microphone permission
                if (checkSelfPermission(android.Manifest.permission.RECORD_AUDIO)
                        != PackageManager.PERMISSION_GRANTED) {
                    if (statusView != null) statusView.setText("No mic permission - grant in app");
                    if (hintView != null) hintView.setText("Open the app to grant permission");
                    return;
                }

                if (isRecording) {
                    stopRecording();
                    updateRecordButtonUI(false);
                } else {
                    startRecording();
                    updateRecordButtonUI(true);
                }
            });

            applyTheme(view);
            updateUiState();
            return view;
        } catch (Exception e) {
            Log.e(TAG, "Error in onCreateInputView", e);
            TextView errorView = new TextView(this);
            errorView.setText("Error loading keyboard: " + e.getMessage());
            return errorView;
        }
    }

    private void applyTheme(View root) {
        android.content.SharedPreferences prefs = getSharedPreferences("settings", Context.MODE_PRIVATE);
        boolean forceDark = prefs.getBoolean("dark_mode", false);
        
        // If not forced dark, let system decide via values-night (default behavior)
        if (!forceDark) return;

        // Force dark colors
        int bgColor = 0xFF000000;
        int cardColor = 0xFF1E1E1E;
        int keyColor = 0xFF333333; // Dark gray
        int textColor = 0xFFE0E0E0;
        int subTextColor = 0xFFB0B0B0;
        int iconColor = 0xFFFFFFFF; // Pure white for icons

        root.setBackgroundColor(bgColor);
        statusView.setTextColor(textColor);
        previewView.setTextColor(textColor);
        hintView.setTextColor(iconColor); // Make hint white in dark mode
        
        recordContainer.setBackgroundTintList(android.content.res.ColorStateList.valueOf(cardColor));
    }
    
    @Override
    public void onStartInputView(android.view.inputmethod.EditorInfo info, boolean restarting) {
        super.onStartInputView(info, restarting);
        Log.d(TAG, "onStartInputView");
        
        // Auto-start recording if permissions are granted
        if (checkSelfPermission(android.Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED) {
            mainHandler.postDelayed(() -> {
                if (!isRecording) {
                    startRecording();
                    updateRecordButtonUI(true);
                }
            }, 300); // Small delay to ensure everything is ready
        }
    }

    private void updateRecordButtonUI(boolean recording) {
        isRecording = recording;
        if (recording) {
            micIcon.setColorFilter(0xFFF44336); // Red
            micIcon.setVisibility(View.GONE);
            hintView.setVisibility(View.GONE);
            previewScroll.setVisibility(View.VISIBLE);
            previewView.setText("");
            statusView.setText("Recording... Tap Anywhere to Send");
            statusView.setTextColor(0xFFF44336); // Red
        } else {
            micIcon.setColorFilter(0xFF2196F3); // Blue
            micIcon.setVisibility(View.VISIBLE);
            hintView.setVisibility(View.VISIBLE);
            previewScroll.setVisibility(View.GONE);
            statusView.setText("Ready");
            
            // Resolve text color from theme if not forced dark
            android.content.SharedPreferences prefs = getSharedPreferences("settings", Context.MODE_PRIVATE);
            if (prefs.getBoolean("dark_mode", false)) {
                statusView.setTextColor(0xFFE0E0E0);
            } else {
                statusView.setTextColor(0xFF333333);
            }
            hintView.setText("Tap to Record");
        }
    }
    
    @Override
    public void onDestroy() {
        super.onDestroy();
        cleanupNative();
    }

    // Native methods
    private native void initNative(RustInputMethodService service);
    private native void cleanupNative();
    private native void startRecording();
    private native void stopRecording();
    
    // Called from Rust
    public void onStatusUpdate(String status) {
        mainHandler.post(() -> {
            Log.d(TAG, "Status: " + status);
            lastStatus = status;
            updateUiState();
        });
    }

    // Called from Rust
    public void onPartialResult(String text) {
        mainHandler.post(() -> {
            if (previewView != null && isRecording) {
                previewView.setText(text);
                // Auto-scroll to bottom
                previewScroll.post(() -> previewScroll.fullScroll(View.FOCUS_DOWN));
            }
        });
    }

    private void updateUiState() {
        if (statusView != null && !isRecording) {
            statusView.setText(lastStatus);
        }

        boolean isLoading = lastStatus.contains("Loading") || lastStatus.contains("Initializing");
        boolean isWaiting = lastStatus.contains("Waiting");
        boolean isTranscribing = lastStatus.contains("Transcribing") || lastStatus.contains("Processing");
        boolean isError = lastStatus.startsWith("Error");

        // Show progress bar during loading or waiting for model
        if (progressBar != null) {
            progressBar.setVisibility(isLoading || isWaiting ? View.VISIBLE : View.GONE);
        }

        // Disable button only during transcription/processing/waiting or fatal errors
        if (recordContainer != null) {
            boolean disable = isTranscribing || isWaiting || isError;
            recordContainer.setEnabled(!disable);
            recordContainer.setAlpha(disable ? 0.5f : 1.0f);
        }

        // Update hint during loading to indicate recording is available
        if (isLoading && hintView != null && !isRecording) {
            hintView.setText("Tap to Record (model loading)");
        }
    }
    
    // Called from Rust
    public void onTextTranscribed(String text) {
        mainHandler.post(() -> {
            if (getCurrentInputConnection() != null) {
                getCurrentInputConnection().commitText(text + " ", 1);
            }
            updateRecordButtonUI(false);
            if (statusView != null) statusView.setText("Ready");
            
            // Revert back to the previous input method
            switchToPreviousInputMethod();
        });
    }
}