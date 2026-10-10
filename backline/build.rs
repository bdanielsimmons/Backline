// Puts icon.ico inside the Windows .exe, so the taskbar, pinned shortcuts and Explorer show the logo
// (the window icon set at runtime is not enough: a pinned shortcut reads the icon from the file).
fn main() {
    println!("cargo:rerun-if-changed=icon.ico");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("icon.ico");
        // the name Windows shows for the program (the media flyout, Task Manager, the file's properties)
        res.set("FileDescription", "Backline");
        res.set("ProductName", "Backline");
        res.set("InternalName", "backline");
        res.set("OriginalFilename", "backline.exe");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed the icon: {}", e);
        }
    }
}
