#[cfg(target_os = "macos")]
use crate::settings::TerminalPreference;
use crate::settings::TerminalSettings;

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalTarget {
    Terminal,
    Iterm,
    Ghostty,
    Tabby,
    Custom,
}

pub fn open_session_in_terminal(
    settings: &TerminalSettings,
    tmux_binary: &str,
    session_name: &str,
    socket_path: Option<&str>,
) -> Result<(), String> {
    open_session_impl(settings, tmux_binary, session_name, socket_path)
}

#[cfg_attr(not(any(target_os = "macos", test)), allow(dead_code))]
fn build_tmux_attach_command(
    tmux_binary: &str,
    session_name: &str,
    socket_path: Option<&str>,
) -> String {
    let escaped_session = shell_escape_single_quotes(session_name);
    let escaped_binary = shell_escape_single_quotes(tmux_binary);

    let mut command = format!("'{}'", escaped_binary);
    if let Some(socket_path) = socket_path.map(str::trim).filter(|value| !value.is_empty()) {
        command.push_str(&format!(
            " -S '{}'",
            shell_escape_single_quotes(socket_path)
        ));
    }
    command.push_str(&format!(" attach -t '{}'", escaped_session));
    command
}

#[cfg_attr(not(any(target_os = "macos", test)), allow(dead_code))]
fn render_custom_command(
    template: &str,
    tmux_binary: &str,
    session_name: &str,
    socket_path: Option<&str>,
) -> String {
    let command = build_tmux_attach_command(tmux_binary, session_name, socket_path);
    let tmux_binary = format!("'{}'", shell_escape_single_quotes(tmux_binary));
    let session_name = format!("'{}'", shell_escape_single_quotes(session_name));
    let socket_arg = socket_path
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("-S '{}'", shell_escape_single_quotes(value)))
        .unwrap_or_default();

    let mut rendered = template
        .replace("{{command}}", &command)
        .replace("{{tmux_binary}}", &tmux_binary)
        .replace("{{socket_arg}}", &socket_arg)
        .replace("{{session_name}}", &session_name);

    if rendered == template {
        rendered.push(' ');
        rendered.push_str(&command);
    }

    rendered
}

#[cfg_attr(not(any(target_os = "macos", test)), allow(dead_code))]
fn shell_escape_single_quotes(value: &str) -> String {
    value.replace('\'', r#"'\''"#)
}

#[cfg(target_os = "macos")]
fn open_session_impl(
    settings: &TerminalSettings,
    tmux_binary: &str,
    session_name: &str,
    socket_path: Option<&str>,
) -> Result<(), String> {
    let attach_command = build_tmux_attach_command(tmux_binary, session_name, socket_path);

    match resolve_terminal_target(settings)? {
        TerminalTarget::Terminal => open_in_terminal(&attach_command),
        TerminalTarget::Iterm => open_in_iterm(&attach_command),
        TerminalTarget::Ghostty => open_in_ghostty(&attach_command),
        TerminalTarget::Tabby => open_in_tabby(&attach_command),
        TerminalTarget::Custom => {
            open_with_custom_command(settings, tmux_binary, session_name, socket_path)
        }
    }
}

#[cfg(target_os = "macos")]
fn resolve_terminal_target(settings: &TerminalSettings) -> Result<TerminalTarget, String> {
    match settings.preferred_terminal {
        TerminalPreference::Auto => {
            for target in [
                TerminalTarget::Ghostty,
                TerminalTarget::Iterm,
                TerminalTarget::Tabby,
                TerminalTarget::Terminal,
            ] {
                if terminal_target_available(target) {
                    return Ok(target);
                }
            }

            Ok(TerminalTarget::Terminal)
        }
        TerminalPreference::Terminal => ensure_terminal_target(TerminalTarget::Terminal),
        TerminalPreference::Iterm => ensure_terminal_target(TerminalTarget::Iterm),
        TerminalPreference::Ghostty => ensure_terminal_target(TerminalTarget::Ghostty),
        TerminalPreference::Tabby => ensure_terminal_target(TerminalTarget::Tabby),
        TerminalPreference::Custom => Ok(TerminalTarget::Custom),
    }
}

#[cfg(target_os = "macos")]
fn ensure_terminal_target(target: TerminalTarget) -> Result<TerminalTarget, String> {
    if terminal_target_available(target) {
        Ok(target)
    } else {
        Err(format!(
            "{} is not available on this Mac. Choose another terminal or use Custom command.",
            terminal_target_label(target)
        ))
    }
}

#[cfg(target_os = "macos")]
fn terminal_target_available(target: TerminalTarget) -> bool {
    match target {
        TerminalTarget::Terminal => resolve_application_bundle(&["Terminal"]).is_ok(),
        TerminalTarget::Iterm => resolve_application_bundle(&["iTerm", "iTerm2"]).is_ok(),
        TerminalTarget::Ghostty => resolve_application_bundle(&["Ghostty"]).is_ok(),
        TerminalTarget::Tabby => resolve_application_bundle(&["Tabby"]).is_ok(),
        TerminalTarget::Custom => true,
    }
}

#[cfg(target_os = "macos")]
fn terminal_target_label(target: TerminalTarget) -> &'static str {
    match target {
        TerminalTarget::Terminal => "Terminal",
        TerminalTarget::Iterm => "iTerm",
        TerminalTarget::Ghostty => "Ghostty",
        TerminalTarget::Tabby => "Tabby",
        TerminalTarget::Custom => "Custom command",
    }
}

#[cfg(target_os = "macos")]
fn open_in_terminal(command: &str) -> Result<(), String> {
    run_osascript(&format!(
        "tell application \"Terminal\"\nactivate\ndo script \"{}\"\nend tell",
        applescript_escape(command)
    ))
}

#[cfg(target_os = "macos")]
fn open_in_iterm(command: &str) -> Result<(), String> {
    run_osascript(&format!(
        "tell application \"iTerm2\"\nactivate\ncreate window with default profile command \"{}\"\nend tell",
        applescript_escape(command)
    ))
}

#[cfg(target_os = "macos")]
fn open_in_ghostty(command: &str) -> Result<(), String> {
    // Ghostty on macOS doesn't support CLI launching, use open command instead
    run_osascript(&format!("tell application \"Ghostty\"\nactivate\nend tell"))?;

    // Give Ghostty time to start, then send the command via AppleScript
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Use Terminal.app style command execution for Ghostty
    let script = format!(
        "tell application \"System Events\"\ntell process \"Ghostty\"\nkeystroke \"{}\"\nkeystroke return\nend tell\nend tell",
        applescript_escape(command)
    );
    run_osascript(&script)
}

#[cfg(target_os = "macos")]
fn open_in_tabby(command: &str) -> Result<(), String> {
    let executable = resolve_application_executable(&["Tabby"])?;
    // Tabby's "run" command expects the full command as a single argument
    spawn_detached(&executable, &["run", command])
}

#[cfg(target_os = "macos")]
fn open_with_custom_command(
    settings: &TerminalSettings,
    tmux_binary: &str,
    session_name: &str,
    socket_path: Option<&str>,
) -> Result<(), String> {
    let template = settings.custom_command.trim();
    if template.is_empty() {
        return Err(
            "Custom command is empty. Add a command or choose another terminal option.".to_string(),
        );
    }

    let rendered = render_custom_command(template, tmux_binary, session_name, socket_path);
    spawn_detached(std::path::Path::new("/bin/sh"), &["-lc", rendered.as_str()])
}

#[cfg(target_os = "macos")]
fn run_osascript(script: &str) -> Result<(), String> {
    use std::process::Command;

    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|error| format!("failed to run osascript: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            Err("osascript failed".to_string())
        } else {
            Err(stderr)
        }
    }
}

#[cfg(target_os = "macos")]
fn resolve_application_bundle(app_names: &[&str]) -> Result<std::path::PathBuf, String> {
    use std::path::PathBuf;
    use std::process::Command;

    for app_name in app_names {
        let script = format!(
            "POSIX path of (path to application \"{}\")",
            applescript_escape(app_name)
        );
        let output = Command::new("osascript").arg("-e").arg(script).output();

        let Ok(output) = output else {
            continue;
        };
        if !output.status.success() {
            continue;
        }

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !stdout.is_empty() {
            return Ok(PathBuf::from(stdout));
        }
    }

    Err(format!("Application not found: {}", app_names.join(", ")))
}

#[cfg(target_os = "macos")]
fn resolve_application_executable(app_names: &[&str]) -> Result<std::path::PathBuf, String> {
    let bundle_path = resolve_application_bundle(app_names)?;
    let executable_dir = bundle_path.join("Contents").join("MacOS");
    let entries = std::fs::read_dir(&executable_dir)
        .map_err(|error| format!("failed to inspect {}: {error}", executable_dir.display()))?;

    let mut executables: Vec<std::path::PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();

    executables.sort();

    if executables.is_empty() {
        return Err(format!(
            "No executable found inside {}",
            executable_dir.display()
        ));
    }

    let preferred_names: Vec<String> = app_names
        .iter()
        .flat_map(|name| [name.to_string(), name.to_ascii_lowercase()])
        .collect();

    if let Some(executable) = executables.iter().find(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| {
                preferred_names
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(name))
            })
            .unwrap_or(false)
    }) {
        return Ok(executable.clone());
    }

    Ok(executables[0].clone())
}

#[cfg(target_os = "macos")]
fn spawn_detached(executable: &std::path::Path, args: &[&str]) -> Result<(), String> {
    use std::process::Command;

    Command::new(executable)
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to launch {}: {error}", executable.display()))
}

#[cfg(target_os = "macos")]
fn applescript_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(not(target_os = "macos"))]
fn open_session_impl(
    _settings: &TerminalSettings,
    _tmux_binary: &str,
    _session_name: &str,
    _socket_path: Option<&str>,
) -> Result<(), String> {
    Err("Opening a tmux session from the desktop app is only supported on macOS. Session listing still works on Linux.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_attach_command_with_socket() {
        let command = build_tmux_attach_command(
            "/opt/homebrew/bin/tmux",
            "demo",
            Some("/tmp/tmux-1000/default"),
        );
        assert_eq!(
            command,
            "'/opt/homebrew/bin/tmux' -S '/tmp/tmux-1000/default' attach -t 'demo'"
        );
    }

    #[test]
    fn renders_custom_command_placeholders() {
        let rendered = render_custom_command(
            "open -na Ghostty --args -e /bin/zsh -lc \"{{command}}\"",
            "/opt/homebrew/bin/tmux",
            "demo",
            Some("/tmp/tmux-1000/default"),
        );

        assert!(rendered.contains("open -na Ghostty --args -e /bin/zsh -lc"));
        assert!(rendered.contains("attach -t 'demo'"));
        assert!(rendered.contains("-S '/tmp/tmux-1000/default'"));
    }

    #[test]
    fn appends_attach_command_when_template_has_no_placeholder() {
        let rendered = render_custom_command("custom-launcher", "tmux", "demo", None);
        assert_eq!(rendered, "custom-launcher 'tmux' attach -t 'demo'");
    }
}
