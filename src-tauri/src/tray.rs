//! System tray (macOS menu bar) icon and menu.

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Manager, Wry};

use crate::i18n::{self, Locale};
use crate::window;

const SHOW_ID: &str = "show";
const QUIT_ID: &str = "quit";

/// Handles kept to relabel the tray when the interface language changes.
struct Tray {
    icon: TrayIcon,
    show: MenuItem<Wry>,
    quit: MenuItem<Wry>,
}

pub fn create(app: &AppHandle, locale: Locale) -> tauri::Result<()> {
    let show = MenuItem::with_id(
        app,
        SHOW_ID,
        i18n::t(locale, "tray.show"),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        QUIT_ID,
        i18n::t(locale, "tray.quit"),
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &separator, &quit])?;

    let builder = TrayIconBuilder::with_id("main")
        .icon(tray_icon())
        // macOS tints template images to match the menu bar (light/dark, selected).
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip(i18n::t(locale, "tray.tooltip"))
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            SHOW_ID => window::focus_main(app),
            // Triggers RunEvent::Exit, where window-state saves the window geometry.
            QUIT_ID => app.exit(0),
            _ => {}
        });
    let icon = builder.build(app)?;
    app.manage(Tray { icon, show, quit });
    Ok(())
}

/// Monochrome tray glyph. On macOS it is a black template image the system
/// recolors; elsewhere there is no template support, so a white glyph is used
/// (taskbars are dark by default on Windows and most Linux desktops).
fn tray_icon() -> Image<'static> {
    #[cfg(target_os = "macos")]
    {
        tauri::include_image!("icons/tray-icon-template.png")
    }
    #[cfg(not(target_os = "macos"))]
    {
        tauri::include_image!("icons/tray-icon-light.png")
    }
}

/// Relabels the tray in `locale`.
pub fn set_locale(app: &AppHandle, locale: Locale) -> tauri::Result<()> {
    let Some(tray) = app.try_state::<Tray>() else {
        return Ok(());
    };
    tray.show.set_text(i18n::t(locale, "tray.show"))?;
    tray.quit.set_text(i18n::t(locale, "tray.quit"))?;
    tray.icon.set_tooltip(Some(i18n::t(locale, "tray.tooltip")))
}
