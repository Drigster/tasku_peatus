package com.freya.androidapp;

import android.os.Bundle;

import androidx.activity.ComponentActivity;
import androidx.activity.result.ActivityResultLauncher;
import androidx.activity.result.contract.ActivityResultContracts;

public final class PermissionRequestActivity extends ComponentActivity {
    public static final String EXTRA_CALLBACK_PTR = "extra_callback_ptr";

    private long callbackPtr;
    private boolean resultDelivered;
    private boolean requestLaunched;

    private final ActivityResultLauncher<String> permissionLauncher =
            registerForActivityResult(
                    new ActivityResultContracts.RequestPermission(),
                    isGranted -> {
                resultDelivered = true;
                        RustLocationHelper.onPermissionResult(callbackPtr, isGranted);
                        finish();
                    }
            );

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        callbackPtr = getIntent().getLongExtra(EXTRA_CALLBACK_PTR, 0L);
        if (callbackPtr == 0L) {
            resultDelivered = true;
            finish();
        }
    }

    @Override
    protected void onStart() {
        super.onStart();
        if (callbackPtr != 0L && !requestLaunched && !resultDelivered) {
            requestLaunched = true;
            permissionLauncher.launch(android.Manifest.permission.ACCESS_FINE_LOCATION);
        }
    }

    @Override
    protected void onDestroy() {
        if (!resultDelivered && callbackPtr != 0L) {
            resultDelivered = true;
            RustLocationHelper.onPermissionResult(callbackPtr, false);
        }
        super.onDestroy();
    }
}