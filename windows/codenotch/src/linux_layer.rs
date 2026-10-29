use gtk::prelude::*;
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager, WebviewWindow};

static ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

pub fn wants_native() -> bool {
    if std::env::var_os("CODENOTCH_X11").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_none() {
        return false;
    }
    !std::env::var("XDG_CURRENT_DESKTOP").is_ok_and(|d| d.to_ascii_uppercase().contains("GNOME"))
}

fn on_wayland() -> bool {
    gtk::gdk::Display::default().is_some_and(|d| d.type_().name() == "GdkWaylandDisplay")
}

pub fn install(window: &WebviewWindow) -> tauri::Result<()> {
    if !on_wayland() {
        return Ok(());
    }
    if !gtk_layer_shell::is_supported() {
        crate::applog("layer-shell: not offered by this compositor; the notch cannot sit on an edge");
        return Ok(());
    }
    let gtk_window = window.gtk_window()?;
    if gtk_window.is_realized() {
        gtk_window.unrealize();
    }
    gtk_window.init_layer_shell();
    gtk_window.set_namespace("codenotch");
    gtk_window.set_layer(Layer::Overlay);
    gtk_window.set_keyboard_mode(KeyboardMode::None);
    gtk_window.set_exclusive_zone(-1);
    gtk_window.set_anchor(Edge::Left, true);
    gtk_window.set_anchor(Edge::Top, true);
    ACTIVE.store(true, Ordering::Relaxed);
    crate::applog("layer-shell: notch is a layer surface");
    Ok(())
}

pub fn place(app: &AppHandle, label: &'static str, x: i32, y: i32) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(window) = handle.get_webview_window(label) else { return };
        let Ok(gtk_window) = window.gtk_window() else { return };
        if !gtk_window.is_layer_window() {
            return;
        }
        let Some(display) = gtk::gdk::Display::default() else { return };
        let monitors = (0..display.n_monitors()).filter_map(|i| display.monitor(i));
        let mut nearest: Option<(gtk::gdk::Monitor, i64)> = None;
        for monitor in monitors {
            let g = monitor.geometry();
            let s = monitor.scale_factor().max(1);
            let (left, top) = (g.x() * s, g.y() * s);
            let (right, bottom) = (left + g.width() * s, top + g.height() * s);
            let dx = if x < left { left - x } else if x >= right { x - right + 1 } else { 0 } as i64;
            let dy = if y < top { top - y } else if y >= bottom { y - bottom + 1 } else { 0 } as i64;
            let distance = dx * dx + dy * dy;
            if nearest.as_ref().is_none_or(|(_, best)| distance < *best) {
                nearest = Some((monitor, distance));
            }
        }
        let Some((monitor, _)) = nearest else { return };
        let g = monitor.geometry();
        let s = monitor.scale_factor().max(1);
        gtk_window.set_monitor(&monitor);
        gtk_window.set_layer_shell_margin(Edge::Left, (x - g.x() * s) / s);
        gtk_window.set_layer_shell_margin(Edge::Top, (y - g.y() * s) / s);
    });
}
