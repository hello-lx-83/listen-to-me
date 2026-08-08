import type { PropsWithChildren } from "react";

import { TooltipProvider } from "@/components/ui/tooltip";
import { useThemePreference } from "@/hooks/use-theme-preference";

export function AppProviders({ children }: PropsWithChildren) {
  useThemePreference();
  return <TooltipProvider>{children}</TooltipProvider>;
}
