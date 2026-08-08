export type VoiceSessionState =
  | "idle"
  | "arming"
  | "recording"
  | "transcribing"
  | "rewriting"
  | "injecting"
  | "failed";

export interface AppSnapshot {
  voiceSession: VoiceSessionState;
  defaultShortcut: string;
  modelRoute: "auto" | "cloud" | "local";
}

export interface QwenCredentialStatus {
  configured: boolean;
}

export type RewriteMode = "raw" | "clean" | "article" | "structured";

export interface AppSettings {
  theme: "system" | "light" | "dark";
  language: "auto" | "zh" | "en";
  rewriteMode: RewriteMode;
  saveHistory: boolean;
  historyRetentionDays: 7 | 30;
}

export interface HistoryRecord {
  id: number;
  createdAt: number;
  mode: RewriteMode;
  transcript: string;
  output: string;
}

export interface DictionaryEntry {
  id: number;
  source: string;
  replacement: string;
  category: string;
  updatedAt: number;
}

export interface DictionaryEntryInput {
  id?: number;
  source: string;
  replacement: string;
  category: string;
}

export interface DictionaryCategory {
  name: string;
  entryCount: number;
}

export interface DashboardOverview {
  qwenConfigured: boolean;
  historyCount: number;
  dictionaryCount: number;
}
