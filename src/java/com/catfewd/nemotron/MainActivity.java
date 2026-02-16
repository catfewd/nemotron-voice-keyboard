package com.catfewd.nemotron;

import android.app.Activity;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.os.Bundle;
import android.provider.Settings;
import android.util.Log;
import android.view.View;
import android.widget.Button;
import android.widget.TextView;

public class MainActivity extends Activity implements View.OnClickListener, android.widget.CompoundButton.OnCheckedChangeListener {

    private static final String TAG = "MainActivity";

    private static final int PERM_REQ_CODE = 101;



    static {

        try {

            System.loadLibrary("c++_shared");

            System.loadLibrary("onnxruntime");

        } catch (UnsatisfiedLinkError e) {

            Log.w(TAG, "Failed to load dependencies", e);

        }

        System.loadLibrary("android_transcribe_app");

    }



    private TextView statusText;

    private TextView welcomeSubtitle;

    private Button grantButton;

    private View permsCard;

    private Button startSubsButton;

    private android.widget.Switch darkThemeSwitch;



    @Override

    protected void onCreate(Bundle savedInstanceState) {

        super.onCreate(savedInstanceState);

        setContentView(R.layout.activity_main);



        statusText = findViewById(R.id.text_status);

        welcomeSubtitle = findViewById(R.id.text_welcome_subtitle);

        permsCard = findViewById(R.id.card_permissions);

        

        // Setup welcome subtitle click

        welcomeSubtitle.setOnClickListener(this);

        

        grantButton = findViewById(R.id.btn_grant_perms);

        startSubsButton = findViewById(R.id.btn_subs_start);

        Button imeSettingsButton = findViewById(R.id.btn_ime_settings);

        darkThemeSwitch = findViewById(R.id.switch_dark_mode);



        grantButton.setOnClickListener(this);

                startSubsButton.setOnClickListener(this);

                        imeSettingsButton.setOnClickListener(this);

                        

                        findViewById(R.id.credit_catfewd).setOnClickListener(this);

                        findViewById(R.id.credit_altunenes).setOnClickListener(this);

                        findViewById(R.id.credit_notune).setOnClickListener(this);

                findViewById(R.id.credit_lokkju).setOnClickListener(this);

                findViewById(R.id.credit_ort).setOnClickListener(this);

                findViewById(R.id.credit_cpal).setOnClickListener(this);

        

                android.content.SharedPreferences prefs = getSharedPreferences("settings", MODE_PRIVATE);

        darkThemeSwitch.setChecked(prefs.getBoolean("dark_mode", false));

        darkThemeSwitch.setOnCheckedChangeListener(this);



        updatePermissionUI();

        initNative(this);

    }



    @Override

    public void onCheckedChanged(android.widget.CompoundButton buttonView, boolean isChecked) {

        getSharedPreferences("settings", MODE_PRIVATE).edit().putBoolean("dark_mode", isChecked).apply();

    }



    @Override

    public void onClick(View v) {

        int id = v.getId();

        if (id == R.id.btn_grant_perms) {

            checkAndRequestPermissions();

        } else if (id == R.id.btn_subs_start) {

            startActivity(new Intent(this, LiveSubtitleActivity.class));

                        } else if (id == R.id.btn_ime_settings) {

                            startActivity(new Intent(Settings.ACTION_INPUT_METHOD_SETTINGS));

                        } else if (id == R.id.text_welcome_subtitle) {

                            openUrl("https://huggingface.co/lokkju/nemotron-speech-streaming-en-0.6b-int8");

                        } else if (id == R.id.credit_catfewd) {

                            openUrl("https://github.com/catfewd/nemotron-voice-keyboard");

                        } else if (id == R.id.credit_altunenes) {

                            openUrl("https://github.com/altunenes/parakeet-rs");

                        } else if (id == R.id.credit_notune) {

                            openUrl("https://github.com/notune/android_transcribe_app");

                        } else if (id == R.id.credit_lokkju) {

            openUrl("https://huggingface.co/lokkju/nemotron-speech-streaming-en-0.6b-int8");

        } else if (id == R.id.credit_ort) {

            openUrl("https://github.com/pykeio/ort");

        } else if (id == R.id.credit_cpal) {

            openUrl("https://github.com/RustAudio/cpal");

        }

    }



    private void openUrl(String url) {

        try {

            Intent intent = new Intent(Intent.ACTION_VIEW, Uri.parse(url));

            startActivity(intent);

        } catch (Exception e) {

            Log.e(TAG, "Error opening URL", e);

        }

    }



    @Override

    protected void onResume() {

        super.onResume();

        updatePermissionUI();

    }



    private void updatePermissionUI() {

        boolean hasAudio = checkSelfPermission(android.Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED;

        permsCard.setVisibility(hasAudio ? View.GONE : View.VISIBLE);

    }



    private void checkAndRequestPermissions() {

        if (checkSelfPermission(android.Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) {

            requestPermissions(new String[]{android.Manifest.permission.RECORD_AUDIO}, PERM_REQ_CODE);

        }

    }



    @Override

    public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] grantResults) {

        if (requestCode == PERM_REQ_CODE) updatePermissionUI();

    }



    public void onStatusUpdate(String status) {

        runOnUiThread(() -> {

            statusText.setText("Status: " + status);

            if ("Ready".equals(status)) startSubsButton.setEnabled(true);

        });

    }



    private native void initNative(MainActivity activity);

}
