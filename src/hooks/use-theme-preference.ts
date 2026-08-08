import { useEffect } from "react";

import {
  applyThemeIfUnchanged,
  getThemeRevision,
  refreshTheme,
} from "@/lib/theme";
import { tauriClient } from "@/services/tauri-client";
import type { AppSettings } from "@/shared/contracts";

export function useThemePreference() {
  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    let theme: AppSettings["theme"] = "system";
    let cancelled = false;
    const initialRevision = getThemeRevision();
    const refresh = () => void refreshTheme().catch(() => undefined);

    async function initialize() {
      try {
        const settings = await tauriClient.getSettings();
        theme = settings.theme;
      } catch {
        // Use the system theme when persisted settings cannot be read.
      }

      try {
        await applyThemeIfUnchanged(theme, initialRevision);
      } catch {
        // The CSS theme is already applied even if the native theme call fails.
      }

      await nextPaint();
      if (!cancelled && "__TAURI_INTERNALS__" in window) {
        await tauriClient.finishStartup();
      }
    }

    void initialize();
    media.addEventListener("change", refresh);
    return () => {
      cancelled = true;
      media.removeEventListener("change", refresh);
    };
  }, []);
}

function nextPaint() {
  return new Promise<void>((resolve) => {
    requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
  });
}
