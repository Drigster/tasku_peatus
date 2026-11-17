package dev.drigster.taskupeatus

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager

object RustPermissionHelper {
    init {
        try {
            System.loadLibrary("tasku_peatus")
        } catch (_: UnsatisfiedLinkError) {
        }
    }

    @JvmStatic
    external fun onPermissionResult(callbackPtr: Long, granted: Boolean)

    // Since Android 12, requesting fine location alone is ignored; coarse must be requested with it.
    val LOCATION_PERMISSIONS = arrayOf(
        android.Manifest.permission.ACCESS_FINE_LOCATION,
        android.Manifest.permission.ACCESS_COARSE_LOCATION,
    )

    @JvmStatic
    fun requestLocationPermission(context: Context, callbackPtr: Long) {
        if (LOCATION_PERMISSIONS.any {
                context.checkSelfPermission(it) == PackageManager.PERMISSION_GRANTED
            }
        ) {
            onPermissionResult(callbackPtr, true)
            return
        }

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
