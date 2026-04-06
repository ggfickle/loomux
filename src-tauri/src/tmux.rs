use serde::Serialize;
use std::env;
use std::path::Path;
use std::process::Command;

const COMMON_TMUX_PATHS: &[&str] = &[
    "/opt/homebrew/bin/tmux",
    "/usr/local/bin/tmux",
    "/usr/bin/tmux",
    "/bin/tmux",
    "/opt/local/bin/tmux",
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TmuxSession {
    pub name: String,
    pub attached: bool,
    pub windows: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TmuxOverview {
    pub session_count: usize,
    pub tmux_process_count: usize,
    pub tmux_binary_path: String,
    pub sessions: Vec<TmuxSession>,
}

pub fn get_overview() -> Result<TmuxOverview, String> {
    let tmux_binary = resolve_tmux_binary()?;
    let sessions = list_sessions(&tmux_binary)?;
    let tmux_process_count = count_tmux_processes(&tmux_binary);

    Ok(TmuxOverview {
        session_count: sessions.len(),
        tmux_process_count,
        tmux_binary_path: tmux_binary,
        sessions,
    })
}

pub fn create_session(session_name: &str) -> Result<(), String> {
    let session_name = validate_session_name(session_name)?;
    let tmux_binary = resolve_tmux_binary()?;
    run_tmux_command(&tmux_binary, &["new-session", "-d", "-s", session_name])
}

pub fn rename_session(old_name: &str, new_name: &str) -> Result<(), String> {
    let old_name = validate_session_name(old_name)?;
    let new_name = validate_session_name(new_name)?;
    let tmux_binary = resolve_tmux_binary()?;
    run_tmux_command(&tmux_binary, &["rename-session", "-t", old_name, new_name])
}

pub fn delete_session(session_name: &str) -> Result<(), String> {
    let session_name = validate_session_name(session_name)?;
    let tmux_binary = resolve_tmux_binary()?;
    run_tmux_command(&tmux_binary, &["kill-session", "-t", session_name])
}

pub fn tmux_binary_for_open() -> Result<String, String> {
    resolve_tmux_binary()
}

fn validate_session_name(session_name: &str) -> Result<&str, String> {
    let trimmed = session_name.trim();
    if trimmed.is_empty() {
        return Err("session name cannot be empty".to_string());
    }

    Ok(trimmed)
}

fn resolve_tmux_binary() -> Result<String, String> {
    if let Ok(override_path) = env::var("TMUX_BINARY_PATH") {
        let trimmed = override_path.trim();
        if trimmed.is_empty() {
            return Err("TMUX_BINARY_PATH is set but empty".to_string());
        }
        if is_usable_tmux(trimmed) {
            return Ok(trimmed.to_string());
        }
        return Err(format!(
            "tmux binary not usable at TMUX_BINARY_PATH={}.",
            trimmed
        ));
    }

    if is_usable_tmux("tmux") {
        return Ok("tmux".to_string());
    }

    for candidate in COMMON_TMUX_PATHS {
        if is_usable_tmux(candidate) {
            return Ok((*candidate).to_string());
        }
    }

    Err(format!(
        "tmux binary not found. Checked PATH plus: {}. If tmux is installed in a custom location, set TMUX_BINARY_PATH.",
        COMMON_TMUX_PATHS.join(", ")
    ))
}

fn is_usable_tmux(candidate: &str) -> bool {
    if candidate.contains('/') && !Path::new(candidate).exists() {
        return false;
    }

    Command::new(candidate)
        .arg("-V")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn run_tmux_command(tmux_binary: &str, args: &[&str]) -> Result<(), String> {
    let output = Command::new(tmux_binary).args(args).output();

    let output = match output {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!("tmux binary not found: {tmux_binary}"))
        }
        Err(error) => return Err(format!("failed to run tmux via {tmux_binary}: {error}")),
    };

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            Err("tmux command failed".to_string())
        } else {
            Err(stderr)
        }
    }
}

fn list_sessions(tmux_binary: &str) -> Result<Vec<TmuxSession>, String> {
    let output = Command::new(tmux_binary)
        .args(["list-sessions", "-F", "#{session_name}\t#{session_attached}\t#{session_windows}"])
        .output();

    let output = match output {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!("tmux binary not found: {tmux_binary}"))
        }
        Err(error) => return Err(format!("failed to run tmux via {tmux_binary}: {error}")),
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("no server running") || stderr.contains("failed to connect to server") {
            return Ok(Vec::new());
        }
        return Err(stderr.trim().to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(parse_session_line)
        .collect())
}

fn parse_session_line(line: &str) -> Option<TmuxSession> {
    let mut parts = line.split('\t');
    let name = parts.next()?.to_string();
    let attached = matches!(parts.next()?, "1");
    let windows = parts.next()?.parse::<u32>().ok()?;

    Some(TmuxSession {
        name,
        attached,
        windows,
    })
}

fn count_tmux_processes(tmux_binary: &str) -> usize {
    let process_name = if tmux_binary.contains('/') {
        Path::new(tmux_binary)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("tmux")
    } else {
        "tmux"
    };

    if let Ok(output) = Command::new("pgrep").args(["-x", process_name]).output() {
        if output.status.success() {
            return String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count();
        }
    }

    if let Ok(output) = Command::new("ps").args(["-axo", "comm="]).output() {
        if output.status.success() {
            return String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .filter(|line| *line == process_name)
                .count();
        }
    }

    0
}
