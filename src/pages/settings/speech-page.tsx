import { AlertCircleIcon } from "lucide-react";

import { SettingRow, SettingsSection } from "@/components/app/settings-section";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { useAppSettings } from "@/hooks/use-app-settings";
import { SettingsPage } from "@/pages/settings/settings-page";
import type { AppSettings, RewriteMode } from "@/shared/contracts";

export function SpeechSettingsPage() {
  const { settings, loading, saving, error, update } = useAppSettings();
  const disabled = loading || saving;

  return (
    <SettingsPage title="语音与语言" description="设置识别语言和默认改写方式。">
      {error ? <Alert variant="destructive"><AlertCircleIcon /><AlertDescription>{error}</AlertDescription></Alert> : null}
      <SettingsSection title="识别">
        <SettingRow
          title="语言"
          description="单一语言可提高识别准确率；中英混合请选择自动检测。"
          control={
            <Select value={settings.language} onValueChange={(value) => update({ language: value as AppSettings["language"] })} disabled={disabled}>
              <SelectTrigger><SelectValue /></SelectTrigger>
              <SelectContent><SelectGroup><SelectItem value="auto">自动检测</SelectItem><SelectItem value="zh">中文</SelectItem><SelectItem value="en">英语</SelectItem></SelectGroup></SelectContent>
            </Select>
          }
        />
        <SettingRow
          title="默认模式"
          description="新的语音输入会立即使用此模式。"
          control={
            <Select value={settings.rewriteMode} onValueChange={(value) => update({ rewriteMode: value as RewriteMode })} disabled={disabled}>
              <SelectTrigger><SelectValue /></SelectTrigger>
              <SelectContent><SelectGroup><SelectItem value="raw">原样</SelectItem><SelectItem value="clean">智能清理</SelectItem><SelectItem value="article">整理成文</SelectItem><SelectItem value="structured">结构化</SelectItem></SelectGroup></SelectContent>
            </Select>
          }
        />
      </SettingsSection>
    </SettingsPage>
  );
}
