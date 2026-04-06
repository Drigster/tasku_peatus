package dev.drigster.taskupeatus

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent

object RustPermissionHelper {
    init {
        try {
            System.loadLibrary("android_example")
        } catch (_: UnsatisfiedLinkError) {
        }
    }

    @JvmStatic
    external fun onPermissionResult(callbackPtr: Long, granted: Boolean)

    @JvmStatic
    fun requestLocationPermission(context: Context, callbackPtr: Long) {
        val intent = Intent(context, PermissionRequestActivity::class.java)
            .putExtra(PermissionRequestActivity.EXTRA_CALLBACK_PTR, callbackPtr)

        if (context !is android.app.Activity) {
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }

        try {
            context.startActivity(intent)
        } catch (_: ActivityNotFoundException) {
            onPermissionResult(callbackPtr, false)
        } catch (_: SecurityException) {
            onPermissionResult(callbackPtr, false)
        }
    }
}
