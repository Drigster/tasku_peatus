package dev.drigster.taskupeatus

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import android.os.Build
import android.view.WindowInsets

object AndroidUiHelper {
    @JvmStatic
    fun getBarSizes(context: Context): FloatArray {
        val activity = resolveActivity(context) ?: return floatArrayOf(0f, 0f, 0f, 0f)
        val decorView = activity.window?.decorView ?: return floatArrayOf(0f, 0f, 0f, 0f)
        val rootInsets = decorView.rootWindowInsets ?: return floatArrayOf(0f, 0f, 0f, 0f)

        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            val insets = rootInsets.getInsets(WindowInsets.Type.systemBars())
            floatArrayOf(
                insets.top.toFloat(),
                insets.right.toFloat(),
                insets.bottom.toFloat(),
                insets.left.toFloat()
            )
        } else {
            @Suppress("DEPRECATION")
            floatArrayOf(
                rootInsets.systemWindowInsetTop.toFloat(),
                rootInsets.systemWindowInsetRight.toFloat(),
                rootInsets.systemWindowInsetBottom.toFloat(),
                rootInsets.systemWindowInsetLeft.toFloat()
            )
        }
    }

    private fun resolveActivity(context: Context): Activity? {
        if (context is Activity) {
            return context
        }

        var current: Context? = context
        while (current is ContextWrapper) {
            if (current is Activity) {
                return current
            }
            current = current.baseContext
        }

        return ActivityProvider.getCurrentActivity()
    }
}
