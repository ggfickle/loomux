export type TmuxSession = {
  name: string;
  attached: boolean;
  windows: number;
};

export type TmuxOverview = {
  sessionCount: number;
  tmuxProcessCount: number;
  tmuxBinaryPath: string;
  sessions: TmuxSession[];
};
