//! Main window lifecycle: restoring size/position across launches (also
//! when monitors were added, removed or rearranged), hiding instead of
//! closing, and focusing the existing window when a second instance is
//! started or the Dock icon is clicked.
//!
//! `tauri-plugin-window-state` saves the geometry; we restore it ourselves
//! so it can be fitted to the monitors connected at launch.

use std::collections::HashMap;

use serde::Deserialize;
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewWindow, Window, WindowEvent,
};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

pub const MAIN: &str = "main";

/// What window-state persists. Visibility and decorations are left to the app.
pub fn state_flags() -> StateFlags {
    StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED | StateFlags::FULLSCREEN
}

/// Axis-aligned rectangle in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    fn right(self) -> i64 {
        i64::from(self.x) + i64::from(self.width)
    }

    fn bottom(self) -> i64 {
        i64::from(self.y) + i64::from(self.height)
    }

    fn overlap_area(self, other: Rect) -> i64 {
        let w = self.right().min(other.right()) - i64::from(self.x.max(other.x));
        let h = self.bottom().min(other.bottom()) - i64::from(self.y.max(other.y));
        w.max(0) * h.max(0)
    }
}

/// Returns where `window` should go so it is fully visible: inside the work
/// area it overlaps the most, or the primary one if it is off every screen
/// (e.g. a monitor was unplugged). The size shrinks if it doesn't fit, e.g.
/// after moving from a large external display to the laptop screen.
/// Returns `None` if the window is already fully visible.
pub fn fit_on_screen(window: Rect, work_areas: &[Rect], primary: Rect) -> Option<Rect> {
    let area = work_areas
        .iter()
        .copied()
        .map(|area| (area.overlap_area(window), area))
        .filter(|(overlap, _)| *overlap > 0)
        .max_by_key(|(overlap, _)| *overlap)
        .map_or(primary, |(_, area)| area);

    let width = window.width.min(area.width);
    let height = window.height.min(area.height);
    let max_x = i64::from(area.x) + i64::from(area.width - width);
    let max_y = i64::from(area.y) + i64::from(area.height - height);
    let x = i64::from(window.x).clamp(i64::from(area.x), max_x) as i32;
    let y = i64::from(window.y).clamp(i64::from(area.y), max_y) as i32;

    let fitted = Rect {
        x,
        y,
        width,
        height,
    };
    (fitted != window).then_some(fitted)
}

fn rect(position: PhysicalPosition<i32>, size: PhysicalSize<u32>) -> Rect {
    Rect {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    }
}

/// Geometry saved by `tauri-plugin-window-state` (physical pixels: outer
/// position, inner size). Only the fields we restore are read.
#[derive(Debug, Deserialize)]
struct SavedWindow {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    prev_x: i32,
    prev_y: i32,
    #[serde(default)]
    maximized: bool,
    #[serde(default)]
    fullscreen: bool,
}

fn saved_state<R: Runtime>(app: &AppHandle<R>) -> Option<SavedWindow> {
    let path = app.path().app_config_dir().ok()?.join(app.filename());
    let file = std::fs::read(path).ok()?;
    let mut windows: HashMap<String, SavedWindow> = serde_json::from_slice(&file).ok()?;
    windows.remove(MAIN).filter(|w| w.width > 0 && w.height > 0)
}

fn apply_saved_state<R: Runtime>(
    window: &WebviewWindow<R>,
    saved: &SavedWindow,
) -> tauri::Result<()> {
    let work_areas: Vec<Rect> = window
        .available_monitors()?
        .iter()
        .map(|m| rect(m.work_area().position, m.work_area().size))
        .collect();
    let primary = window
        .primary_monitor()?
        .map(|m| rect(m.work_area().position, m.work_area().size))
        .or_else(|| work_areas.first().copied());

    // While maximized, x/y hold the monitor's corner; prev_x/prev_y the normal position.
    let (x, y) = if saved.maximized {
        (saved.prev_x, saved.prev_y)
    } else {
        (saved.x, saved.y)
    };
    // The saved size excludes the title bar, but the title bar must fit on screen too.
    let title_bar = title_bar_height(window)?;
    let saved_rect = Rect {
        x,
        y,
        width: saved.width,
        height: saved.height + title_bar,
    };
    let target = primary
        .and_then(|primary| fit_on_screen(saved_rect, &work_areas, primary))
        .unwrap_or(saved_rect);

    window.set_size(PhysicalSize::new(
        target.width,
        target.height.saturating_sub(title_bar),
    ))?;
    window.set_position(PhysicalPosition::new(target.x, target.y))?;
    if saved.maximized {
        window.maximize()?;
    }
    if saved.fullscreen {
        window.set_fullscreen(true)?;
    }
    Ok(())
}

/// Height of the native title bar in physical pixels. A hidden window may
/// report equal inner and outer sizes, so fall back to the standard macOS
/// title bar (28 points).
fn title_bar_height<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<u32> {
    const MACOS_TITLE_BAR_POINTS: f64 = 28.0;
    let measured = window
        .outer_size()?
        .height
        .saturating_sub(window.inner_size()?.height);
    Ok(if measured > 0 {
        measured
    } else {
        (MACOS_TITLE_BAR_POINTS * window.scale_factor()?).round() as u32
    })
}

/// Restores the saved geometry, fitted to the monitors connected now, then
/// shows the window. The window starts hidden (`visible: false` in
/// tauri.conf.json) to avoid a jump on launch.
///
/// The geometry is computed from the saved state rather than read back from
/// the window: on macOS a hidden window still reports its old position right
/// after `set_position`.
pub fn restore_and_show<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(MAIN) else {
        return Ok(());
    };
    let restored = match saved_state(app) {
        Some(saved) => apply_saved_state(&window, &saved),
        None => Ok(()), // First launch: keep the default size, centered.
    };
    // Show the window even if restoring failed: a default-placed window beats an invisible app.
    window.show()?;
    window.set_focus()?;
    restored
}

/// Closing the main window hides it instead: Esperanto keeps running in the
/// menu bar (tray) until the user quits from the tray or with ⌘Q.
pub fn on_window_event<R: Runtime>(window: &Window<R>, event: &WindowEvent) {
    if window.label() == MAIN
        && let WindowEvent::CloseRequested { api, .. } = event
    {
        api.prevent_close();
        if let Err(error) = window.hide() {
            log::error!("could not hide the main window: {error}");
        }
    }
}

/// WebKit runs the interface in a separate process. If macOS terminates it
/// (it happens occasionally at launch, or under memory pressure), the window
/// goes blank and stays blank: reload the interface. At most a few times in
/// a row, so a page that crashes on load can't loop forever.
pub fn on_web_content_terminated<R: Runtime>(webview: &tauri::Webview<R>) {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    const MAX_RELOADS: u32 = 3;
    const WINDOW_SECS: u64 = 60;
    static RELOADS: AtomicU32 = AtomicU32::new(0);
    static WINDOW_START: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    if now.saturating_sub(WINDOW_START.load(Ordering::Relaxed)) > WINDOW_SECS {
        WINDOW_START.store(now, Ordering::Relaxed);
        RELOADS.store(0, Ordering::Relaxed);
    }
    if RELOADS.fetch_add(1, Ordering::Relaxed) >= MAX_RELOADS {
        log::error!("the interface process keeps terminating; not reloading again");
        return;
    }
    log::warn!("the interface process terminated; reloading the interface");
    // The reloaded page can't receive the running session's events or stop
    // it (its state starts idle): stop it now.
    crate::session::stop_running(webview.app_handle());
    if let Err(error) = webview.reload() {
        log::error!("could not reload the interface: {error}");
    }
}

/// Brings the main window to the front (used when a second instance starts).
pub fn focus_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAPTOP: Rect = Rect {
        x: 0,
        y: 25,
        width: 1512,
        height: 920,
    };
    const EXTERNAL: Rect = Rect {
        x: 1512,
        y: -300,
        width: 2560,
        height: 1415,
    };

    fn r(x: i32, y: i32, width: u32, height: u32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn visible_window_is_left_alone() {
        assert_eq!(
            fit_on_screen(r(100, 100, 960, 640), &[LAPTOP, EXTERNAL], LAPTOP),
            None
        );
        assert_eq!(
            fit_on_screen(r(2000, 0, 960, 640), &[LAPTOP, EXTERNAL], LAPTOP),
            None
        );
    }

    #[test]
    fn window_on_unplugged_monitor_moves_to_primary() {
        let fitted = fit_on_screen(r(2000, 0, 960, 640), &[LAPTOP], LAPTOP).unwrap();
        assert_eq!(fitted, r(552, 25, 960, 640));
    }

    #[test]
    fn oversized_window_shrinks_to_work_area() {
        let fitted = fit_on_screen(r(0, 25, 2400, 1300), &[LAPTOP], LAPTOP).unwrap();
        assert_eq!(fitted, LAPTOP);
    }

    #[test]
    fn partly_hidden_window_is_pulled_into_the_monitor_it_overlaps_most() {
        // Mostly on the external monitor, title bar above its top edge.
        let fitted = fit_on_screen(r(2000, -400, 960, 640), &[LAPTOP, EXTERNAL], LAPTOP).unwrap();
        assert_eq!(fitted, r(2000, -300, 960, 640));
        // Hanging off the laptop's left edge.
        let fitted = fit_on_screen(r(-500, 100, 960, 640), &[LAPTOP, EXTERNAL], LAPTOP).unwrap();
        assert_eq!(fitted, r(0, 100, 960, 640));
    }
}
