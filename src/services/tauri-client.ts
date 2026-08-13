import { invoke } from "@tauri-apps/api/core";

import type {
  AppSettings,
  AppSnapshot,
  DictionaryEntry,
  DictionaryEntryInput,
  DictionaryCategory,
  DashboardOverview,
  HistoryRecord,
  QwenCredentialStatus,
  QwenModelSettings,
} from "@/shared/contracts";

export const tauriClient = {
  finishStartup: () => invoke<void>("finish_startup"),
  getAppSnapshot: () => invoke<AppSnapshot>("get_app_snapshot"),
  getDashboardOverview: () => invoke<DashboardOverview>("get_dashboard_overview"),
  getQwenCredentialStatus: () =>
    invoke<QwenCredentialStatus>("get_qwen_credential_status"),
  saveQwenApiKey: (apiKey: string) =>
    invoke<QwenCredentialStatus>("save_qwen_api_key", { apiKey }),
  deleteQwenApiKey: () =>
    invoke<QwenCredentialStatus>("delete_qwen_api_key"),
  getQwenModelSettings: () =>
    invoke<QwenModelSettings>("get_qwen_model_settings"),
  updateQwenModelSettings: (settings: QwenModelSettings) =>
    invoke<QwenModelSettings>("update_qwen_model_settings", { settings }),
  testQwenAsrModel: () => invoke<string>("test_qwen_asr_model"),
  testQwenRewriteModel: (model: QwenModelSettings["rewriteModel"]) =>
    invoke<void>("test_qwen_rewrite_model", { model }),
  getAutostartEnabled: () => invoke<boolean>("get_autostart_enabled"),
  setAutostartEnabled: (enabled: boolean) =>
    invoke<boolean>("set_autostart_enabled", { enabled }),
  getSettings: () => invoke<AppSettings>("get_settings"),
  updateSettings: (settings: AppSettings) =>
    invoke<AppSettings>("update_settings", { settings }),
  listHistory: (limit = 200) =>
    invoke<HistoryRecord[]>("list_history", { limit }),
  deleteHistory: (id: number) => invoke<void>("delete_history", { id }),
  clearHistory: () => invoke<void>("clear_history"),
  listDictionary: () => invoke<DictionaryEntry[]>("list_dictionary"),
  upsertDictionary: (input: DictionaryEntryInput) =>
    invoke<DictionaryEntry>("upsert_dictionary", { input }),
  deleteDictionary: (id: number) => invoke<void>("delete_dictionary", { id }),
  listDictionaryCategories: () => invoke<DictionaryCategory[]>("list_dictionary_categories"),
  createDictionaryCategory: (name: string) =>
    invoke<DictionaryCategory>("create_dictionary_category", { name }),
  renameDictionaryCategory: (oldName: string, newName: string) =>
    invoke<DictionaryCategory>("rename_dictionary_category", { oldName, newName }),
  deleteDictionaryCategory: (name: string) =>
    invoke<void>("delete_dictionary_category", { name }),
};
