package dev.drigster.taskupeatus

import android.app.Activity
import java.lang.ref.WeakReference

object ActivityProvider {
    @Volatile
    private var activityRef: WeakReference<Activity>? = null

    @JvmStatic
    fun setCurrentActivity(activity: Activity) {
        activityRef = WeakReference(activity)
    }

    @JvmStatic
    fun getCurrentActivity(): Activity? = activityRef?.get()

    @JvmStatic
    fun clearCurrentActivity(activity: Activity) {
        if (activityRef?.get() === activity) {
            activityRef = null
        }
    }
}
