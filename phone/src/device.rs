//! The music already on the phone: Android's own list of it (MediaStore: every audio file the phone has found, with
//! its title, artist, album and length), read through Java. Asked for once (Android's "music and audio" permission,
//! or storage on older phones). A track is played from its file, or through Android when the file can't be opened
//! by its path; its album art comes from Android too.

use crate::api;
use crate::yt::Song;

/// A track on the phone as a song: its id is "file:" and Android's number for it; `path` its file.
pub(crate) fn is_file(vid: &str) -> bool {
    vid.starts_with("file:")
}

/// The phone's top folder (its own storage), which Files shows folders from. An iPhone lets an app see only its own
/// folder: the one the Files app shows as On My iPhone > Backline.
fn root() -> String {
    #[cfg(target_os = "android")]
    {
        return "/storage/emulated/0".to_string();
    }
    #[cfg(target_os = "ios")]
    {
        return crate::ios::documents().to_string_lossy().to_string();
    }
    #[allow(unreachable_code)]
    api::config_dir().join("Music").to_string_lossy().replace('\\', "/")
}

/// A track's folder from the phone's top folder ("Music/Albums"; "" right in it), a memory card's under "SD card".
pub(crate) fn folder_of(path: &str) -> String {
    let path = path.replace('\\', "/");
    let dir = path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let root = root();
    if dir == root {
        return String::new();
    }
    if let Some(r) = dir.strip_prefix(&format!("{}/", root)) {
        return r.trim_matches('/').to_string();
    }
    if let Some(r) = dir.strip_prefix("/storage/") {
        let rest = r.split_once('/').map(|(_, x)| x.trim_matches('/')).unwrap_or("");
        return if rest.is_empty() { "SD card".to_string() } else { format!("SD card/{}", rest) };
    }
    if dir.is_empty() {
        "Other".to_string()
    } else {
        dir.trim_matches('/').to_string()
    }
}

#[cfg(target_os = "android")]
fn with_env<R>(f: impl FnOnce(&mut jni::JNIEnv, &jni::objects::JObject) -> Option<R>) -> Option<R> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let app = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };
    let r = f(&mut env, &app);
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
    r
}

/// Android's version number (33 is Android 13).
#[cfg(target_os = "android")]
fn sdk() -> i32 {
    with_env(|env, _| env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I").ok()?.i().ok()).unwrap_or(30)
}

/// The permission that lets an app read the phone's music (Android 13 and on: music and audio; before: storage).
#[cfg(target_os = "android")]
fn perm() -> &'static str {
    if sdk() >= 33 {
        "android.permission.READ_MEDIA_AUDIO"
    } else {
        "android.permission.READ_EXTERNAL_STORAGE"
    }
}

/// Whether the app may read the phone's music.
pub(crate) fn allowed() -> bool {
    #[cfg(target_os = "android")]
    {
        let p = perm();
        return with_env(|env, app| {
            let s = env.new_string(p).ok()?;
            Some(env.call_method(app, "checkSelfPermission", "(Ljava/lang/String;)I", &[(&s).into()]).ok()?.i().ok()? == 0)
        })
        .unwrap_or(false);
    }
    #[allow(unreachable_code)]
    true
}

/// Android's question: may Backline see the music on this phone?
pub(crate) fn ask() {
    #[cfg(target_os = "android")]
    {
        let p = perm();
        let _ = with_env(|env, _| {
            // (the Activity itself asks: the app's context is the Application, which can't)
            let act = crate::ANDROID.get()?.activity_as_ptr();
            let activity = unsafe { jni::objects::JObject::from_raw(act.cast()) };
            let s = env.new_string(p).ok()?;
            let arr = env.new_object_array(1, "java/lang/String", &s).ok()?;
            env.call_method(&activity, "requestPermissions", "([Ljava/lang/String;I)V", &[(&arr).into(), jni::objects::JValue::Int(8)]).ok()?;
            Some(())
        });
    }
}

/// Every music track on the phone (newest first), from Android's list.
pub(crate) fn scan() -> Result<Vec<Song>, String> {
    #[cfg(target_os = "android")]
    {
        use jni::objects::{JObject, JString, JValue};
        return with_env(|env, app| {
            let cr = env.call_method(app, "getContentResolver", "()Landroid/content/ContentResolver;", &[]).ok()?.l().ok()?;
            let uri = env.get_static_field("android/provider/MediaStore$Audio$Media", "EXTERNAL_CONTENT_URI", "Landroid/net/Uri;").ok()?.l().ok()?;
            let cols = ["_id", "title", "artist", "duration", "_data"];
            let proj = env.new_object_array(cols.len() as i32, "java/lang/String", JObject::null()).ok()?;
            for (i, c) in cols.iter().enumerate() {
                let s = env.new_string(c).ok()?;
                env.set_object_array_element(&proj, i as i32, s).ok()?;
            }
            // (music, and other audio over 10 seconds: recordings, lessons; not the phone's ringtones and alerts; a file
            // Android hasn't looked inside yet has none of these, and is shown)
            let sel = env
                .new_string(
                    "(is_music != 0 OR is_music IS NULL OR duration > 10000) AND (is_ringtone IS NULL OR is_ringtone = 0)                      AND (is_notification IS NULL OR is_notification = 0) AND (is_alarm IS NULL OR is_alarm = 0)",
                )
                .ok()?;
            let order = env.new_string("date_added DESC").ok()?;
            let cur = env
                .call_method(
                    &cr,
                    "query",
                    "(Landroid/net/Uri;[Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;Ljava/lang/String;)Landroid/database/Cursor;",
                    &[(&uri).into(), (&proj).into(), (&sel).into(), JValue::Object(&JObject::null()), (&order).into()],
                )
                .ok()?
                .l()
                .ok()?;
            if cur.is_null() {
                return Some(Vec::new());
            }
            let mut out = Vec::new();
            let get_s = |env: &mut jni::JNIEnv, i: i32| -> String {
                env.call_method(&cur, "getString", "(I)Ljava/lang/String;", &[JValue::Int(i)])
                    .ok()
                    .and_then(|v| v.l().ok())
                    .filter(|o| !o.is_null())
                    .and_then(|o| env.get_string(&JString::from(o)).ok().map(String::from))
                    .unwrap_or_default()
            };
            while env.call_method(&cur, "moveToNext", "()Z", &[]).ok()?.z().ok()? {
                let id = env.call_method(&cur, "getLong", "(I)J", &[JValue::Int(0)]).ok()?.j().ok()?;
                let title = get_s(env, 1);
                let artist = get_s(env, 2);
                let dur = env.call_method(&cur, "getLong", "(I)J", &[JValue::Int(3)]).ok()?.j().ok()?;
                let path = get_s(env, 4);
                let artist = if artist == "<unknown>" { String::new() } else { artist };
                out.push(Song { vid: format!("file:{}", id), title, channel: artist, secs: dur as f32 / 1000.0, path });
            }
            let _ = env.call_method(&cur, "close", "()V", &[]);
            Some(out)
        })
        .ok_or_else(|| "the phone's music list couldn't be read".to_string());
    }
    #[allow(unreachable_code)]
    {
        // (an iPhone: the songs put in Backline's folder in the Files app, and the folders in it; the PC copy: the
        // music in the profile's Music folder, for trying it out)
        let mut out = Vec::new();
        let top = std::path::PathBuf::from(root());
        let _ = std::fs::create_dir_all(&top);
        let mut dirs = vec![top];
        while let Some(dir) = dirs.pop() {
            for e in std::fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten() {
                let p = e.path();
                if p.is_dir() {
                    dirs.push(p);
                } else if p.extension().is_some_and(|x| ["mp3", "m4a", "flac", "wav", "ogg"].contains(&x.to_string_lossy().to_lowercase().as_str())) {
                    let name = p.file_stem().unwrap_or_default().to_string_lossy().to_string();
                    let (who, what) = name.split_once(" - ").map(|(a, b)| (a.to_string(), b.to_string())).unwrap_or((String::new(), name.clone()));
                    out.push(Song { vid: format!("file:{}", crate::yt::hash_str(&p.to_string_lossy())), title: what, channel: who, secs: 0.0, path: p.to_string_lossy().to_string() });
                }
            }
        }
        Ok(out)
    }
}

/// A track's file: read by its path, else through Android (a file the app can't open by its path).
pub(crate) fn read(s: &Song) -> Result<Vec<u8>, String> {
    if !s.path.is_empty() {
        if let Ok(b) = std::fs::read(&s.path) {
            if !b.is_empty() {
                return Ok(b);
            }
        }
    }
    #[cfg(target_os = "android")]
    {
        use jni::objects::JValue;
        let id: i64 = s.vid.trim_start_matches("file:").parse().map_err(|_| "not a track on this phone".to_string())?;
        return with_env(|env, app| {
            let cr = env.call_method(app, "getContentResolver", "()Landroid/content/ContentResolver;", &[]).ok()?.l().ok()?;
            let base = env.get_static_field("android/provider/MediaStore$Audio$Media", "EXTERNAL_CONTENT_URI", "Landroid/net/Uri;").ok()?.l().ok()?;
            let uri = env.call_static_method("android/content/ContentUris", "withAppendedId", "(Landroid/net/Uri;J)Landroid/net/Uri;", &[(&base).into(), JValue::Long(id)]).ok()?.l().ok()?;
            let ins = env.call_method(&cr, "openInputStream", "(Landroid/net/Uri;)Ljava/io/InputStream;", &[(&uri).into()]).ok()?.l().ok()?;
            let buf = env.new_byte_array(1 << 16).ok()?;
            let mut out: Vec<u8> = Vec::new();
            let mut chunk = vec![0i8; 1 << 16];
            loop {
                let n = env.call_method(&ins, "read", "([B)I", &[(&buf).into()]).ok()?.i().ok()?;
                if n <= 0 {
                    break;
                }
                env.get_byte_array_region(&buf, 0, &mut chunk[..n as usize]).ok()?;
                out.extend(chunk[..n as usize].iter().map(|b| *b as u8));
            }
            let _ = env.call_method(&ins, "close", "()V", &[]);
            Some(out)
        })
        .filter(|b| !b.is_empty())
        .ok_or_else(|| "the file couldn't be read".to_string());
    }
    #[allow(unreachable_code)]
    Err("the file couldn't be read".to_string())
}

/// A track's album art (as Android keeps it), square, 160 across.
pub(crate) fn art(s: &Song) -> Option<eframe::egui::ColorImage> {
    #[cfg(target_os = "android")]
    {
        use jni::objects::{JObject, JValue};
        let id: i64 = s.vid.trim_start_matches("file:").parse().ok()?;
        let png = with_env(|env, app| {
            let cr = env.call_method(app, "getContentResolver", "()Landroid/content/ContentResolver;", &[]).ok()?.l().ok()?;
            let base = env.get_static_field("android/provider/MediaStore$Audio$Media", "EXTERNAL_CONTENT_URI", "Landroid/net/Uri;").ok()?.l().ok()?;
            let uri = env.call_static_method("android/content/ContentUris", "withAppendedId", "(Landroid/net/Uri;J)Landroid/net/Uri;", &[(&base).into(), JValue::Long(id)]).ok()?.l().ok()?;
            let size = env.new_object("android/util/Size", "(II)V", &[JValue::Int(160), JValue::Int(160)]).ok()?;
            // (Android 10 and on; a track with no art throws, which comes back as none)
            let bmp = env
                .call_method(&cr, "loadThumbnail", "(Landroid/net/Uri;Landroid/util/Size;Landroid/os/CancellationSignal;)Landroid/graphics/Bitmap;", &[(&uri).into(), (&size).into(), JValue::Object(&JObject::null())])
                .ok()?
                .l()
                .ok()?;
            if bmp.is_null() {
                return None;
            }
            let fmt = env.get_static_field("android/graphics/Bitmap$CompressFormat", "PNG", "Landroid/graphics/Bitmap$CompressFormat;").ok()?.l().ok()?;
            let bos = env.new_object("java/io/ByteArrayOutputStream", "()V", &[]).ok()?;
            env.call_method(&bmp, "compress", "(Landroid/graphics/Bitmap$CompressFormat;ILjava/io/OutputStream;)Z", &[(&fmt).into(), JValue::Int(90), (&bos).into()]).ok()?;
            let arr = env.call_method(&bos, "toByteArray", "()[B", &[]).ok()?.l().ok()?;
            env.convert_byte_array(jni::objects::JByteArray::from(arr)).ok()
        })?;
        let img = image::load_from_memory(&png).ok()?.to_rgba8();
        let (w, h) = (img.width(), img.height());
        let sq = w.min(h);
        let img = image::imageops::crop_imm(&img, (w - sq) / 2, (h - sq) / 2, sq, sq).to_image();
        let img = image::imageops::resize(&img, 160, 160, image::imageops::FilterType::Triangle);
        return Some(eframe::egui::ColorImage::from_rgba_unmultiplied([160, 160], img.as_raw()));
    }
    #[allow(unreachable_code)]
    {
        let _ = s;
        None
    }
}
