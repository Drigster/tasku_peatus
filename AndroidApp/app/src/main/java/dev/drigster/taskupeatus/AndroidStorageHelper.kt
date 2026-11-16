package dev.drigster.taskupeatus

import android.content.Context

object AndroidStorageHelper {
    @JvmStatic
    fun getCacheDir(context: Context): String = context.cacheDir.absolutePath
}
