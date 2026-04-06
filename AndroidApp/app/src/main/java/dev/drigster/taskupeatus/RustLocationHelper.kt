package dev.drigster.taskupeatus

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.os.Looper
import java.util.concurrent.ConcurrentHashMap
import java.util.function.Consumer

object RustLocationHelper {
    init {
        try {
            System.loadLibrary("android_example")
        } catch (_: UnsatisfiedLinkError) {
        }
    }

    private val LOCATION_LISTENERS = ConcurrentHashMap<Long, LocationListener>()
    private val LOCATION_STATE_RECEIVERS = ConcurrentHashMap<Long, BroadcastReceiver>()
    private val LOCATION_STATE_LAST_EMITTED = ConcurrentHashMap<Long, Boolean>()

    @JvmStatic
    fun createLocationConsumer(callbackPtr: Long): Consumer<Location> {
        return Consumer { location ->
            if (location == null) {
                onLocationError(callbackPtr)
                return@Consumer
            }

            onLocationResult(
                callbackPtr,
                location.latitude,
                location.longitude,
                location.accuracy
            )
        }
    }

    @JvmStatic
    external fun onLocationResult(
        callbackPtr: Long,
        lat: Double,
        lng: Double,
        accuracy: Float
    )

    @JvmStatic
    external fun onLocationError(callbackPtr: Long)

    @JvmStatic
    external fun onLocationChanged(
        callbackPtr: Long,
        lat: Double,
        lng: Double,
        accuracy: Float
    )

    @JvmStatic
    external fun onLocationEnabledChanged(callbackPtr: Long, enabled: Boolean)

    @JvmStatic
    fun startLocationUpdates(context: Context, callbackPtr: Long) {
        val locationManager = context.getSystemService(Context.LOCATION_SERVICE) as? LocationManager
            ?: throw IllegalStateException("LocationManager is unavailable")

        val listener = LocationListener { location ->
            if (location == null) {
                return@LocationListener
            }

            onLocationChanged(
                callbackPtr,
                location.latitude,
                location.longitude,
                location.accuracy
            )
        }

        LOCATION_LISTENERS[callbackPtr] = listener

        try {
            locationManager.requestLocationUpdates(
                LocationManager.GPS_PROVIDER,
                1000L,
                1f,
                listener,
                Looper.getMainLooper()
            )
        } catch (e: SecurityException) {
            LOCATION_LISTENERS.remove(callbackPtr)
            throw e
        }
    }

    @JvmStatic
    fun stopLocationUpdates(context: Context, callbackPtr: Long) {
        val locationManager = context.getSystemService(Context.LOCATION_SERVICE) as? LocationManager
        val listener = LOCATION_LISTENERS.remove(callbackPtr)
        if (locationManager == null || listener == null) {
            return
        }

        locationManager.removeUpdates(listener)
    }

    @JvmStatic
    fun startLocationEnabledUpdates(context: Context, callbackPtr: Long) {
        val locationManager = context.getSystemService(Context.LOCATION_SERVICE) as? LocationManager
        if (locationManager == null) {
            onLocationEnabledChanged(callbackPtr, false)
            return
        }

        val receiver = object : BroadcastReceiver() {
            override fun onReceive(c: Context, intent: Intent) {
                val enabled = locationManager.isLocationEnabled
                val lastEmitted = LOCATION_STATE_LAST_EMITTED[callbackPtr]
                if (lastEmitted != null && lastEmitted == enabled) {
                    return
                }

                LOCATION_STATE_LAST_EMITTED[callbackPtr] = enabled
                onLocationEnabledChanged(callbackPtr, enabled)
            }
        }

        LOCATION_STATE_RECEIVERS[callbackPtr] = receiver

        val filter = IntentFilter(LocationManager.PROVIDERS_CHANGED_ACTION)
        filter.addAction(LocationManager.MODE_CHANGED_ACTION)

        try {
            context.registerReceiver(receiver, filter)
        } catch (e: Exception) {
            LOCATION_STATE_RECEIVERS.remove(callbackPtr)
            LOCATION_STATE_LAST_EMITTED.remove(callbackPtr)
            throw e
        }

        val enabled = locationManager.isLocationEnabled
        LOCATION_STATE_LAST_EMITTED[callbackPtr] = enabled
        onLocationEnabledChanged(callbackPtr, enabled)
    }

    @JvmStatic
    fun stopLocationEnabledUpdates(context: Context, callbackPtr: Long) {
        val receiver = LOCATION_STATE_RECEIVERS.remove(callbackPtr)
        LOCATION_STATE_LAST_EMITTED.remove(callbackPtr)
        if (receiver == null) {
            return
        }

        try {
            context.unregisterReceiver(receiver)
        } catch (_: IllegalArgumentException) {
        }
    }
}
