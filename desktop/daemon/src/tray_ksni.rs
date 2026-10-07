//! Linux tray icon over D-Bus StatusNotifierItem (e.g. Waybar's tray, KDE, GNOME with
//! the AppIndicator extension). Optional: without a host the daemon still works.

use std::sync::OnceLock;

use ksni::blocking::{Handle, TrayMethods};

use crate::{daemon, open_ui, toggle_playing, tray_icons};

static HANDLE: OnceLock<Handle<Tray>> = OnceLock::new();

struct Tray {
    playing: bool,
    icons: Vec<ksni::Icon>,
}

impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "moodist".into()
    }

    fn title(&self) -> String {
        "Moodist".into()
    }

    fn icon_name(&self) -> String {
        "moodist".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        self.icons.clone()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        open_ui();
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;
        vec![
            StandardItem { label: "Open Moodist".into(), activate: Box::new(|_| open_ui()), ..Default::default() }.into(),
            StandardItem {
                label: if self.playing { "Pause" } else { "Play" }.into(),
                activate: Box::new(|_| toggle_playing()),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem { label: "Quit".into(), activate: Box::new(|_| daemon().quit()), ..Default::default() }.into(),
        ]
    }
}

pub fn start() {
    // SNI wants ARGB32 in network byte order.
    let icons = tray_icons()
        .into_iter()
        .map(|(width, height, rgba)| ksni::Icon {
            width: width as i32,
            height: height as i32,
            data: rgba.chunks_exact(4).flat_map(|p| [p[3], p[0], p[1], p[2]]).collect(),
        })
        .collect();
    match (Tray { playing: false, icons }).spawn() {
        Ok(handle) => {
            let _ = HANDLE.set(handle);
        }
        Err(e) => eprintln!("moodistd: tray unavailable: {e}"),
    }
}

pub fn set_playing(playing: bool) {
    if let Some(tray) = HANDLE.get() {
        tray.update(|t| {
            if t.playing != playing {
                t.playing = playing;
            }
        });
    }
}
