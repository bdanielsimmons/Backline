//! The iPhone app (built into Backline.app by .github/workflows/ios.yml).
fn main() {
    #[cfg(target_os = "ios")]
    backline_phone::run_ios();
}
