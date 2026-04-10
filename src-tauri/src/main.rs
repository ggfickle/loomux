mod macos;
mod settings;
mod tmux;

use settings::TerminalSettings;
use tauri::image::Image;
use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Runtime, WindowEvent};
use tmux::TmuxOverview;

const TRAY_ID: &str = "loomux-tray";
const MENU_OPEN_WINDOW: &str = "tray.open-window";
const MENU_REFRESH: &str = "tray.refresh";
const MENU_QUIT: &str = "tray.quit";
const MENU_EMPTY: &str = "tray.empty";
const MENU_ERROR: &str = "tray.error";
const MENU_SESSION_PREFIX: &str = "tray.session.";

#[derive(Debug, PartialEq, Eq)]
enum TrayMenuEntry {
    Item {
        id: String,
        text: String,
        enabled: bool,
    },
    Separator,
}

#[tauri::command]
fn get_tmux_overview(app: AppHandle) -> Result<TmuxOverview, String> {
    let overview = tmux::get_overview()?;
    let _ = refresh_tray_menu(&app, Some(&overview));
    Ok(overview)
}

#[tauri::command]
fn get_terminal_settings(app: AppHandle) -> Result<TerminalSettings, String> {
    settings::load(&app)
}

#[tauri::command]
fn update_terminal_settings(
    app: AppHandle,
    settings: TerminalSettings,
) -> Result<TerminalSettings, String> {
    let settings = settings::save(&app, settings)?;
    let _ = refresh_tray_menu(&app, None);
    Ok(settings)
}

#[tauri::command]
fn create_tmux_session(app: AppHandle, session_name: String) -> Result<(), String> {
    tmux::create_session(&session_name)?;
    let _ = refresh_tray_menu(&app, None);
    Ok(())
}

#[tauri::command]
fn rename_tmux_session(
    app: AppHandle,
    old_name: String,
    new_name: String,
    socket_path: Option<String>,
) -> Result<(), String> {
    tmux::rename_session(&old_name, &new_name, socket_path.as_deref())?;
    let _ = refresh_tray_menu(&app, None);
    Ok(())
}

#[tauri::command]
fn delete_tmux_session(
    app: AppHandle,
    session_name: String,
    socket_path: Option<String>,
) -> Result<(), String> {
    tmux::delete_session(&session_name, socket_path.as_deref())?;
    let _ = refresh_tray_menu(&app, None);
    Ok(())
}

#[tauri::command]
fn open_tmux_session(
    app: AppHandle,
    session_name: String,
    socket_path: Option<String>,
) -> Result<(), String> {
    open_tmux_session_impl(&app, &session_name, socket_path.as_deref())
}

fn open_tmux_session_impl(
    app: &AppHandle,
    session_name: &str,
    socket_path: Option<&str>,
) -> Result<(), String> {
    let (tmux_binary, target) = tmux::tmux_command_target_for_session(session_name, socket_path)?;
    let terminal_settings = settings::load(app).unwrap_or_default();
    let result = macos::open_session_in_terminal(
        &terminal_settings,
        &tmux_binary,
        session_name,
        target.socket_path.as_deref(),
    );
    let _ = refresh_tray_menu(app, None);
    result
}

fn main() {
    let app = tauri::Builder::default()
        .setup(|app| {
            create_tray(app.handle())?;
            let _ = refresh_tray_menu(app.handle(), None);
            Ok(())
        })
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            match id {
                MENU_OPEN_WINDOW => {
                    let _ = show_main_window(app);
                }
                MENU_REFRESH => {
                    let _ = refresh_tray_menu(app, None);
                }
                MENU_QUIT => {
                    app.exit(0);
                }
                _ if id.starts_with(MENU_SESSION_PREFIX) => {
                    let session_name = &id[MENU_SESSION_PREFIX.len()..];
                    let _ = open_tmux_session_impl(app, session_name, None);
                }
                _ => {}
            }
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }

            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_tmux_overview,
            get_terminal_settings,
            update_terminal_settings,
            create_tmux_session,
            rename_tmux_session,
            delete_tmux_session,
            open_tmux_session
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app, event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen {
            has_visible_windows,
            ..
        } = event
        {
            if should_restore_main_window_on_reopen(has_visible_windows) {
                let _ = show_main_window(app);
            }
        }
    });
}

fn create_tray<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let menu = build_tray_menu(app, None, Some("Loading tmux sessions…"))?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("Loomux")
        .show_menu_on_left_click(true);

    if let Some(icon) = tray_icon_image(app) {
        builder = builder.icon(icon);
    }

    #[cfg(target_os = "macos")]
    {
        builder = builder.icon_as_template(true);
    }

    builder
        .build(app)
        .map(|_| ())
        .map_err(|error| format!("failed to create tray icon: {error}"))
}

fn refresh_tray_menu<R: Runtime>(
    app: &AppHandle<R>,
    overview: Option<&TmuxOverview>,
) -> Result<(), String> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };

    let (owned_overview, error_message) = match overview {
        Some(_) => (None, None),
        None => match tmux::get_overview() {
            Ok(fresh_overview) => (Some(fresh_overview), None),
            Err(error) => (None, Some(error)),
        },
    };

    let overview = overview.or(owned_overview.as_ref());
    let menu = build_tray_menu(app, overview, error_message.as_deref())?;
    tray.set_menu(Some(menu))
        .map_err(|error| format!("failed to update tray menu: {error}"))
}

fn build_tray_menu<R: Runtime>(
    app: &AppHandle<R>,
    overview: Option<&TmuxOverview>,
    error_message: Option<&str>,
) -> Result<Menu<R>, String> {
    let mut builder = MenuBuilder::new(app);

    for entry in tray_menu_entries(overview, error_message) {
        match entry {
            TrayMenuEntry::Item { id, text, enabled } => {
                let item = MenuItemBuilder::with_id(id, text)
                    .enabled(enabled)
                    .build(app)
                    .map_err(|error| format!("failed to build tray menu item: {error}"))?;
                builder = builder.item(&item);
            }
            TrayMenuEntry::Separator => {
                builder = builder.separator();
            }
        }
    }

    builder
        .build()
        .map_err(|error| format!("failed to build tray menu: {error}"))
}

fn tray_menu_entries(
    overview: Option<&TmuxOverview>,
    error_message: Option<&str>,
) -> Vec<TrayMenuEntry> {
    let mut entries = vec![
        TrayMenuEntry::Item {
            id: MENU_OPEN_WINDOW.to_string(),
            text: "Open Loomux".to_string(),
            enabled: true,
        },
        TrayMenuEntry::Separator,
    ];

    match overview {
        Some(overview) if !overview.sessions.is_empty() => {
            for session in &overview.sessions {
                entries.push(TrayMenuEntry::Item {
                    id: format!("{MENU_SESSION_PREFIX}{}", session.name),
                    text: session.name.clone(),
                    enabled: true,
                });
            }
        }
        Some(_) => {
            entries.push(TrayMenuEntry::Item {
                id: MENU_EMPTY.to_string(),
                text: "No tmux sessions".to_string(),
                enabled: false,
            });
        }
        None => {
            entries.push(TrayMenuEntry::Item {
                id: MENU_ERROR.to_string(),
                text: error_message
                    .unwrap_or("Unable to load tmux sessions")
                    .to_string(),
                enabled: false,
            });
        }
    }

    entries.push(TrayMenuEntry::Separator);
    entries.push(TrayMenuEntry::Item {
        id: MENU_REFRESH.to_string(),
        text: "Refresh Sessions".to_string(),
        enabled: true,
    });
    entries.push(TrayMenuEntry::Item {
        id: MENU_QUIT.to_string(),
        text: "Quit Loomux".to_string(),
        enabled: true,
    });
    entries
}

#[cfg(target_os = "macos")]
fn tray_icon_image<R: Runtime>(_app: &AppHandle<R>) -> Option<Image<'static>> {
    Some(macos_template_tray_icon())
}

#[cfg(not(target_os = "macos"))]
fn tray_icon_image<R: Runtime>(app: &AppHandle<R>) -> Option<Image<'static>> {
    app.default_window_icon().cloned()
}

#[cfg(target_os = "macos")]
fn macos_template_tray_icon() -> Image<'static> {
    const SIZE: u32 = 18;

    let mut rgba = vec![0; (SIZE * SIZE * 4) as usize];
    paint_rounded_bar(&mut rgba, SIZE, 4, 6, 4, 7);
    paint_rounded_bar(&mut rgba, SIZE, 10, 4, 4, 10);

    Image::new_owned(rgba, SIZE, SIZE)
}

#[cfg(target_os = "macos")]
fn paint_rounded_bar(rgba: &mut [u8], image_width: u32, x: u32, y: u32, width: u32, height: u32) {
    fill_rect(rgba, image_width, x, y + 1, width, height.saturating_sub(2));
    fill_rect(rgba, image_width, x + 1, y, width.saturating_sub(2), height);
}

#[cfg(target_os = "macos")]
fn fill_rect(rgba: &mut [u8], image_width: u32, x: u32, y: u32, width: u32, height: u32) {
    if width == 0 || height == 0 {
        return;
    }

    for row in y..y + height {
        for col in x..x + width {
            let index = ((row * image_width + col) * 4) as usize;
            rgba[index + 3] = u8::MAX;
        }
    }
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Err("main window not found".to_string());
    };

    let _ = window.unminimize();
    window
        .show()
        .map_err(|error| format!("failed to show main window: {error}"))?;
    window
        .set_focus()
        .map_err(|error| format!("failed to focus main window: {error}"))
}

fn should_restore_main_window_on_reopen(has_visible_windows: bool) -> bool {
    !has_visible_windows
}

#[cfg(test)]
mod tests {
    use super::{
        should_restore_main_window_on_reopen, tray_menu_entries, TmuxOverview, TrayMenuEntry,
        MENU_EMPTY, MENU_ERROR, MENU_OPEN_WINDOW, MENU_QUIT, MENU_REFRESH, MENU_SESSION_PREFIX,
    };
    use crate::tmux::TmuxSession;

    #[test]
    fn reopen_restores_window_when_none_are_visible() {
        assert!(should_restore_main_window_on_reopen(false));
    }

    #[test]
    fn reopen_skips_restore_when_a_window_is_already_visible() {
        assert!(!should_restore_main_window_on_reopen(true));
    }

    #[test]
    fn tray_menu_prioritizes_sessions_and_hides_diagnostics() {
        let overview = TmuxOverview {
            session_count: 2,
            tmux_process_count: 1,
            tmux_binary_path: "/opt/homebrew/bin/tmux".to_string(),
            primary_socket_path: Some("/tmp/tmux-1000/default".to_string()),
            session_detection: "test".to_string(),
            debug_notes: Vec::new(),
            sessions: vec![
                TmuxSession {
                    name: "alpha".to_string(),
                    attached: false,
                    windows: 3,
                    socket_path: Some("/tmp/tmux-1000/default".to_string()),
                },
                TmuxSession {
                    name: "beta".to_string(),
                    attached: true,
                    windows: 1,
                    socket_path: Some("/tmp/tmux-1000/default".to_string()),
                },
            ],
        };

        assert_eq!(
            tray_menu_entries(Some(&overview), None),
            vec![
                TrayMenuEntry::Item {
                    id: MENU_OPEN_WINDOW.to_string(),
                    text: "Open Loomux".to_string(),
                    enabled: true,
                },
                TrayMenuEntry::Separator,
                TrayMenuEntry::Item {
                    id: format!("{MENU_SESSION_PREFIX}alpha"),
                    text: "alpha".to_string(),
                    enabled: true,
                },
                TrayMenuEntry::Item {
                    id: format!("{MENU_SESSION_PREFIX}beta"),
                    text: "beta".to_string(),
                    enabled: true,
                },
                TrayMenuEntry::Separator,
                TrayMenuEntry::Item {
                    id: MENU_REFRESH.to_string(),
                    text: "Refresh Sessions".to_string(),
                    enabled: true,
                },
                TrayMenuEntry::Item {
                    id: MENU_QUIT.to_string(),
                    text: "Quit Loomux".to_string(),
                    enabled: true,
                },
            ]
        );
    }

    #[test]
    fn tray_menu_uses_compact_empty_state() {
        let overview = TmuxOverview {
            session_count: 0,
            tmux_process_count: 1,
            tmux_binary_path: "/opt/homebrew/bin/tmux".to_string(),
            primary_socket_path: Some("/tmp/tmux-1000/default".to_string()),
            session_detection: "test".to_string(),
            debug_notes: Vec::new(),
            sessions: Vec::new(),
        };

        assert_eq!(
            tray_menu_entries(Some(&overview), None),
            vec![
                TrayMenuEntry::Item {
                    id: MENU_OPEN_WINDOW.to_string(),
                    text: "Open Loomux".to_string(),
                    enabled: true,
                },
                TrayMenuEntry::Separator,
                TrayMenuEntry::Item {
                    id: MENU_EMPTY.to_string(),
                    text: "No tmux sessions".to_string(),
                    enabled: false,
                },
                TrayMenuEntry::Separator,
                TrayMenuEntry::Item {
                    id: MENU_REFRESH.to_string(),
                    text: "Refresh Sessions".to_string(),
                    enabled: true,
                },
                TrayMenuEntry::Item {
                    id: MENU_QUIT.to_string(),
                    text: "Quit Loomux".to_string(),
                    enabled: true,
                },
            ]
        );
    }

    #[test]
    fn tray_menu_uses_compact_error_state() {
        assert_eq!(
            tray_menu_entries(None, Some("Loading tmux sessions…")),
            vec![
                TrayMenuEntry::Item {
                    id: MENU_OPEN_WINDOW.to_string(),
                    text: "Open Loomux".to_string(),
                    enabled: true,
                },
                TrayMenuEntry::Separator,
                TrayMenuEntry::Item {
                    id: MENU_ERROR.to_string(),
                    text: "Loading tmux sessions…".to_string(),
                    enabled: false,
                },
                TrayMenuEntry::Separator,
                TrayMenuEntry::Item {
                    id: MENU_REFRESH.to_string(),
                    text: "Refresh Sessions".to_string(),
                    enabled: true,
                },
                TrayMenuEntry::Item {
                    id: MENU_QUIT.to_string(),
                    text: "Quit Loomux".to_string(),
                    enabled: true,
                },
            ]
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_tray_icon_uses_two_bars_with_transparent_background() {
        let icon = super::macos_template_tray_icon();
        let rgba = icon.rgba();

        assert_eq!(icon.width(), 18);
        assert_eq!(icon.height(), 18);
        assert_eq!(alpha_at(rgba, 18, 0, 0), 0);
        assert_eq!(alpha_at(rgba, 18, 5, 8), u8::MAX);
        assert_eq!(alpha_at(rgba, 18, 11, 8), u8::MAX);
        assert_eq!(alpha_at(rgba, 18, 8, 8), 0);
        assert_eq!(alpha_at(rgba, 18, 4, 6), 0);
        assert_eq!(alpha_at(rgba, 18, 5, 6), u8::MAX);
    }

    #[cfg(target_os = "macos")]
    fn alpha_at(rgba: &[u8], width: u32, x: u32, y: u32) -> u8 {
        let index = ((y * width + x) * 4 + 3) as usize;
        rgba[index]
    }
}
