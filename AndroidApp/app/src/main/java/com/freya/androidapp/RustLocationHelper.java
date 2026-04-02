package com.freya.androidapp;

import android.content.ActivityNotFoundException;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.location.Location;
import android.location.LocationListener;
import android.location.LocationManager;
import android.os.Looper;

import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.Consumer;

public final class RustLocationHelper {
    private static final Map<Long, LocationListener> LOCATION_LISTENERS = new ConcurrentHashMap<>();
    private static final Map<Long, BroadcastReceiver> LOCATION_STATE_RECEIVERS = new ConcurrentHashMap<>();
    private static final Map<Long, Boolean> LOCATION_STATE_LAST_EMITTED = new ConcurrentHashMap<>();

    private RustLocationHelper() {
    }

    public static Consumer<Location> createLocationConsumer(long callbackPtr) {
        return location -> {
            if (location == null) {
                onLocationError(callbackPtr);
                return;
            }

            onLocationResult(
                    callbackPtr,
                    location.getLatitude(),
                    location.getLongitude(),
                    location.getAccuracy()
            );
        };
    }

    public static native void onLocationResult(
            long callbackPtr,
            double lat,
            double lng,
            float accuracy
    );

    public static native void onLocationError(long callbackPtr);

    public static native void onPermissionResult(long callbackPtr, boolean granted);

    public static native void onLocationChanged(
            long callbackPtr,
            double lat,
            double lng,
            float accuracy
    );

        public static native void onLocationEnabledChanged(long callbackPtr, boolean enabled);

    public static void requestLocationPermission(Context context, long callbackPtr) {
        Intent intent = new Intent(context, PermissionRequestActivity.class)
                .putExtra(PermissionRequestActivity.EXTRA_CALLBACK_PTR, callbackPtr);

        if (!(context instanceof android.app.Activity)) {
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        }

        try {
            context.startActivity(intent);
        } catch (ActivityNotFoundException | SecurityException e) {
            onPermissionResult(callbackPtr, false);
        }
    }

    public static void startLocationUpdates(Context context, long callbackPtr) {
        LocationManager locationManager = (LocationManager) context.getSystemService(Context.LOCATION_SERVICE);
        if (locationManager == null) {
            throw new IllegalStateException("LocationManager is unavailable");
        }

        LocationListener listener = location -> {
            if (location == null) {
                return;
            }

            onLocationChanged(
                    callbackPtr,
                    location.getLatitude(),
                    location.getLongitude(),
                    location.getAccuracy()
            );
        };

        LOCATION_LISTENERS.put(callbackPtr, listener);

        try {
            locationManager.requestLocationUpdates(
                    LocationManager.GPS_PROVIDER,
                    1000L,
                    1f,
                    listener,
                    Looper.getMainLooper()
            );
        } catch (SecurityException e) {
            LOCATION_LISTENERS.remove(callbackPtr);
            throw e;
        }
    }

    public static void stopLocationUpdates(Context context, long callbackPtr) {
        LocationManager locationManager = (LocationManager) context.getSystemService(Context.LOCATION_SERVICE);
        LocationListener listener = LOCATION_LISTENERS.remove(callbackPtr);
        if (locationManager == null || listener == null) {
            return;
        }

        locationManager.removeUpdates(listener);
    }

    public static void startLocationEnabledUpdates(Context context, long callbackPtr) {
        LocationManager locationManager = (LocationManager) context.getSystemService(Context.LOCATION_SERVICE);
        if (locationManager == null) {
            onLocationEnabledChanged(callbackPtr, false);
            return;
        }

        BroadcastReceiver receiver = new BroadcastReceiver() {
            @Override
            public void onReceive(Context c, Intent intent) {
                boolean enabled = locationManager.isLocationEnabled();
                Boolean lastEmitted = LOCATION_STATE_LAST_EMITTED.get(callbackPtr);
                if (lastEmitted != null && lastEmitted == enabled) {
                    return;
                }

                LOCATION_STATE_LAST_EMITTED.put(callbackPtr, enabled);
                onLocationEnabledChanged(callbackPtr, enabled);
            }
        };

        LOCATION_STATE_RECEIVERS.put(callbackPtr, receiver);

        IntentFilter filter = new IntentFilter(LocationManager.PROVIDERS_CHANGED_ACTION);
        filter.addAction(LocationManager.MODE_CHANGED_ACTION);

        try {
            context.registerReceiver(receiver, filter);
        } catch (Exception e) {
            LOCATION_STATE_RECEIVERS.remove(callbackPtr);
            LOCATION_STATE_LAST_EMITTED.remove(callbackPtr);
            throw e;
        }

        boolean enabled = locationManager.isLocationEnabled();
        LOCATION_STATE_LAST_EMITTED.put(callbackPtr, enabled);
        onLocationEnabledChanged(callbackPtr, enabled);
    }

    public static void stopLocationEnabledUpdates(Context context, long callbackPtr) {
        BroadcastReceiver receiver = LOCATION_STATE_RECEIVERS.remove(callbackPtr);
        LOCATION_STATE_LAST_EMITTED.remove(callbackPtr);
        if (receiver == null) {
            return;
        }

        try {
            context.unregisterReceiver(receiver);
        } catch (IllegalArgumentException ignored) {
        }
    }
}
