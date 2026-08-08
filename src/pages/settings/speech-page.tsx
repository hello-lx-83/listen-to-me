import { AlertCircleIcon } from "lucide-react";

import { SettingRow, SettingsSection } from "@/components/app/settings-section";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { useAppSettings } from "@/hooks/use-app-settings";
import { SettingsPage } from "@/pages/settings/settings-page";
import { CORE_REWRITE_MODES } from "@/shared/rewrite-mode-config";
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
          description="轻点右 Alt 可随时循环切换；长按仍然是开始说话。"
          control={
            <Select value={settings.rewriteMode} onValueChange={(value) => update({ rewriteMode: value as RewriteMode })} disabled={disabled}>
              <SelectTrigger><SelectValue /></SelectTrigger>
              <SelectContent><SelectGroup>{CORE_REWRITE_MODES.map((mode) => <SelectItem key={mode.value} value={mode.value}>{mode.label}</SelectItem>)}</SelectGroup></SelectContent>
            </Select>
          }
        />
        <div className="grid gap-2 p-4 pt-0 sm:grid-cols-3">
          {CORE_REWRITE_MODES.map((mode) => (
            <button
              key={mode.value}
              type="button"
              className="rounded-lg border p-3 text-left transition-colors hover:bg-muted/60 disabled:pointer-events-none disabled:opacity-50 data-[active=true]:border-primary/40 data-[active=true]:bg-primary/5"
              data-active={settings.rewriteMode === mode.value}
              onClick={() => void update({ rewriteMode: mode.value })}
              disabled={disabled}
              aria-pressed={settings.rewriteMode === mode.value}
            >
              <span className="text-sm font-medium">{mode.label}</span>
              <span className="mt-1 block text-xs leading-relaxed text-muted-foreground">{mode.description}</span>
            </button>
          ))}
        </div>
      </SettingsSection>
    </SettingsPage>
  );
}
