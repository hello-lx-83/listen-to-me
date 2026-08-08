import { lazy, Suspense } from "react";
import { Navigate, Route, Routes } from "react-router-dom";

import { AppShell } from "@/layouts/app-shell";
import { SettingsLayout } from "@/layouts/settings-layout";

const HomePage = lazy(() => import("@/pages/home/home-page").then(({ HomePage }) => ({ default: HomePage })));
const HistoryPage = lazy(() => import("@/pages/history/history-page").then(({ HistoryPage }) => ({ default: HistoryPage })));
const DictionaryPage = lazy(() => import("@/pages/dictionary/dictionary-page").then(({ DictionaryPage }) => ({ default: DictionaryPage })));
const GeneralSettingsPage = lazy(() => import("@/pages/settings/general-page").then(({ GeneralSettingsPage }) => ({ default: GeneralSettingsPage })));
const ShortcutSettingsPage = lazy(() => import("@/pages/settings/shortcut-page").then(({ ShortcutSettingsPage }) => ({ default: ShortcutSettingsPage })));
const SpeechSettingsPage = lazy(() => import("@/pages/settings/speech-page").then(({ SpeechSettingsPage }) => ({ default: SpeechSettingsPage })));
const ModelsSettingsPage = lazy(() => import("@/pages/settings/models-page").then(({ ModelsSettingsPage }) => ({ default: ModelsSettingsPage })));
const PrivacySettingsPage = lazy(() => import("@/pages/settings/privacy-page").then(({ PrivacySettingsPage }) => ({ default: PrivacySettingsPage })));
const AboutSettingsPage = lazy(() => import("@/pages/settings/about-page").then(({ AboutSettingsPage }) => ({ default: AboutSettingsPage })));

export function AppRouter() {
  return (
    <Suspense fallback={null}>
      <Routes>
        <Route element={<AppShell />}>
          <Route index element={<HomePage />} />
          <Route path="history" element={<HistoryPage />} />
          <Route path="dictionary" element={<DictionaryPage />} />
          <Route path="settings" element={<SettingsLayout />}>
            <Route index element={<Navigate replace to="general" />} />
            <Route path="general" element={<GeneralSettingsPage />} />
            <Route path="shortcut" element={<ShortcutSettingsPage />} />
            <Route path="speech" element={<SpeechSettingsPage />} />
            <Route path="models" element={<ModelsSettingsPage />} />
            <Route path="privacy" element={<PrivacySettingsPage />} />
            <Route path="about" element={<AboutSettingsPage />} />
          </Route>
        </Route>
        <Route path="*" element={<Navigate replace to="/" />} />
      </Routes>
    </Suspense>
  );
}
