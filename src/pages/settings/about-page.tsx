import { SettingsSection } from "@/components/app/settings-section";
import { Badge } from "@/components/ui/badge";
import { SettingsPage } from "@/pages/settings/settings-page";

export function AboutSettingsPage() {
  return <SettingsPage title="关于" description="应用版本和运行环境。"><SettingsSection title="Listen to Me"><div className="flex items-center justify-between gap-4"><span className="text-sm text-muted-foreground">当前版本</span><Badge variant="secondary">0.1.0</Badge></div></SettingsSection></SettingsPage>;
}
