import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  createTmuxSession,
  deleteTmuxSession,
  getTerminalSettings,
  getTmuxOverview,
  openTmuxSession,
  renameTmuxSession,
  updateTerminalSettings,
} from "./lib/api";
import { handleWindowDragMouseDown } from "./lib/windowDrag";
import type { TerminalPreference, TerminalSettings, TmuxOverview } from "./types";

const emptyOverview: TmuxOverview = {
  sessionCount: 0,
  tmuxProcessCount: 0,
  tmuxBinaryPath: "",
  primarySocketPath: null,
  sessionDetection: "",
  debugNotes: [],
  sessions: [],
};

const defaultTerminalSettings: TerminalSettings = {
  preferredTerminal: "auto",
  customCommand: "",
};

const terminalOptions: Array<{ value: TerminalPreference; label: string; description: string }> = [
  {
    value: "auto",
    label: "Auto",
    description: "Prefer Ghostty, then iTerm, then Tabby, and fall back to Terminal if none of those apps are installed.",
  },
  {
    value: "terminal",
    label: "Terminal",
    description: "Use the built-in Terminal.app AppleScript path.",
  },
  {
    value: "iterm",
    label: "iTerm",
    description: "Open a new iTerm window with the tmux attach command.",
  },
  {
    value: "ghostty",
    label: "Ghostty",
    description: "Launch Ghostty and run the attach command through its command-line entry point.",
  },
  {
    value: "tabby",
    label: "Tabby",
    description: "Launch Tabby and use its `run` command-line mode to start tmux.",
  },
  {
    value: "custom",
    label: "Custom command",
    description: "Run your own shell command template when opening a tmux session.",
  },
];

export default function App() {
  const [overview, setOverview] = useState<TmuxOverview>(emptyOverview);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [createName, setCreateName] = useState("");
  const [creating, setCreating] = useState(false);
  const [openingSession, setOpeningSession] = useState<string | null>(null);
  const [renamingSession, setRenamingSession] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [savingRenameSession, setSavingRenameSession] = useState<string | null>(null);
  const [deletingSession, setDeletingSession] = useState<string | null>(null);
  const [terminalSettings, setTerminalSettings] = useState<TerminalSettings>(defaultTerminalSettings);
  const [settingsLoading, setSettingsLoading] = useState(true);
  const autoRefreshIntervalRef = useRef<number | null>(null);

  const loadOverview = useCallback(async (mode: "initial" | "refresh") => {
    if (mode === "initial") {
      setLoading(true);
    } else {
      setRefreshing(true);
    }

    setError(null);

    try {
      const next = await getTmuxOverview();
      setOverview(next);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load tmux sessions.");
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, []);

  const loadTerminalPreference = useCallback(async () => {
    setSettingsLoading(true);

    try {
      const next = await getTerminalSettings();
      setTerminalSettings(next);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load terminal settings.");
    } finally {
      setSettingsLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadOverview("initial");
    void loadTerminalPreference();

    // Auto-refresh every 3 seconds
    autoRefreshIntervalRef.current = window.setInterval(() => {
      void loadOverview("refresh");
    }, 3000);

    return () => {
      if (autoRefreshIntervalRef.current !== null) {
        clearInterval(autoRefreshIntervalRef.current);
      }
    };
  }, [loadOverview, loadTerminalPreference]);

  const createSession = useCallback(
    async (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();

      const sessionName = createName.trim();
      if (!sessionName) {
        return;
      }

      setCreating(true);
      setError(null);

      try {
        await createTmuxSession(sessionName);
        setCreateName("");
        await loadOverview("refresh");
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to create tmux session.");
      } finally {
        setCreating(false);
      }
    },
    [createName, loadOverview],
  );

  const openSession = useCallback(async (sessionName: string, socketPath?: string | null) => {
    setOpeningSession(sessionName);
    setError(null);

    try {
      await openTmuxSession(sessionName, socketPath);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to open tmux session.");
    } finally {
      setOpeningSession(null);
    }
  }, []);

  const startRename = useCallback((sessionName: string) => {
    setError(null);
    setRenamingSession(sessionName);
    setRenameValue(sessionName);
  }, []);

  const cancelRename = useCallback(() => {
    setRenamingSession(null);
    setRenameValue("");
  }, []);

  const saveRename = useCallback(
    async (
      event: React.FormEvent<HTMLFormElement>,
      currentName: string,
      socketPath?: string | null,
    ) => {
      event.preventDefault();

      const nextName = renameValue.trim();
      if (!nextName || nextName === currentName) {
        return;
      }

      setSavingRenameSession(currentName);
      setError(null);

      try {
        await renameTmuxSession(currentName, nextName, socketPath);
        setRenamingSession(null);
        setRenameValue("");
        await loadOverview("refresh");
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to rename tmux session.");
      } finally {
        setSavingRenameSession(null);
      }
    },
    [loadOverview, renameValue],
  );

  const removeSession = useCallback(
    async (sessionName: string, socketPath?: string | null) => {
      setDeletingSession(sessionName);
      setError(null);

      try {
        await deleteTmuxSession(sessionName, socketPath);
        if (renamingSession === sessionName) {
          setRenamingSession(null);
          setRenameValue("");
        }
        await loadOverview("refresh");
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to delete tmux session.");
      } finally {
        setDeletingSession(null);
      }
    },
    [loadOverview, renamingSession],
  );

  const summary = useMemo(
    () => [
      { label: "Sessions", value: overview.sessionCount },
      { label: "tmux Processes", value: overview.tmuxProcessCount },
    ],
    [overview],
  );

  const activeTerminalOption = useMemo(
    () => terminalOptions.find((option) => option.value === terminalSettings.preferredTerminal) ?? terminalOptions[0],
    [terminalSettings.preferredTerminal],
  );

  return (
    <main className="app-shell">
      <div
        className="window-chrome-spacer"
        data-tauri-drag-region
        aria-hidden="true"
        onMouseDown={(event) => {
          void handleWindowDragMouseDown(event);
        }}
      />

      <div className="toolbar">
        <div
          className="toolbar-left"
          data-tauri-drag-region
          onMouseDown={(event) => {
            void handleWindowDragMouseDown(event);
          }}
        >
          <h1 className="app-title">Loomux</h1>
          <span className="session-count">{overview.sessionCount} sessions</span>
        </div>
        <div className="toolbar-right">
          <div className="toolbar-controls">
            <div className="terminal-select-shell">
              <select
                className="terminal-select"
                value={terminalSettings.preferredTerminal}
                disabled={settingsLoading}
                onChange={(event) => {
                  const newSettings = {
                    ...terminalSettings,
                    preferredTerminal: event.target.value as TerminalPreference,
                  };
                  setTerminalSettings(newSettings);
                  void updateTerminalSettings(newSettings);
                }}
                title="Terminal preference"
              >
                {terminalOptions.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
              <span className="terminal-select-chevron" aria-hidden="true">
                ▾
              </span>
            </div>
            <button
              className="icon-button"
              onClick={() => void loadOverview("refresh")}
              disabled={loading || refreshing}
              title="Refresh sessions"
            >
              {refreshing ? "⟳" : "↻"}
            </button>
          </div>
        </div>
      </div>

      <form className="create-bar" onSubmit={createSession}>
        <input
          className="create-input"
          type="text"
          value={createName}
          onChange={(event) => setCreateName(event.target.value)}
          placeholder="New session name..."
          autoComplete="off"
        />
        <button className="create-button" type="submit" disabled={creating || !createName.trim()}>
          {creating ? "..." : "+"}
        </button>
      </form>

      {error ? <div className="error-banner">{error}</div> : null}
      {loading ? <div className="loading-banner">Loading...</div> : null}

      {!loading && overview.sessions.length === 0 ? (
        <div className="empty-state">
          <p>No tmux sessions found</p>
          <button className="link-button" onClick={() => void loadOverview("refresh")}>
            Refresh
          </button>
        </div>
      ) : null}

      {!loading && overview.sessions.length > 0 ? (
        <div className="session-list">
          {overview.sessions.map((session) => {
            const isOpening = openingSession === session.name;
            const isRenaming = renamingSession === session.name;
            const isSavingRename = savingRenameSession === session.name;
            const isDeleting = deletingSession === session.name;
            const renameDisabled = !renameValue.trim() || renameValue.trim() === session.name;

            return (
              <div className="session-row" key={`${session.socketPath ?? "default"}:${session.name}`}>
                {isRenaming ? (
                  <form
                    className="rename-form"
                    onSubmit={(event) => void saveRename(event, session.name, session.socketPath)}
                  >
                    <input
                      className="rename-input"
                      type="text"
                      value={renameValue}
                      onChange={(event) => setRenameValue(event.target.value)}
                      autoComplete="off"
                      autoFocus
                    />
                    <button
                      className="action-button save-button"
                      type="submit"
                      disabled={isSavingRename || renameDisabled}
                      title="Save"
                    >
                      ✓
                    </button>
                    <button
                      className="action-button cancel-button"
                      type="button"
                      onClick={cancelRename}
                      disabled={isSavingRename}
                      title="Cancel"
                    >
                      ✕
                    </button>
                  </form>
                ) : (
                  <>
                    <div className="session-info">
                      <span className="session-name">{session.name}</span>
                      <span className="session-meta">
                        {session.windows}w
                        {session.attached ? " · attached" : ""}
                      </span>
                    </div>
                    <div className="session-actions">
                      <button
                        className="action-button open-button"
                        onClick={() => void openSession(session.name, session.socketPath)}
                        disabled={isOpening || isDeleting}
                        title="Open session"
                      >
                        {isOpening ? "..." : "▶"}
                      </button>
                      <button
                        className="action-button rename-button"
                        onClick={() => startRename(session.name)}
                        disabled={isOpening || isDeleting}
                        title="Rename"
                      >
                        ✎
                      </button>
                      <button
                        className="action-button delete-button"
                        onClick={() => void removeSession(session.name, session.socketPath)}
                        disabled={isOpening || isDeleting}
                        title="Delete"
                      >
                        {isDeleting ? "..." : "✕"}
                      </button>
                    </div>
                  </>
                )}
              </div>
            );
          })}
        </div>
      ) : null}
    </main>
  );
}
