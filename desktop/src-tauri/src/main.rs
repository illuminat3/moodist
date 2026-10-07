fn main() {
    // WebKitGTK's DMA-BUF renderer crashes on NVIDIA under Wayland ("Gdk Error 71").
    // The fallback path is just as fast for a UI like this one.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // SAFETY: single-threaded at this point; nothing else reads the environment yet.
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }
    moodist_lib::run();
}
