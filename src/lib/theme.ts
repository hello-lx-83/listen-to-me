import { getCurrentWindow } from "@tauri-apps/api/window";

import type { AppSettings } from "@/shared/contracts";

let activeTheme: AppSettings["theme"] = "system";
let themeRevision = 0;

export function getThemeRevision() {
  return themeRevision;
}

export async function applyTheme(theme: AppSettings["theme"]) {
  activeTheme = theme;
  themeRevision += 1;
  await syncTheme();
}

export async function applyThemeIfUnchanged(
  theme: AppSettings["theme"],
  expectedRevision: number,
) {
  if (themeRevision !== expectedRevision) return;

  activeTheme = theme;
  themeRevision += 1;
  await syncTheme();
}

export async function refreshTheme() {
  await syncTheme();
}

async function syncTheme() {
  const theme = activeTheme;
  const dark = theme === "dark"
    || (theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);

  document.documentElement.classList.toggle("dark", dark);
  document.documentElement.style.colorScheme = dark ? "dark" : "light";

  if ("__TAURI_INTERNALS__" in window) {
    await getCurrentWindow().setTheme(theme === "system" ? null : theme);
  }
}
