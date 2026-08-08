import { SettingRow, SettingsSection } from "@/components/app/settings-section";
import { Kbd } from "@/components/ui/kbd";
import { SettingsPage } from "@/pages/settings/settings-page";

export function ShortcutSettingsPage() {
  return <SettingsPage title="快捷键" description="配置语音输入与取消操作。"><SettingsSection title="语音输入"><SettingRow title="按住说话" description="松开后开始转写与改写" control={<Kbd>Right Alt</Kbd>} /><SettingRow title="取消当前输入" control={<Kbd>Esc</Kbd>} /></SettingsSection></SettingsPage>;
}
