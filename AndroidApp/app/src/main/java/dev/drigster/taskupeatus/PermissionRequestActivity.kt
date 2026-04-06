package dev.drigster.taskupeatus

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.contract.ActivityResultContracts

class PermissionRequestActivity : ComponentActivity() {
    private var callbackPtr: Long = 0L
    private var resultDelivered = false
    private var requestLaunched = false

    private val permissionLauncher: ActivityResultLauncher<String> =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { isGranted ->
            resultDelivered = true
            RustPermissionHelper.onPermissionResult(callbackPtr, isGranted)
            finish()
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        callbackPtr = intent.getLongExtra(EXTRA_CALLBACK_PTR, 0L)
        if (callbackPtr == 0L) {
            resultDelivered = true
            finish()
        }
    }

    override fun onStart() {
        super.onStart()
        if (callbackPtr != 0L && !requestLaunched && !resultDelivered) {
            requestLaunched = true
            permissionLauncher.launch(android.Manifest.permission.ACCESS_FINE_LOCATION)
        }
    }

    override fun onDestroy() {
        if (!resultDelivered && callbackPtr != 0L) {
            resultDelivered = true
            RustPermissionHelper.onPermissionResult(callbackPtr, false)
        }
        super.onDestroy()
    }

    companion object {
        const val EXTRA_CALLBACK_PTR = "extra_callback_ptr"
    }
}
