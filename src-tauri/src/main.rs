mod macos;
mod tmux;

use tmux::TmuxOverview;

#[tauri::command]
fn get_tmux_overview() -> Result<TmuxOverview, String> {
    tmux::get_overview()
}

#[tauri::command]
fn create_tmux_session(session_name: String) -> Result<(), String> {
    tmux::create_session(&session_name)
}

#[tauri::command]
fn rename_tmux_session(old_name: String, new_name: String) -> Result<(), String> {
    tmux::rename_session(&old_name, &new_name)
}

#[tauri::command]
fn delete_tmux_session(session_name: String) -> Result<(), String> {
    tmux::delete_session(&session_name)
}

#[tauri::command]
fn open_tmux_session(session_name: String) -> Result<(), String> {
    if session_name.trim().is_empty() {
        return Err("session name cannot be empty".to_string());
    }

    macos::open_session_in_terminal(&session_name)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_tmux_overview,
            create_tmux_session,
            rename_tmux_session,
            delete_tmux_session,
            open_tmux_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
