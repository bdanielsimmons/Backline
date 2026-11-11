//! Crashes Windows saw. Backline's log hears of its own errors (a panic), but a crash in the graphics driver or in
//! Windows ends it before it can write a word. Windows keeps a record of each (the Application log, "Application
//! Error", event 1000): at start Backline reads the ones about itself it hasn't noted yet into log.txt and into
//! crashes.txt (which isn't cut when the log is), so a look at either says what happened and when.

/// One of Windows' crash records: when (UTC, as Windows wrote it), the program, the file the crash was in, the error
/// code and where in that file.
#[derive(Debug, PartialEq)]
pub(crate) struct Crash {
    pub at: String,
    pub app: String,
    pub module: String,
    pub code: String,
    pub offset: String,
}

impl Crash {
    fn line(&self) -> String {
        format!("{}  {} crashed in {}  code 0x{}  at 0x{}", self.at, self.app, self.module, self.code, self.offset)
    }
}

/// Read Windows' records in the background and note the new ones about this program (Windows only).
pub(crate) fn note_new() {
    #[cfg(windows)]
    std::thread::spawn(|| {
        if let Err(e) = note() {
            crate::api::log(&format!("crash records: couldn't read them: {}", e));
        }
    });
}

#[cfg(windows)]
fn note() -> Result<(), String> {
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    // (no window: CREATE_NO_WINDOW)
    let out = std::process::Command::new("wevtutil")
        .args([
            "qe",
            "Application",
            "/q:*[System[Provider[@Name='Application Error'] and (EventID=1000)]]",
            "/rd:true",
            "/c:60",
            "/f:xml",
        ])
        .creation_flags(0x0800_0000)
        .output()
        .map_err(|e| e.to_string())?;
    let xml = String::from_utf8_lossy(&out.stdout);
    let me = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()))
        .unwrap_or_default();
    let file = crate::api::config_dir().join("crashes.txt");
    let seen = std::fs::read_to_string(&file).unwrap_or_default();
    let mut new: Vec<Crash> = parse(&xml)
        .into_iter()
        .filter(|c| {
            let app = c.app.to_lowercase();
            (app == me || app == "backline.exe") && !seen.contains(&c.at)
        })
        .collect();
    // (Windows lists the newest first)
    new.reverse();
    if new.is_empty() {
        return Ok(());
    }
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&file).map_err(|e| e.to_string())?;
    for c in &new {
        let _ = writeln!(f, "{}", c.line());
        crate::api::log(&format!("CRASH (Windows' record): {}", c.line()));
    }
    Ok(())
}

/// Windows' crash records (wevtutil's XML) as crashes.
pub(crate) fn parse(xml: &str) -> Vec<Crash> {
    xml.split("<Event ")
        .skip(1)
        .filter_map(|e| {
            let at = between(e, "SystemTime='", "'")?.to_string();
            let data = |name: &str| between(e, &format!("<Data Name='{}'>", name), "</Data>").unwrap_or("").to_string();
            Some(Crash {
                at,
                app: data("AppName"),
                module: data("ModuleName"),
                code: data("ExceptionCode"),
                offset: data("FaultingOffset"),
            })
        })
        .collect()
}

fn between<'a>(s: &'a str, a: &str, b: &str) -> Option<&'a str> {
    let i = s.find(a)? + a.len();
    let j = s[i..].find(b)? + i;
    Some(&s[i..j])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_windows_crash_records() {
        let xml = "<Event xmlns='http://schemas.microsoft.com/win/2004/08/events/event'><System><Provider Name='Application Error'/>\
            <EventID>1000</EventID><TimeCreated SystemTime='2026-11-06T13:26:40.4390000Z'/></System><EventData>\
            <Data Name='AppName'>tidalite-practice(1).exe</Data><Data Name='AppVersion'>0.1.0.0</Data>\
            <Data Name='ModuleName'>VCRUNTIME140.dll</Data><Data Name='ExceptionCode'>c0000005</Data>\
            <Data Name='FaultingOffset'>000000000001cbab</Data></EventData></Event>\
            <Event xmlns='x'><System><TimeCreated SystemTime='2026-11-05T01:02:03Z'/></System><EventData>\
            <Data Name='AppName'>rustc.exe</Data></EventData></Event>";
        let c = parse(xml);
        assert_eq!(c.len(), 2);
        assert_eq!(
            c[0],
            Crash {
                at: "2026-11-06T13:26:40.4390000Z".into(),
                app: "tidalite-practice(1).exe".into(),
                module: "VCRUNTIME140.dll".into(),
                code: "c0000005".into(),
                offset: "000000000001cbab".into(),
            }
        );
        assert_eq!(c[1].app, "rustc.exe");
        assert_eq!(c[1].module, "");
        assert!(parse("").is_empty());
    }
}
