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

export type QwenAsrModel = "qwen-audio-3.0-asr-flash";
export type QwenRewriteModel =
  | "qwen3.7-flash"
  | "qwen3.7-plus"
  | "qwen3.7-max"
  | "qwen3.6-flash"
  | "qwen3.6-plus"
  | "qwen3.5-flash"
  | "qwen3.5-plus";

export interface QwenModelSettings {
  asrModel: QwenAsrModel;
  rewriteModel: QwenRewriteModel;
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
