package dev.drigster.taskupeatus

import android.app.NativeActivity
import android.os.Bundle
import android.view.SurfaceView
import android.view.View
import android.view.ViewGroup

class MainActivity : NativeActivity() {
    private fun findNativeSurfaceView(view: View): View? {
        if (view is SurfaceView) return view
        if (view is ViewGroup) {
            for (i in 0 until view.childCount) {
                val found = findNativeSurfaceView(view.getChildAt(i))
                if (found != null) return found
            }
        }
        return null
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        ActivityProvider.setCurrentActivity(this)
        window.decorView.post {
            findNativeSurfaceView(window.decorView)?.apply {
                isFocusable = true
                isFocusableInTouchMode = true
                requestFocus()
            }
        }
    }

    override fun onResume() {
        super.onResume()
        ActivityProvider.setCurrentActivity(this)
    }

    override fun onDestroy() {
        ActivityProvider.clearCurrentActivity(this)
        super.onDestroy()
    }
}
