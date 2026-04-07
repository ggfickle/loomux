mod macos;
mod settings;
mod tmux;

use settings::TerminalSettings;
use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime, WindowEvent};
use tmux::TmuxOverview;

const TRAY_ID: &str = "loomux-tray";
const MENU_OPEN_WINDOW: &str = "tray.open-window";
const MENU_REFRESH: &str = "tray.refresh";
const MENU_QUIT: &str = "tray.quit";
const MENU_EMPTY: &str = "tray.empty";
const MENU_STATUS: &str = "tray.status";
const MENU_OPEN_WITH: &str = "tray.open-with";
const MENU_ERROR: &str = "tray.error";
const MENU_SESSION_PREFIX: &str = "tray.session.";

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
fn update_terminal_settings(app: AppHandle, settings: TerminalSettings) -> Result<TerminalSettings, String> {
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
    tauri::Builder::default()
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn create_tray<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let menu = build_tray_menu(app, None, Some("Loading tmux sessions…"))?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("Loomux")
        .show_menu_on_left_click(true)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick { .. } = event {
                let _ = show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon).icon_as_template(true);
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
    let mut builder = MenuBuilder::new(app)
        .text(MENU_OPEN_WINDOW, "Open Loomux")
        .text(MENU_REFRESH, "Refresh Sessions")
        .separator();

    let terminal_settings = settings::load(app).unwrap_or_default();
    let open_with = MenuItemBuilder::with_id(
        MENU_OPEN_WITH,
        format!("Open with: {}", terminal_settings.preferred_terminal_label()),
    )
    .enabled(false)
    .build(app)
    .map_err(|error| format!("failed to build tray terminal item: {error}"))?;
    builder = builder.item(&open_with).separator();

    match overview {
        Some(overview) if !overview.sessions.is_empty() => {
            if let Some(socket_path) = &overview.primary_socket_path {
                let status = MenuItemBuilder::with_id(MENU_STATUS, format!("Socket: {socket_path}"))
                    .enabled(false)
                    .build(app)
                    .map_err(|error| format!("failed to build tray status item: {error}"))?;
                builder = builder.item(&status).separator();
            }

            for session in &overview.sessions {
                let label = if session.attached {
                    format!("{} · {} windows · attached", session.name, session.windows)
                } else {
                    format!("{} · {} windows", session.name, session.windows)
                };
                builder = builder.text(format!("{MENU_SESSION_PREFIX}{}", session.name), label);
            }
        }
        Some(overview) => {
            let empty_text = if let Some(socket_path) = &overview.primary_socket_path {
                format!("No tmux sessions found (socket: {socket_path})")
            } else {
                "No tmux sessions found".to_string()
            };
            let item = MenuItemBuilder::with_id(MENU_EMPTY, empty_text)
                .enabled(false)
                .build(app)
                .map_err(|error| format!("failed to build tray empty item: {error}"))?;
            builder = builder.item(&item);
        }
        None => {
            let item = MenuItemBuilder::with_id(
                MENU_ERROR,
                error_message.unwrap_or("Unable to load tmux sessions"),
            )
            .enabled(false)
            .build(app)
            .map_err(|error| format!("failed to build tray error item: {error}"))?;
            builder = builder.item(&item);
        }
    }

    builder
        .separator()
        .text(MENU_QUIT, "Quit Loomux")
        .build()
        .map_err(|error| format!("failed to build tray menu: {error}"))
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
