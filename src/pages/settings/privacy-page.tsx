import { AlertCircleIcon, Trash2Icon } from "lucide-react";

import { SettingRow, SettingsSection } from "@/components/app/settings-section";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { useAppSettings } from "@/hooks/use-app-settings";
import { SettingsPage } from "@/pages/settings/settings-page";
import { tauriClient } from "@/services/tauri-client";

export function PrivacySettingsPage() {
  const { settings, loading, saving, error, update } = useAppSettings();

  async function clearHistory() {
    if (!window.confirm("确定清空全部历史记录吗？此操作无法撤销。")) return;
    await tauriClient.clearHistory();
  }

  return (
    <SettingsPage title="隐私与数据" description="管理历史记录和本地数据策略。">
      {error ? <Alert variant="destructive"><AlertCircleIcon /><AlertDescription>{error}</AlertDescription></Alert> : null}
      <SettingsSection title="历史记录">
        <SettingRow
          title="保存输入历史"
          description="仅保存原始识别和整理文本，不保存音频及来源应用。"
          control={<Switch checked={settings.saveHistory} onCheckedChange={(checked) => update({ saveHistory: checked })} disabled={loading || saving} aria-label="保存输入历史" />}
        />
        <SettingRow title="清空历史" description="永久删除当前设备上的全部输入历史。" control={<Button variant="destructive" size="sm" onClick={clearHistory}><Trash2Icon data-icon="inline-start" />清空</Button>} />
      </SettingsSection>
      <SettingsSection title="本地数据">
        <SettingRow title="音频文件" description="录音仅在内存中处理，请求结束后释放，不写入磁盘。" control={<span className="text-sm text-muted-foreground">不保存</span>} />
        <SettingRow title="来源应用" description="不识别、不记录当前输入来自哪个应用。" control={<span className="text-sm text-muted-foreground">不采集</span>} />
      </SettingsSection>
    </SettingsPage>
  );
}
