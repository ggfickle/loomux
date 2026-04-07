import { invoke } from "@tauri-apps/api/core";
import type { TmuxOverview } from "../types";

export async function getTmuxOverview(): Promise<TmuxOverview> {
  return invoke<TmuxOverview>("get_tmux_overview");
}

export async function createTmuxSession(sessionName: string): Promise<void> {
  await invoke("create_tmux_session", { sessionName });
}

export async function renameTmuxSession(
  oldName: string,
  newName: string,
  socketPath?: string | null,
): Promise<void> {
  await invoke("rename_tmux_session", { oldName, newName, socketPath });
}

export async function deleteTmuxSession(sessionName: string, socketPath?: string | null): Promise<void> {
  await invoke("delete_tmux_session", { sessionName, socketPath });
}

export async function openTmuxSession(sessionName: string, socketPath?: string | null): Promise<void> {
  await invoke("open_tmux_session", { sessionName, socketPath });
}
