//! Windows notification-area icon. Win32 ties the icon's hidden window to the thread
//! that created it, so the tray lives on its own thread with a message loop, and other
//! threads reach it by posting thread messages.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, PostThreadMessageW, TranslateMessage, MSG, WM_APP,
};

use crate::{daemon, open_ui, toggle_playing, tray_icons};

/// Thread id of the tray's message loop; 0 until it's running.
static THREAD: AtomicU32 = AtomicU32::new(0);
static PLAYING: AtomicBool = AtomicBool::new(false);
const WM_PLAYING_CHANGED: u32 = WM_APP + 1;

pub fn start() {
    let spawned = std::thread::Builder::new().name("tray".into()).spawn(|| {
        if let Err(e) = run() {
            eprintln!("moodistd: tray unavailable: {e}");
        }
    });
    if let Err(e) = spawned {
        eprintln!("moodistd: tray unavailable: {e}");
    }
}

pub fn set_playing(playing: bool) {
    if PLAYING.swap(playing, Ordering::Relaxed) != playing {
        let thread = THREAD.load(Ordering::Acquire);
        if thread != 0 {
            // SAFETY: posting a message with no pointers in it to a thread id we own.
            unsafe { PostThreadMessageW(thread, WM_PLAYING_CHANGED, 0, 0) };
        }
    }
}

fn play_label() -> &'static str {
    if PLAYING.load(Ordering::Relaxed) { "Pause" } else { "Play" }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let open = MenuItem::new("Open Moodist", true, None);
    let play = MenuItem::new(play_label(), true, None);
    let quit = MenuItem::new("Quit", true, None);
    let menu = Menu::with_items(&[&open, &play, &PredefinedMenuItem::separator(), &quit])?;

    let (open_id, play_id, quit_id) = (open.id().clone(), play.id().clone(), quit.id().clone());
    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
        if e.id == open_id {
            open_ui();
        } else if e.id == play_id {
            toggle_playing();
        } else if e.id == quit_id {
            daemon().quit();
        }
    }));
    TrayIconEvent::set_event_handler(Some(|e: TrayIconEvent| {
        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
            open_ui();
        }
    }));

    let (width, height, rgba) = tray_icons().into_iter().next().ok_or("no tray icon image")?;
    let _tray = TrayIconBuilder::new()
        .with_tooltip("Moodist")
        .with_icon(Icon::from_rgba(rgba, width, height)?)
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .build()?;

    // SAFETY: standard Win32 message loop on this thread; MSG is plain data.
    unsafe {
        THREAD.store(GetCurrentThreadId(), Ordering::Release);
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            if msg.message == WM_PLAYING_CHANGED {
                play.set_text(play_label());
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}
