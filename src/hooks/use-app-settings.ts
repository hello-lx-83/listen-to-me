import { useCallback, useEffect, useState } from "react";

import { tauriClient } from "@/services/tauri-client";
import type { AppSettings } from "@/shared/contracts";

const defaults: AppSettings = {
  theme: "system",
  language: "auto",
  rewriteMode: "clean",
  saveHistory: true,
};

export function useAppSettings() {
  const [settings, setSettings] = useState<AppSettings>(defaults);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    void tauriClient
      .getSettings()
      .then(setSettings)
      .catch(() => setError("无法读取设置。"))
      .finally(() => setLoading(false));
  }, []);

  const update = useCallback(async (patch: Partial<AppSettings>) => {
    setSaving(true);
    setError("");
    try {
      const next = await tauriClient.updateSettings({ ...settings, ...patch });
      setSettings(next);
      return next;
    } catch {
      setError("设置保存失败。 ");
      return null;
    } finally {
      setSaving(false);
    }
  }, [settings]);

  return { settings, loading, saving, error, update };
}
