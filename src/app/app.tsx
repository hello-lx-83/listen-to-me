import { getCurrentWindow } from "@tauri-apps/api/window";
import { lazy, Suspense } from "react";

const MainApp = lazy(() => import("@/app/main-app").then(({ MainApp }) => ({ default: MainApp })));
const OverlayApp = lazy(() => import("@/app/overlay-app").then(({ OverlayApp }) => ({ default: OverlayApp })));

function isVoiceOverlay() {
  try {
    return getCurrentWindow().label === "voice-overlay";
  } catch {
    return false;
  }
}

export function App() {
  return <Suspense fallback={null}>{isVoiceOverlay() ? <OverlayApp /> : <MainApp />}</Suspense>;
}
