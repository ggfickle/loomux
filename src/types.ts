export type TmuxSession = {
  name: string;
  attached: boolean;
  windows: number;
  socketPath: string | null;
};

export type TmuxOverview = {
  sessionCount: number;
  tmuxProcessCount: number;
  tmuxBinaryPath: string;
  primarySocketPath: string | null;
  sessionDetection: string;
  debugNotes: string[];
  sessions: TmuxSession[];
};

export type TerminalPreference = "auto" | "terminal" | "iterm" | "ghostty" | "tabby" | "custom";

export type TerminalSettings = {
  preferredTerminal: TerminalPreference;
  customCommand: string;
};
