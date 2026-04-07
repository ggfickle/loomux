#[cfg(target_os = "macos")]
pub fn open_session_in_terminal(
    tmux_binary: &str,
    session_name: &str,
    socket_path: Option<&str>,
) -> Result<(), String> {
    use std::process::Command;

    let escaped_session = shell_escape_single_quotes(session_name);
    let escaped_binary = shell_escape_single_quotes(tmux_binary);

    let mut command = format!("'{}'", escaped_binary);
    if let Some(socket_path) = socket_path.map(str::trim).filter(|value| !value.is_empty()) {
        command.push_str(&format!(" -S '{}'", shell_escape_single_quotes(socket_path)));
    }
    command.push_str(&format!(" attach -t '{}'", escaped_session));

    let script = format!(
        "tell application \"Terminal\"\nactivate\ndo script \"{}\"\nend tell",
        applescript_escape(&command)
    );

    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|error| format!("failed to run osascript: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[cfg(target_os = "macos")]
fn shell_escape_single_quotes(value: &str) -> String {
    value.replace('\'', r#"'\''"#)
}

#[cfg(target_os = "macos")]
fn applescript_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(not(target_os = "macos"))]
pub fn open_session_in_terminal(
    _tmux_binary: &str,
    _session_name: &str,
    _socket_path: Option<&str>,
) -> Result<(), String> {
    Err("Opening a tmux session from the desktop app is only supported on macOS. Session listing still works on Linux.".to_string())
}
