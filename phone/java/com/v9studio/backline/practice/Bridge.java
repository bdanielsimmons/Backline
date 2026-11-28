package com.v9studio.backline.practice;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.drawable.Icon;
import android.media.AudioAttributes;
import android.media.AudioFocusRequest;
import android.media.AudioManager;
import android.media.MediaMetadata;
import android.media.session.MediaSession;
import android.media.session.PlaybackState;
import android.os.Handler;
import android.os.Looper;
import android.os.SystemClock;

/**
 * Backline as one of the phone's players, for the Rust app (phone/src/bridge.rs): audio focus (other players pause
 * when Backline plays, Backline pauses when they play or a call comes in), a media session (the lock screen, the
 * notification's controls, headphone buttons) and the notification itself, kept up by the Playback service so the
 * song keeps going in the background. What the phone asks for comes back to Rust through event(); the Rust side
 * registers it at start (RegisterNatives), so no library lookup is needed.
 */
public class Bridge {
    /** To the app: what (FOCUS..SEEK), and its value (the focus change, a position in ms). */
    static native void event(int what, long arg);

    static final int FOCUS = 1, PLAY = 2, PAUSE = 3, NEXT = 4, PREV = 5, STOP = 6, SEEK = 7;

    static final Handler main = new Handler(Looper.getMainLooper());
    static AudioManager am;
    static AudioFocusRequest req;
    static MediaSession session;
    static boolean serviceOn;
    // what's playing (for the notification and the session's state)
    static String title = "", artist = "";
    static Bitmap art;
    static boolean playing;
    static long durMs, posMs, posAt;
    static float speed = 1f;

    static void send(int what, long arg) {
        try {
            event(what, arg);
        } catch (Throwable t) {
            // (the app isn't up: nothing to tell)
        }
    }

    static final AudioManager.OnAudioFocusChangeListener focus = new AudioManager.OnAudioFocusChangeListener() {
        public void onAudioFocusChange(int change) {
            send(FOCUS, change);
        }
    };

    /** Ask for the phone's audio (other players pause). */
    public static boolean requestFocus(Context ctx) {
        try {
            if (am == null) am = (AudioManager) ctx.getApplicationContext().getSystemService(Context.AUDIO_SERVICE);
            if (req == null) {
                AudioAttributes aa = new AudioAttributes.Builder()
                        .setUsage(AudioAttributes.USAGE_MEDIA)
                        .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                        .build();
                req = new AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
                        .setAudioAttributes(aa)
                        .setOnAudioFocusChangeListener(focus, main)
                        .build();
            }
            return am.requestAudioFocus(req) == AudioManager.AUDIOFOCUS_REQUEST_GRANTED;
        } catch (Throwable t) {
            return false;
        }
    }

    /** Give the phone's audio back. */
    public static void abandonFocus() {
        try {
            if (am != null && req != null) am.abandonAudioFocusRequest(req);
        } catch (Throwable t) {
        }
    }

    /** What's playing, from the app: the session, the notification, the service. `active` false: none of it. */
    public static void update(Context ctx, final String t, final String a, final long dur, final long pos,
                              final boolean play, final float spd, final byte[] png, final boolean active) {
        final Context c = ctx.getApplicationContext();
        main.post(new Runnable() {
            public void run() {
                try {
                    if (!active) {
                        if (session != null) session.setActive(false);
                        if (serviceOn) c.stopService(new Intent(c, Playback.class));
                        serviceOn = false;
                        return;
                    }
                    makeSession(c);
                    title = t;
                    artist = a;
                    durMs = dur;
                    // (a new cover; an empty one: none; none sent: the one there stays)
                    if (png != null) art = png.length > 0 ? BitmapFactory.decodeByteArray(png, 0, png.length) : null;
                    MediaMetadata.Builder m = new MediaMetadata.Builder()
                            .putString(MediaMetadata.METADATA_KEY_TITLE, t)
                            .putString(MediaMetadata.METADATA_KEY_ARTIST, a)
                            .putLong(MediaMetadata.METADATA_KEY_DURATION, dur);
                    if (art != null) m.putBitmap(MediaMetadata.METADATA_KEY_ART, art);
                    session.setMetadata(m.build());
                    setState(play, pos, spd);
                    session.setActive(true);
                    if (!serviceOn && play) {
                        serviceOn = true;
                        c.startForegroundService(new Intent(c, Playback.class));
                    } else if (serviceOn) {
                        notifyNow(c);
                    }
                } catch (Throwable e) {
                    android.util.Log.w("Backline", "update: " + e);
                }
            }
        });
    }

    /** Playing or paused (the app's own word, or after a pause from the lock screen or another player). */
    public static void state(final boolean play) {
        main.post(new Runnable() {
            public void run() {
                try {
                    if (session == null) return;
                    setState(play, nowPos(), speed);
                    if (serviceOn && session.getController() != null) notifyNow(null);
                } catch (Throwable e) {
                }
            }
        });
    }

    static long nowPos() {
        long p = posMs;
        if (playing) p += (long) ((SystemClock.elapsedRealtime() - posAt) * speed);
        return Math.max(0, Math.min(p, durMs > 0 ? durMs : p));
    }

    static void setState(boolean play, long pos, float spd) {
        playing = play;
        posMs = pos;
        posAt = SystemClock.elapsedRealtime();
        speed = spd;
        long actions = PlaybackState.ACTION_PLAY | PlaybackState.ACTION_PAUSE | PlaybackState.ACTION_PLAY_PAUSE
                | PlaybackState.ACTION_SKIP_TO_NEXT | PlaybackState.ACTION_SKIP_TO_PREVIOUS | PlaybackState.ACTION_STOP
                | PlaybackState.ACTION_SEEK_TO;
        session.setPlaybackState(new PlaybackState.Builder()
                .setActions(actions)
                .setState(play ? PlaybackState.STATE_PLAYING : PlaybackState.STATE_PAUSED, pos, play ? spd : 0f, posAt)
                .build());
    }

    static void makeSession(Context c) {
        if (session != null) return;
        session = new MediaSession(c, "Backline");
        session.setCallback(new MediaSession.Callback() {
            public void onPlay() {
                requestFocus(appCtx);
                send(PLAY, 0);
            }

            public void onPause() {
                send(PAUSE, 0);
            }

            public void onSkipToNext() {
                send(NEXT, 0);
            }

            public void onSkipToPrevious() {
                send(PREV, 0);
            }

            public void onStop() {
                send(STOP, 0);
            }

            public void onSeekTo(long p) {
                send(SEEK, p);
            }
        }, main);
        appCtx = c;
    }

    static Context appCtx;

    static void notifyNow(Context c) {
        Context ctx = c != null ? c : appCtx;
        if (ctx == null) return;
        NotificationManager nm = ctx.getSystemService(NotificationManager.class);
        nm.notify(Playback.ID, notification(ctx));
    }

    /** The notification: the song, its cover, Previous / Play or Pause / Next, tied to the session. */
    static Notification notification(Context c) {
        NotificationManager nm = c.getSystemService(NotificationManager.class);
        if (nm.getNotificationChannel("play") == null) {
            NotificationChannel ch = new NotificationChannel("play", "Playing", NotificationManager.IMPORTANCE_LOW);
            ch.setShowBadge(false);
            nm.createNotificationChannel(ch);
        }
        Intent open = c.getPackageManager().getLaunchIntentForPackage(c.getPackageName());
        Notification.Builder b = new Notification.Builder(c, "play")
                .setSmallIcon(android.R.drawable.ic_media_play)
                .setContentTitle(title)
                .setContentText(artist)
                .setLargeIcon(art)
                .setOngoing(playing)
                .setShowWhen(false)
                .setVisibility(Notification.VISIBILITY_PUBLIC)
                .addAction(new Notification.Action.Builder(Icon.createWithResource(c, android.R.drawable.ic_media_previous), "Previous", act(c, "prev", 1)).build())
                .addAction(playing
                        ? new Notification.Action.Builder(Icon.createWithResource(c, android.R.drawable.ic_media_pause), "Pause", act(c, "pause", 2)).build()
                        : new Notification.Action.Builder(Icon.createWithResource(c, android.R.drawable.ic_media_play), "Play", act(c, "play", 3)).build())
                .addAction(new Notification.Action.Builder(Icon.createWithResource(c, android.R.drawable.ic_media_next), "Next", act(c, "next", 4)).build());
        if (open != null) b.setContentIntent(PendingIntent.getActivity(c, 0, open, PendingIntent.FLAG_IMMUTABLE | PendingIntent.FLAG_UPDATE_CURRENT));
        if (session != null) b.setStyle(new Notification.MediaStyle().setMediaSession(session.getSessionToken()).setShowActionsInCompactView(0, 1, 2));
        return b.build();
    }

    static PendingIntent act(Context c, String a, int n) {
        return PendingIntent.getService(c, n, new Intent(c, Playback.class).setAction(a), PendingIntent.FLAG_IMMUTABLE | PendingIntent.FLAG_UPDATE_CURRENT);
    }
}
