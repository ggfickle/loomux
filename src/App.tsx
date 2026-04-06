import { useCallback, useEffect, useMemo, useState } from "react";
import {
  createTmuxSession,
  deleteTmuxSession,
  getTmuxOverview,
  openTmuxSession,
  renameTmuxSession,
} from "./lib/api";
import type { TmuxOverview } from "./types";

const emptyOverview: TmuxOverview = {
  sessionCount: 0,
  tmuxProcessCount: 0,
  tmuxBinaryPath: "",
  sessions: [],
};

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

  useEffect(() => {
    void loadOverview("initial");
  }, [loadOverview]);

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

  const openSession = useCallback(async (sessionName: string) => {
    setOpeningSession(sessionName);
    setError(null);

    try {
      await openTmuxSession(sessionName);
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
    async (event: React.FormEvent<HTMLFormElement>, currentName: string) => {
      event.preventDefault();

      const nextName = renameValue.trim();
      if (!nextName || nextName === currentName) {
        return;
      }

      setSavingRenameSession(currentName);
      setError(null);

      try {
        await renameTmuxSession(currentName, nextName);
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
    async (sessionName: string) => {
      if (!window.confirm(`Delete tmux session "${sessionName}"?`)) {
        return;
      }

      setDeletingSession(sessionName);
      setError(null);

      try {
        await deleteTmuxSession(sessionName);
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

  return (
    <main className="app-shell">
      <div className="window-chrome-spacer" aria-hidden="true" />
      <section className="hero">
        <div>
          <p className="eyebrow">Loomux</p>
          <h1>Manage local tmux sessions from a cleaner macOS desktop view.</h1>
          <p className="subtitle">
            Create, rename, delete, refresh, and open tmux sessions in a cleaner native-feeling macOS app.
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

      <section className="create-panel">
        <div>
          <h2>Create a tmux session</h2>
          <p>New sessions start detached so you can open them when you are ready.</p>
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
        <div className="banner">Resolved tmux binary: <code>{overview.tmuxBinaryPath}</code></div>
      ) : null}

      {error ? <div className="banner error">{error}</div> : null}
      {loading ? <div className="banner">Loading tmux sessions…</div> : null}

      {!loading && overview.sessions.length === 0 ? (
        <section className="empty-state">
          <h2>No tmux sessions detected</h2>
          <p>
            Create one above or start tmux elsewhere and refresh. On Linux, session management works.
            Opening a session from the app stays macOS-only.
          </p>
        </section>
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
              <article className="session-card" key={session.name}>
                <div className="session-main">
                  {isRenaming ? (
                    <form className="rename-form" onSubmit={(event) => void saveRename(event, session.name)}>
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
                      <p>{session.windows} window{session.windows === 1 ? "" : "s"}</p>
                    </>
                  )}
                </div>

                {!isRenaming ? (
                  <div className="session-actions">
                    <button
                      className="primary-button"
                      onClick={() => void openSession(session.name)}
                      disabled={isOpening || isDeleting}
                    >
                      {isOpening ? "Opening…" : "Open in Terminal"}
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
                      onClick={() => void removeSession(session.name)}
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
