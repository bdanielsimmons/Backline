//! Where songs come from besides Tidal (files on the phone, YouTube clips), and what is kept between runs.

use crate::api::{self, Track};
use crate::decode;

/// A YouTube clip in the YT list.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct YtItem {
    pub vid: String,
    pub title: String,
    pub channel: String,
    pub secs: f32,
}

impl YtItem {
    pub(crate) fn track(&self) -> Track {
        Track { id: yt_id(&self.vid), title: self.title.clone(), artist: self.channel.clone(), duration: self.secs, ..Default::default() }
    }
}

/// A song's length and a 400-peak waveform (0-255), from its encoded bytes.
pub(crate) fn wave(bytes: &[u8]) -> (f32, Vec<u8>) {
    use rodio::Source;
    let Ok(src) = decode::SymSource::new(bytes.to_vec()) else { return (0.0, Vec::new()) };
    let (ch, rate) = (src.channels().max(1) as usize, src.sample_rate().max(1) as f32);
    let samples: Vec<i16> = src.collect();
    let frames = samples.len() / ch;
    let dur = frames as f32 / rate;
    let n = 400;
    let per = (frames / n).max(1);
    let peaks = (0..n)
        .map(|k| {
            let s = (k * per * ch).min(samples.len());
            let e = ((k + 1) * per * ch).min(samples.len());
            let m = samples[s..e].iter().step_by(ch * 8).map(|v| v.unsigned_abs()).max().unwrap_or(0);
            (m as u32 * 255 / 32768) as u8
        })
        .collect();
    (dur, peaks)
}

/// A YouTube clip's track number (its own range, apart from Tidal's and the files').
pub(crate) fn yt_id(vid: &str) -> i64 {
    -(1 + (hash(&format!("yt:{}", vid)) & 0x7fff_ffff_ffff) as i64) - 0x1_0000_0000_0000
}

/// The video in a link: watch?v=, youtu.be/, shorts/, or the bare 11-letter id.
pub(crate) fn video_id(link: &str) -> Option<String> {
    let l = link.trim();
    let take = |s: &str| -> Option<String> {
        let id: String = s.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
        (id.len() == 11).then_some(id)
    };
    for key in ["v=", "youtu.be/", "shorts/", "embed/", "live/"] {
        if let Some(i) = l.find(key) {
            return take(&l[i + key.len()..]);
        }
    }
    take(l)
}

/// The playlist in a link (list=...), when it is only a playlist (a video in a playlist is the video).
pub(crate) fn list_id(link: &str) -> Option<String> {
    if link.contains("v=") || link.contains("youtu.be/") {
        return None;
    }
    let i = link.find("list=")?;
    let id: String = link[i + 5..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    (!id.is_empty()).then_some(id)
}

pub(crate) fn yt_client() -> Result<rustypipe::client::RustyPipe, String> {
    let dir = api::config_dir().join("yt_client");
    let _ = std::fs::create_dir_all(&dir);
    rustypipe::client::RustyPipe::builder().storage_dir(dir).build().map_err(|e| e.to_string())
}

/// A clip's audio (AAC in an m4a, which the player reads), fetched once and kept.
pub(crate) fn yt_audio(vid: &str) -> Result<Vec<u8>, String> {
    let dir = api::config_dir().join("yt");
    let file = dir.join(format!("{}.m4a", vid));
    if let Ok(b) = std::fs::read(&file) {
        return Ok(b);
    }
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().map_err(|e| e.to_string())?;
    let stream = rt.block_on(async {
        let rp = yt_client()?;
        let p = rp.query().player(vid).await.map_err(|e| e.to_string())?;
        p.audio_streams
            .iter()
            .filter(|s| s.format == rustypipe::model::AudioFormat::M4a)
            .max_by_key(|s| s.bitrate)
            .map(|s| (s.url.clone(), s.size))
            .ok_or_else(|| "no audio this phone can play".to_string())
    })?;
    // in pieces: YouTube slows down one long download
    let http = reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(60)).build().map_err(|e| e.to_string())?;
    let (url, size) = stream;
    let mut out: Vec<u8> = Vec::with_capacity(size as usize);
    let piece = 4 * 1024 * 1024u64;
    let mut at = 0u64;
    while at < size.max(1) {
        let end = (at + piece - 1).min(size.saturating_sub(1));
        let b = http
            .get(&url)
            .header("Range", format!("bytes={}-{}", at, end))
            .send()
            .and_then(|r| r.error_for_status())
            .and_then(|r| r.bytes())
            .map_err(|e| e.to_string())?;
        if b.is_empty() {
            break;
        }
        at += b.len() as u64;
        out.extend_from_slice(&b);
        if size == 0 {
            break;
        }
    }
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(&file, &out);
    Ok(out)
}

/// What is on the phone's clipboard (PASTE), through Android's ClipboardManager.
pub(crate) fn clipboard() -> Option<String> {
    #[cfg(target_os = "android")]
    {
        use jni::objects::{JObject, JString};
        let ctx = ndk_context::android_context();
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
        let mut env = vm.attach_current_thread().ok()?;
        let activity = unsafe { JObject::from_raw(ctx.context().cast()) };
        let name = env.new_string("clipboard").ok()?;
        let cm = env.call_method(&activity, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[(&name).into()]).ok()?.l().ok()?;
        let clip = env.call_method(&cm, "getPrimaryClip", "()Landroid/content/ClipData;", &[]).ok()?.l().ok()?;
        if clip.is_null() {
            return None;
        }
        let item = env.call_method(&clip, "getItemAt", "(I)Landroid/content/ClipData$Item;", &[0i32.into()]).ok()?.l().ok()?;
        let text = env.call_method(&item, "getText", "()Ljava/lang/CharSequence;", &[]).ok()?.l().ok()?;
        if text.is_null() {
            return None;
        }
        let s = env.call_method(&text, "toString", "()Ljava/lang/String;", &[]).ok()?.l().ok()?;
        let s: String = env.get_string(&JString::from(s)).ok()?.into();
        return Some(s);
    }
    #[allow(unreachable_code)]
    None
}

pub(crate) fn load_json<T: serde::de::DeserializeOwned + Default>(name: &str) -> T {
    std::fs::read_to_string(api::config_dir().join(name)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub(crate) fn save_json<T: serde::Serialize>(name: &str, v: &T) {
    let _ = std::fs::create_dir_all(api::config_dir());
    if let Ok(s) = serde_json::to_string(v) {
        let _ = std::fs::write(api::config_dir().join(name), s);
    }
}

/// Backline's own Music folder on the phone: songs put here play without any permission.
pub(crate) fn music_dir() -> std::path::PathBuf {
    let base = std::env::var_os("BACKLINE_FILES").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("."));
    let d = base.join("Music");
    let _ = std::fs::create_dir_all(&d);
    d
}

pub(crate) fn hash(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links() {
        assert_eq!(video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=1").as_deref(), Some("dQw4w9WgXcQ"));
        assert_eq!(video_id("https://youtu.be/dQw4w9WgXcQ?si=x").as_deref(), Some("dQw4w9WgXcQ"));
        assert_eq!(list_id("https://youtube.com/playlist?list=PLabc_-1").as_deref(), Some("PLabc_-1"));
        assert_eq!(list_id("https://youtube.com/watch?v=dQw4w9WgXcQ&list=PLabc"), None);
    }
}
