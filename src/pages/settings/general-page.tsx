import { useEffect, useState } from "react";
import { AlertCircleIcon } from "lucide-react";

import { SettingRow, SettingsSection } from "@/components/app/settings-section";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { useAppSettings } from "@/hooks/use-app-settings";
import { applyTheme } from "@/lib/theme";
import { SettingsPage } from "@/pages/settings/settings-page";
import { tauriClient } from "@/services/tauri-client";
import type { AppSettings } from "@/shared/contracts";

export function GeneralSettingsPage() {
  const { settings, loading, saving, error, update } = useAppSettings();
  const [autostart, setAutostart] = useState(false);
  const [autostartPending, setAutostartPending] = useState(true);
  const [autostartError, setAutostartError] = useState("");

  useEffect(() => {
    void tauriClient.getAutostartEnabled()
      .then(setAutostart)
      .catch(() => setAutostartError("无法读取开机启动状态。"))
      .finally(() => setAutostartPending(false));
  }, []);

  async function changeTheme(theme: AppSettings["theme"]) {
    const saved = await update({ theme });
    if (saved) await applyTheme(saved.theme);
  }

  async function changeAutostart(enabled: boolean) {
    setAutostartPending(true);
    setAutostartError("");
    try {
      setAutostart(await tauriClient.setAutostartEnabled(enabled));
    } catch {
      setAutostartError("开机启动设置失败。 ");
    } finally {
      setAutostartPending(false);
    }
  }

  return (
    <SettingsPage title="通用" description="管理主题和界面偏好。">
      {error || autostartError ? <Alert variant="destructive"><AlertCircleIcon /><AlertDescription>{error || autostartError}</AlertDescription></Alert> : null}
      <SettingsSection title="外观">
        <SettingRow
          title="主题"
          description="选择浅色、深色或跟随 Windows。"
          control={
            <Select value={settings.theme} onValueChange={(value) => changeTheme(value as AppSettings["theme"])} disabled={loading || saving}>
              <SelectTrigger><SelectValue /></SelectTrigger>
              <SelectContent><SelectGroup><SelectItem value="system">跟随系统</SelectItem><SelectItem value="light">浅色</SelectItem><SelectItem value="dark">深色</SelectItem></SelectGroup></SelectContent>
            </Select>
          }
        />
      </SettingsSection>
      <SettingsSection title="启动">
        <SettingRow title="开机自动启动" description="登录 Windows 后在后台启动，主窗口可关闭到系统托盘。" control={<Switch checked={autostart} onCheckedChange={changeAutostart} disabled={autostartPending} aria-label="开机自动启动" />} />
      </SettingsSection>
    </SettingsPage>
  );
}
