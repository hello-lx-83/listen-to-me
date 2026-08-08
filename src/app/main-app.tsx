import { HashRouter } from "react-router-dom";

import { AppProviders } from "@/app/providers";
import { AppRouter } from "@/app/router";

export function MainApp() {
  return (
    <AppProviders>
      <HashRouter>
        <AppRouter />
      </HashRouter>
    </AppProviders>
  );
}
