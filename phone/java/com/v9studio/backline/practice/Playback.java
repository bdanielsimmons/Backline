package com.v9studio.backline.practice;

import android.app.Service;
import android.content.Intent;
import android.content.pm.ServiceInfo;
import android.os.Build;
import android.os.IBinder;

/**
 * Keeps Backline playing in the background (a media playback service with the song's notification), and takes the
 * notification's buttons (Previous, Play or Pause, Next) to the app.
 */
public class Playback extends Service {
    static final int ID = 7;

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        String a = intent != null ? intent.getAction() : null;
        if ("prev".equals(a)) Bridge.send(Bridge.PREV, 0);
        else if ("next".equals(a)) Bridge.send(Bridge.NEXT, 0);
        else if ("pause".equals(a)) Bridge.send(Bridge.PAUSE, 0);
        else if ("play".equals(a)) {
            Bridge.requestFocus(this);
            Bridge.send(Bridge.PLAY, 0);
        }
        try {
            if (Build.VERSION.SDK_INT >= 29) {
                startForeground(ID, Bridge.notification(this), ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK);
            } else {
                startForeground(ID, Bridge.notification(this));
            }
        } catch (Throwable t) {
            android.util.Log.w("Backline", "foreground: " + t);
        }
        // (not started again by itself if the app is closed: the app starts it)
        return START_NOT_STICKY;
    }

    @Override
    public void onTaskRemoved(Intent rootIntent) {
        // the app swiped away: the song and its notification go with it
        Bridge.send(Bridge.STOP, 0);
        stopForeground(STOP_FOREGROUND_REMOVE);
        stopSelf();
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }
}
