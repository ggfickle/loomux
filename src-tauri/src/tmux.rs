use serde::Serialize;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const COMMON_TMUX_PATHS: &[&str] = &[
    "/opt/homebrew/bin/tmux",
    "/usr/local/bin/tmux",
    "/usr/bin/tmux",
    "/bin/tmux",
    "/opt/local/bin/tmux",
];

const SESSION_FORMAT: &str = "#{session_name}\t#{session_attached}\t#{session_windows}";

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TmuxSession {
    pub name: String,
    pub attached: bool,
    pub windows: u32,
    pub socket_path: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TmuxOverview {
    pub session_count: usize,
    pub tmux_process_count: usize,
    pub tmux_binary_path: String,
    pub primary_socket_path: Option<String>,
    pub session_detection: String,
    pub debug_notes: Vec<String>,
    pub sessions: Vec<TmuxSession>,
}

#[derive(Debug, Clone)]
pub struct TmuxCommandTarget {
    pub socket_path: Option<String>,
}

#[derive(Debug, Clone)]
struct ProbeCandidate {
    socket_path: Option<PathBuf>,
    source: String,
}

#[derive(Debug, Clone)]
struct ProbeOutcome {
    sessions: Vec<TmuxSession>,
    candidate: ProbeCandidate,
    detection: String,
    debug_notes: Vec<String>,
}

#[derive(Debug)]
enum QueryResult {
    Success(String),
    NoServer(String),
    Failure(String),
}

pub fn get_overview() -> Result<TmuxOverview, String> {
    let tmux_binary = resolve_tmux_binary()?;
    let probe = detect_sessions(&tmux_binary)?;
    let tmux_process_count = count_tmux_processes(&tmux_binary);

    Ok(TmuxOverview {
        session_count: probe.sessions.len(),
        tmux_process_count,
        tmux_binary_path: tmux_binary,
        primary_socket_path: probe
            .candidate
            .socket_path
            .as_ref()
            .map(|path| path.display().to_string()),
        session_detection: probe.detection,
        debug_notes: probe.debug_notes,
        sessions: probe.sessions,
    })
}

pub fn create_session(session_name: &str) -> Result<(), String> {
    let session_name = validate_session_name(session_name)?;
    let tmux_binary = resolve_tmux_binary()?;
    let target = resolve_primary_target(&tmux_binary)?;
    run_tmux_command(
        &tmux_binary,
        Some(&target),
        &["new-session", "-d", "-s", session_name],
    )
}

pub fn rename_session(
    old_name: &str,
    new_name: &str,
    socket_path: Option<&str>,
) -> Result<(), String> {
    let old_name = validate_session_name(old_name)?;
    let new_name = validate_session_name(new_name)?;
    let tmux_binary = resolve_tmux_binary()?;
    let target = resolve_target_for_session(&tmux_binary, old_name, socket_path)?;
    run_tmux_command(
        &tmux_binary,
        Some(&target),
        &["rename-session", "-t", old_name, new_name],
    )
}

pub fn delete_session(session_name: &str, socket_path: Option<&str>) -> Result<(), String> {
    let session_name = validate_session_name(session_name)?;
    let tmux_binary = resolve_tmux_binary()?;
    let target = resolve_target_for_session(&tmux_binary, session_name, socket_path)?;
    run_tmux_command(
        &tmux_binary,
        Some(&target),
        &["kill-session", "-t", session_name],
    )
}

pub fn tmux_command_target_for_session(
    session_name: &str,
    socket_path: Option<&str>,
) -> Result<(String, TmuxCommandTarget), String> {
    let session_name = validate_session_name(session_name)?;
    let tmux_binary = resolve_tmux_binary()?;
    let target = resolve_target_for_session(&tmux_binary, session_name, socket_path)?;

    Ok((
        tmux_binary,
        TmuxCommandTarget {
            socket_path: target.socket_path.map(|path| path.display().to_string()),
        },
    ))
}

fn validate_session_name(session_name: &str) -> Result<&str, String> {
    let trimmed = session_name.trim();
    if trimmed.is_empty() {
        return Err("session name cannot be empty".to_string());
    }

    Ok(trimmed)
}

fn resolve_primary_target(tmux_binary: &str) -> Result<ProbeCandidate, String> {
    Ok(detect_sessions(tmux_binary)?.candidate)
}

fn resolve_target_for_session(
    tmux_binary: &str,
    session_name: &str,
    socket_path: Option<&str>,
) -> Result<ProbeCandidate, String> {
    if let Some(explicit_socket) = normalize_socket_path(socket_path) {
        return Ok(ProbeCandidate {
            socket_path: Some(PathBuf::from(explicit_socket)),
            source: "explicit socket supplied by UI".to_string(),
        });
    }

    let probe = detect_sessions(tmux_binary)?;
    if let Some(session) = probe
        .sessions
        .iter()
        .find(|session| session.name == session_name)
    {
        return Ok(ProbeCandidate {
            socket_path: session.socket_path.as_ref().map(PathBuf::from),
            source: format!("auto-resolved socket for session {session_name}"),
        });
    }

    Ok(probe.candidate)
}

fn normalize_socket_path(socket_path: Option<&str>) -> Option<String> {
    socket_path
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
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

    if let Some(shell_binary) = resolve_tmux_binary_from_login_shell() {
        if is_usable_tmux(&shell_binary) {
            return Ok(shell_binary);
        }
    }

    for candidate in COMMON_TMUX_PATHS {
        if is_usable_tmux(candidate) {
            return Ok((*candidate).to_string());
        }
    }

    Err(format!(
        "tmux binary not found. Checked PATH, login shell, plus: {}. If tmux is installed in a custom location, set TMUX_BINARY_PATH.",
        COMMON_TMUX_PATHS.join(", ")
    ))
}

fn resolve_tmux_binary_from_login_shell() -> Option<String> {
    login_shell_capture("command -v tmux 2>/dev/null | head -n 1")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
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

fn run_tmux_command(
    tmux_binary: &str,
    target: Option<&ProbeCandidate>,
    args: &[&str],
) -> Result<(), String> {
    let output = build_tmux_command(tmux_binary, target).args(args).output();

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

fn detect_sessions(tmux_binary: &str) -> Result<ProbeOutcome, String> {
    let mut shared_notes = Vec::new();
    let candidates = build_probe_candidates(&mut shared_notes)?;

    let mut best_outcome: Option<ProbeOutcome> = None;
    let mut last_failure: Option<String> = None;

    for candidate in candidates {
        let mut probe_notes = Vec::new();
        match list_sessions_for_candidate(tmux_binary, &candidate, &mut probe_notes) {
            Ok(sessions) => {
                let detection = candidate_description(&candidate);
                let outcome = ProbeOutcome {
                    sessions,
                    candidate: candidate.clone(),
                    detection,
                    debug_notes: probe_notes,
                };

                if outcome_is_better(&outcome, best_outcome.as_ref()) {
                    best_outcome = Some(outcome);
                }
            }
            Err(error) => {
                shared_notes.push(format!(
                    "{} failed: {}",
                    candidate_description(&candidate),
                    error
                ));
                last_failure = Some(error);
            }
        }
    }

    if let Some(mut outcome) = best_outcome {
        shared_notes.append(&mut outcome.debug_notes);
        outcome.debug_notes = dedupe_notes(shared_notes);
        return Ok(outcome);
    }

    if let Some(error) = last_failure {
        let details = dedupe_notes(shared_notes).join(" | ");
        if details.is_empty() {
            Err(format!("failed to inspect tmux sessions: {error}"))
        } else {
            Err(format!(
                "failed to inspect tmux sessions: {error}. {details}"
            ))
        }
    } else {
        Ok(ProbeOutcome {
            sessions: Vec::new(),
            candidate: ProbeCandidate {
                socket_path: None,
                source: "tmux default socket".to_string(),
            },
            detection: "tmux default socket".to_string(),
            debug_notes: dedupe_notes(shared_notes),
        })
    }
}

fn outcome_is_better(candidate: &ProbeOutcome, current: Option<&ProbeOutcome>) -> bool {
    match current {
        None => true,
        Some(current) => candidate_rank(candidate) > candidate_rank(current),
    }
}

fn candidate_rank(outcome: &ProbeOutcome) -> (usize, usize) {
    let explicit_socket_bonus = if outcome.sessions.is_empty() {
        0
    } else {
        usize::from(outcome.candidate.socket_path.is_some())
    };

    (outcome.sessions.len(), explicit_socket_bonus)
}

fn build_probe_candidates(shared_notes: &mut Vec<String>) -> Result<Vec<ProbeCandidate>, String> {
    let mut candidates = vec![ProbeCandidate {
        socket_path: None,
        source: "tmux default socket".to_string(),
    }];

    let Some(uid) = current_user_uid() else {
        shared_notes.push("Unable to resolve current uid; socket scan skipped.".to_string());
        return Ok(candidates);
    };

    let mut roots = Vec::new();

    if let Ok(tmux_tmpdir) = env::var("TMUX_TMPDIR") {
        let trimmed = tmux_tmpdir.trim();
        if !trimmed.is_empty() {
            shared_notes.push(format!("TMUX_TMPDIR from app env: {trimmed}"));
            roots.push(PathBuf::from(trimmed));
        }
    }

    if let Some(shell_tmux_tmpdir) = login_shell_capture("printf '%s' \"${TMUX_TMPDIR:-}\"") {
        let trimmed = shell_tmux_tmpdir.trim();
        if !trimmed.is_empty() {
            shared_notes.push(format!(
                "TMUX_TMPDIR from login shell: {trimmed} (helps Finder-launched apps)"
            ));
            roots.push(PathBuf::from(trimmed));
        }
    }

    roots.push(PathBuf::from("/tmp"));
    roots.push(PathBuf::from("/private/tmp"));

    let mut seen_roots = HashSet::new();
    for root in roots {
        let normalized = root.display().to_string();
        if !seen_roots.insert(normalized.clone()) {
            continue;
        }

        let socket_dir = root.join(format!("tmux-{uid}"));
        if !socket_dir.exists() {
            continue;
        }

        let Ok(entries) = fs::read_dir(&socket_dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };

            if !file_type.is_socket() {
                continue;
            }

            shared_notes.push(format!("Found tmux socket candidate: {}", path.display()));
            candidates.push(ProbeCandidate {
                socket_path: Some(path),
                source: format!("socket scan under {}", socket_dir.display()),
            });
        }
    }

    Ok(dedupe_candidates(candidates))
}

fn list_sessions_for_candidate(
    tmux_binary: &str,
    candidate: &ProbeCandidate,
    probe_notes: &mut Vec<String>,
) -> Result<Vec<TmuxSession>, String> {
    match run_tmux_query(
        tmux_binary,
        candidate,
        &["list-sessions", "-F", SESSION_FORMAT],
    )? {
        QueryResult::Success(stdout) => {
            if let Some(sessions) = parse_structured_sessions(&stdout, candidate) {
                return Ok(sessions);
            }

            probe_notes.push(format!(
                "{} returned unexpected structured output; falling back to `tmux ls`.",
                candidate_description(candidate)
            ));
        }
        QueryResult::NoServer(message) => {
            probe_notes.push(format!(
                "{} reported no server: {}",
                candidate_description(candidate),
                message
            ));
            return Ok(Vec::new());
        }
        QueryResult::Failure(message) => {
            probe_notes.push(format!(
                "{} failed on `list-sessions -F`: {}. Falling back to `tmux ls`.",
                candidate_description(candidate),
                message
            ));
        }
    }

    match run_tmux_query(tmux_binary, candidate, &["ls"])? {
        QueryResult::Success(stdout) => Ok(parse_legacy_sessions(&stdout, candidate)),
        QueryResult::NoServer(message) => {
            probe_notes.push(format!(
                "{} fallback `tmux ls` also reported no server: {}",
                candidate_description(candidate),
                message
            ));
            Ok(Vec::new())
        }
        QueryResult::Failure(message) => Err(message),
    }
}

fn run_tmux_query(
    tmux_binary: &str,
    candidate: &ProbeCandidate,
    args: &[&str],
) -> Result<QueryResult, String> {
    let output = build_tmux_command(tmux_binary, Some(candidate))
        .args(args)
        .output();

    let output = match output {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!("tmux binary not found: {tmux_binary}"))
        }
        Err(error) => return Err(format!("failed to run tmux via {tmux_binary}: {error}")),
    };

    if output.status.success() {
        return Ok(QueryResult::Success(
            String::from_utf8_lossy(&output.stdout).to_string(),
        ));
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let normalized = stderr.to_lowercase();
    if normalized.contains("no server running")
        || normalized.contains("failed to connect to server")
        || normalized.contains("can't find socket")
        || normalized.contains("no such file or directory")
    {
        return Ok(QueryResult::NoServer(stderr));
    }

    Ok(QueryResult::Failure(if stderr.is_empty() {
        "tmux query failed".to_string()
    } else {
        stderr
    }))
}

fn build_tmux_command(tmux_binary: &str, candidate: Option<&ProbeCandidate>) -> Command {
    let mut command = Command::new(tmux_binary);

    if let Some(socket_path) = candidate.and_then(|candidate| candidate.socket_path.as_ref()) {
        command.arg("-S").arg(socket_path);
    }

    command
}

fn parse_structured_sessions(stdout: &str, candidate: &ProbeCandidate) -> Option<Vec<TmuxSession>> {
    let lines: Vec<&str> = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();

    if lines.is_empty() {
        return Some(Vec::new());
    }

    let mut sessions = Vec::with_capacity(lines.len());
    for line in lines {
        let session = parse_session_line(line, candidate)?;
        sessions.push(session);
    }

    Some(sessions)
}

fn parse_session_line(line: &str, candidate: &ProbeCandidate) -> Option<TmuxSession> {
    let mut parts = line.split('\t');
    let name = parts.next()?.to_string();
    let attached = matches!(parts.next()?, "1");
    let windows = parts.next()?.parse::<u32>().ok()?;

    Some(TmuxSession {
        name,
        attached,
        windows,
        socket_path: candidate
            .socket_path
            .as_ref()
            .map(|path| path.display().to_string()),
    })
}

fn parse_legacy_sessions(stdout: &str, candidate: &ProbeCandidate) -> Vec<TmuxSession> {
    stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| parse_legacy_session_line(line, candidate))
        .collect()
}

fn parse_legacy_session_line(line: &str, candidate: &ProbeCandidate) -> Option<TmuxSession> {
    let mut parts = line.splitn(2, ':');
    let name = parts.next()?.trim().to_string();
    let details = parts.next()?.trim();

    let windows = details
        .split_whitespace()
        .collect::<Vec<_>>()
        .windows(2)
        .find_map(|pair| {
            if pair[1].starts_with("window") {
                pair[0].parse::<u32>().ok()
            } else {
                None
            }
        })?;

    Some(TmuxSession {
        name,
        attached: details.contains("(attached)"),
        windows,
        socket_path: candidate
            .socket_path
            .as_ref()
            .map(|path| path.display().to_string()),
    })
}

fn candidate_description(candidate: &ProbeCandidate) -> String {
    if let Some(socket_path) = &candidate.socket_path {
        format!(
            "{} using socket {}",
            candidate.source,
            socket_path.display()
        )
    } else {
        candidate.source.clone()
    }
}

fn dedupe_candidates(candidates: Vec<ProbeCandidate>) -> Vec<ProbeCandidate> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for candidate in candidates {
        let key = candidate
            .socket_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "__default__".to_string());

        if seen.insert(key) {
            deduped.push(candidate);
        }
    }

    deduped
}

fn dedupe_notes(notes: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for note in notes {
        if seen.insert(note.clone()) {
            deduped.push(note);
        }
    }

    deduped
}

fn login_shell_capture(script: &str) -> Option<String> {
    let shell = env::var("SHELL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            if cfg!(target_os = "macos") {
                "/bin/zsh".to_string()
            } else {
                "/bin/sh".to_string()
            }
        });

    let output = Command::new(shell).arg("-lc").arg(script).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        None
    } else {
        Some(stdout)
    }
}

fn current_user_uid() -> Option<u32> {
    let output = Command::new("id").arg("-u").output().ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u32>()
        .ok()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate_with_socket(path: &str) -> ProbeCandidate {
        ProbeCandidate {
            socket_path: Some(PathBuf::from(path)),
            source: "test".to_string(),
        }
    }

    #[test]
    fn parses_structured_session_lines_with_socket() {
        let candidate = candidate_with_socket("/tmp/tmux-1000/default");
        let session = parse_session_line("oracle\t1\t3", &candidate).expect("session parsed");

        assert_eq!(session.name, "oracle");
        assert!(session.attached);
        assert_eq!(session.windows, 3);
        assert_eq!(
            session.socket_path.as_deref(),
            Some("/tmp/tmux-1000/default")
        );
    }

    #[test]
    fn parses_tmux_ls_fallback_lines() {
        let candidate = ProbeCandidate {
            socket_path: None,
            source: "test".to_string(),
        };
        let session = parse_legacy_session_line(
            "damo: 2 windows (created Sun Apr  6 12:34:56 2026) [80x24] (attached)",
            &candidate,
        )
        .expect("legacy session parsed");

        assert_eq!(session.name, "damo");
        assert!(session.attached);
        assert_eq!(session.windows, 2);
        assert_eq!(session.socket_path, None);
    }

    #[test]
    fn prefers_candidate_with_more_sessions() {
        let default = ProbeOutcome {
            sessions: vec![TmuxSession {
                name: "one".to_string(),
                attached: false,
                windows: 1,
                socket_path: None,
            }],
            candidate: ProbeCandidate {
                socket_path: None,
                source: "default".to_string(),
            },
            detection: "default".to_string(),
            debug_notes: Vec::new(),
        };
        let explicit = ProbeOutcome {
            sessions: vec![
                TmuxSession {
                    name: "one".to_string(),
                    attached: false,
                    windows: 1,
                    socket_path: Some("/tmp/tmux-1000/custom".to_string()),
                },
                TmuxSession {
                    name: "two".to_string(),
                    attached: true,
                    windows: 2,
                    socket_path: Some("/tmp/tmux-1000/custom".to_string()),
                },
            ],
            candidate: candidate_with_socket("/tmp/tmux-1000/custom"),
            detection: "explicit".to_string(),
            debug_notes: Vec::new(),
        };

        assert!(outcome_is_better(&explicit, Some(&default)));
        assert!(!outcome_is_better(&default, Some(&explicit)));
    }
}
