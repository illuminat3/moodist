// No console window behind the app in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Inside an AppImage the daemon is launched through the image itself (so it gets
    // its own mount); hand over to the bundled moodistd.
    #[cfg(target_os = "linux")]
    if std::env::args().nth(1).as_deref() == Some("--daemon") {
        use std::os::unix::process::CommandExt;
        let daemon = std::env::current_exe().ok().and_then(|p| Some(p.parent()?.join("moodistd")));
        let err = std::process::Command::new(daemon.unwrap_or_else(|| "moodistd".into())).args(std::env::args_os().skip(2)).exec();
        eprintln!("moodist: could not start moodistd: {err}");
        std::process::exit(1);
    }

    // WebKitGTK's DMA-BUF renderer crashes on NVIDIA under Wayland ("Gdk Error 71").
    // The fallback path is just as fast for a UI like this one.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // SAFETY: single-threaded at this point; nothing else reads the environment yet.
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }
    moodist_lib::run();
}
