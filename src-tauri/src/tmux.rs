use serde::Serialize;
use std::process::Command;

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
    pub sessions: Vec<TmuxSession>,
}

pub fn get_overview() -> Result<TmuxOverview, String> {
    let sessions = list_sessions()?;
    let tmux_process_count = count_tmux_processes();

    Ok(TmuxOverview {
        session_count: sessions.len(),
        tmux_process_count,
        sessions,
    })
}

pub fn create_session(session_name: &str) -> Result<(), String> {
    let session_name = validate_session_name(session_name)?;
    run_tmux_command(&["new-session", "-d", "-s", session_name])
}

pub fn rename_session(old_name: &str, new_name: &str) -> Result<(), String> {
    let old_name = validate_session_name(old_name)?;
    let new_name = validate_session_name(new_name)?;
    run_tmux_command(&["rename-session", "-t", old_name, new_name])
}

pub fn delete_session(session_name: &str) -> Result<(), String> {
    let session_name = validate_session_name(session_name)?;
    run_tmux_command(&["kill-session", "-t", session_name])
}

fn validate_session_name(session_name: &str) -> Result<&str, String> {
    let trimmed = session_name.trim();
    if trimmed.is_empty() {
        return Err("session name cannot be empty".to_string());
    }

    Ok(trimmed)
}

fn run_tmux_command(args: &[&str]) -> Result<(), String> {
    let output = Command::new("tmux").args(args).output();

    let output = match output {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err("tmux is not installed or not available on PATH".to_string())
        }
        Err(error) => return Err(format!("failed to run tmux: {error}")),
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

fn list_sessions() -> Result<Vec<TmuxSession>, String> {
    let output = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}\t#{session_attached}\t#{session_windows}"])
        .output();

    let output = match output {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("failed to run tmux: {error}")),
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

fn count_tmux_processes() -> usize {
    if let Ok(output) = Command::new("pgrep").args(["-x", "tmux"]).output() {
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
                .filter(|line| *line == "tmux")
                .count();
        }
    }

    0
}
