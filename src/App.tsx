import { useCallback, useEffect, useMemo, useState } from "react";
import {
  createTmuxSession,
  deleteTmuxSession,
  getTerminalSettings,
  getTmuxOverview,
  openTmuxSession,
  renameTmuxSession,
  updateTerminalSettings,
} from "./lib/api";
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
  const [savedTerminalSettings, setSavedTerminalSettings] = useState<TerminalSettings>(defaultTerminalSettings);
  const [settingsLoading, setSettingsLoading] = useState(true);
  const [settingsSaving, setSettingsSaving] = useState(false);

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
      setSavedTerminalSettings(next);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load terminal settings.");
    } finally {
      setSettingsLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadOverview("initial");
    void loadTerminalPreference();
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

  const saveTerminalPreference = useCallback(
    async (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();

      setSettingsSaving(true);
      setError(null);

      try {
        const saved = await updateTerminalSettings(terminalSettings);
        setTerminalSettings(saved);
        setSavedTerminalSettings(saved);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to save terminal settings.");
      } finally {
        setSettingsSaving(false);
      }
    },
    [terminalSettings],
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
      if (!window.confirm(`Delete tmux session "${sessionName}"?`)) {
        return;
      }

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

  const terminalSettingsDirty =
    terminalSettings.preferredTerminal !== savedTerminalSettings.preferredTerminal ||
    terminalSettings.customCommand !== savedTerminalSettings.customCommand;

  return (
    <main className="app-shell">
      <div className="window-chrome-spacer" aria-hidden="true" />
      <section className="hero">
        <div>
          <p className="eyebrow">Loomux</p>
          <h1>Manage local tmux sessions from a cleaner macOS desktop view.</h1>
          <p className="subtitle">
            Main window + menu bar mode for creating, opening, renaming, deleting, debugging tmux detection, and choosing which terminal Loomux should launch.
          </p>
        </div>
        <button
          className="secondary-button"
          onClick={() => void loadOverview("refresh")}
          disabled={loading || refreshing}
        >
          {refreshing ? "Refreshing…" : "Refresh"}
        </button>
      </section>

      <section className="summary-grid">
        {summary.map((item) => (
          <article className="summary-card" key={item.label}>
            <span>{item.label}</span>
            <strong>{item.value}</strong>
          </article>
        ))}
      </section>

      <section className="settings-panel">
        <div>
          <h2>Terminal preference</h2>
          <p>
            Loomux currently opens sessions with <strong>{activeTerminalOption.label}</strong>. This preference also applies to menu bar / tray session opens.
          </p>
        </div>
        <form className="settings-form" onSubmit={saveTerminalPreference}>
          <label className="settings-field">
            <span className="field-label">Open tmux sessions with</span>
            <select
              className="text-input select-input"
              value={terminalSettings.preferredTerminal}
              disabled={settingsLoading || settingsSaving}
              onChange={(event) =>
                setTerminalSettings((current) => ({
                  ...current,
                  preferredTerminal: event.target.value as TerminalPreference,
                }))
              }
            >
              {terminalOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>

          <p className="helper-text">{activeTerminalOption.description}</p>

          {terminalSettings.preferredTerminal === "custom" ? (
            <label className="settings-field">
              <span className="field-label">Custom command template</span>
              <input
                className="text-input"
                type="text"
                value={terminalSettings.customCommand}
                disabled={settingsLoading || settingsSaving}
                onChange={(event) =>
                  setTerminalSettings((current) => ({
                    ...current,
                    customCommand: event.target.value,
                  }))
                }
                placeholder='open -na Ghostty --args -e /bin/zsh -lc "{{command}}"'
                autoComplete="off"
              />
            </label>
          ) : null}

          <div className="helper-block">
            <span className="field-label">Custom placeholders</span>
            <code>{"{{command}}"}</code>
            <code>{"{{tmux_binary}}"}</code>
            <code>{"{{socket_arg}}"}</code>
            <code>{"{{session_name}}"}</code>
          </div>

          <div className="action-row">
            <button
              className="primary-button"
              type="submit"
              disabled={settingsLoading || settingsSaving || !terminalSettingsDirty}
            >
              {settingsSaving ? "Saving…" : terminalSettingsDirty ? "Save terminal preference" : "Saved"}
            </button>
            {terminalSettingsDirty ? (
              <button
                className="secondary-button"
                type="button"
                disabled={settingsSaving}
                onClick={() => setTerminalSettings(savedTerminalSettings)}
              >
                Reset
              </button>
            ) : null}
          </div>
        </form>
      </section>

      <section className="create-panel">
        <div>
          <h2>Create a tmux session</h2>
          <p>New sessions start detached. Loomux keeps using the strongest detected tmux socket and your saved terminal preference when you open a session.</p>
        </div>
        <form className="create-form" onSubmit={createSession}>
          <input
            className="text-input"
            type="text"
            value={createName}
            onChange={(event) => setCreateName(event.target.value)}
            placeholder="session-name"
            autoComplete="off"
          />
          <button className="primary-button" type="submit" disabled={creating || !createName.trim()}>
            {creating ? "Creating…" : "Create session"}
          </button>
        </form>
      </section>

      {!loading && overview.tmuxBinaryPath ? (
        <div className="banner">
          Resolved tmux binary: <code>{overview.tmuxBinaryPath}</code>
        </div>
      ) : null}

      {!loading && overview.sessionDetection ? (
        <div className="banner">
          Session probe: <code>{overview.sessionDetection}</code>
        </div>
      ) : null}

      {error ? <div className="banner error">{error}</div> : null}
      {loading ? <div className="banner">Loading tmux sessions…</div> : null}

      {!loading && overview.sessions.length === 0 ? (
        <section className="empty-state">
          <h2>No tmux sessions detected</h2>
          <p>
            Refresh after starting tmux elsewhere, or use the create box above. On macOS, Loomux scans common tmux sockets and login-shell TMUX_TMPDIR values used by Finder-launched apps.
          </p>
          {overview.debugNotes.length > 0 ? (
            <ul style={{ marginTop: 16, textAlign: "left" }}>
              {overview.debugNotes.map((note) => (
                <li key={note}>{note}</li>
              ))}
            </ul>
          ) : null}
        </section>
      ) : null}

      {!loading && overview.debugNotes.length > 0 && overview.sessions.length > 0 ? (
        <details className="banner" style={{ cursor: "pointer" }}>
          <summary>Detection notes</summary>
          <ul style={{ marginTop: 12, paddingLeft: 18 }}>
            {overview.debugNotes.map((note) => (
              <li key={note}>{note}</li>
            ))}
          </ul>
        </details>
      ) : null}

      {!loading && overview.sessions.length > 0 ? (
        <section className="session-list">
          {overview.sessions.map((session) => {
            const isOpening = openingSession === session.name;
            const isRenaming = renamingSession === session.name;
            const isSavingRename = savingRenameSession === session.name;
            const isDeleting = deletingSession === session.name;
            const renameDisabled = !renameValue.trim() || renameValue.trim() === session.name;

            return (
              <article className="session-card" key={`${session.socketPath ?? "default"}:${session.name}`}>
                <div className="session-main">
                  {isRenaming ? (
                    <form
                      className="rename-form"
                      onSubmit={(event) => void saveRename(event, session.name, session.socketPath)}
                    >
                      <label className="field-label" htmlFor={`rename-${session.name}`}>
                        Rename session
                      </label>
                      <input
                        id={`rename-${session.name}`}
                        className="text-input"
                        type="text"
                        value={renameValue}
                        onChange={(event) => setRenameValue(event.target.value)}
                        autoComplete="off"
                        autoFocus
                      />
                      <div className="action-row">
                        <button
                          className="primary-button"
                          type="submit"
                          disabled={isSavingRename || renameDisabled}
                        >
                          {isSavingRename ? "Saving…" : "Save"}
                        </button>
                        <button
                          className="secondary-button"
                          type="button"
                          onClick={cancelRename}
                          disabled={isSavingRename}
                        >
                          Cancel
                        </button>
                      </div>
                    </form>
                  ) : (
                    <>
                      <div className="session-header">
                        <h2>{session.name}</h2>
                        <span className={session.attached ? "pill attached" : "pill detached"}>
                          {session.attached ? "Attached" : "Detached"}
                        </span>
                      </div>
                      <p>
                        {session.windows} window{session.windows === 1 ? "" : "s"}
                        {session.socketPath ? (
                          <>
                            {" "}· socket <code>{session.socketPath}</code>
                          </>
                        ) : null}
                      </p>
                    </>
                  )}
                </div>

                {!isRenaming ? (
                  <div className="session-actions">
                    <button
                      className="primary-button"
                      onClick={() => void openSession(session.name, session.socketPath)}
                      disabled={isOpening || isDeleting}
                    >
                      {isOpening ? "Opening…" : "Open session"}
                    </button>
                    <button
                      className="secondary-button"
                      onClick={() => startRename(session.name)}
                      disabled={isOpening || isDeleting}
                    >
                      Rename
                    </button>
                    <button
                      className="secondary-button danger-button"
                      onClick={() => void removeSession(session.name, session.socketPath)}
                      disabled={isOpening || isDeleting}
                    >
                      {isDeleting ? "Deleting…" : "Delete"}
                    </button>
                  </div>
                ) : null}
              </article>
            );
          })}
        </section>
      ) : null}
    </main>
  );
}
