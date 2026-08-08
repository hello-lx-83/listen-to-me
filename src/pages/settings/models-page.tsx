import { useEffect, useState, type FormEvent } from "react";

import { SettingRow, SettingsSection } from "@/components/app/settings-section";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Field, FieldDescription, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { SettingsPage } from "@/pages/settings/settings-page";
import { tauriClient } from "@/services/tauri-client";

export function ModelsSettingsPage() {
  const [configured, setConfigured] = useState(false);
  const [apiKey, setApiKey] = useState("");
  const [pending, setPending] = useState(true);
  const [error, setError] = useState("");
  const [testResult, setTestResult] = useState<"" | "success" | "failed">("");

  useEffect(() => {
    void tauriClient
      .getQwenCredentialStatus()
      .then((status) => setConfigured(status.configured))
      .catch(() => setError("无法读取 Windows 凭据状态。"))
      .finally(() => setPending(false));
  }, []);

  async function handleSave(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!apiKey.trim()) {
      setError("请输入 API Key。 ");
      return;
    }

    setPending(true);
    setError("");
    try {
      const status = await tauriClient.saveQwenApiKey(apiKey);
      setConfigured(status.configured);
      setApiKey("");
    } catch {
      setError("保存失败，请确认当前 Windows 会话可以使用凭据管理器。 ");
    } finally {
      setPending(false);
    }
  }

  async function handleDelete() {
    setPending(true);
    setError("");
    try {
      const status = await tauriClient.deleteQwenApiKey();
      setConfigured(status.configured);
      setApiKey("");
    } catch {
      setError("删除失败，请稍后重试。 ");
    } finally {
      setPending(false);
    }
  }

  async function handleTest() {
    setPending(true);
    setError("");
    setTestResult("");
    try {
      await tauriClient.testQwenConnection();
      setTestResult("success");
    } catch {
      setTestResult("failed");
      setError("连接测试失败，请检查密钥、网络和账户额度。 ");
    } finally {
      setPending(false);
    }
  }

  return (
    <SettingsPage title="模型与网络" description="选择云端或本地处理线路。">
      <SettingsSection title="模型线路" description="本地适配器将在云端链路稳定后实现。">
        <SettingRow
          title="处理方式"
          control={
            <Select defaultValue="cloud">
              <SelectTrigger><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectItem value="cloud">云端</SelectItem>
                  <SelectItem value="local" disabled>本地（稍后支持）</SelectItem>
                </SelectGroup>
              </SelectContent>
            </Select>
          }
        />
        <SettingRow
          title="云端状态"
          description="当前接入千问语音识别与文本模型。"
          control={<div className="flex items-center gap-2"><Badge variant={configured ? "default" : "secondary"}>{configured ? "已配置" : "未配置"}</Badge>{configured ? <Button size="sm" variant="outline" disabled={pending} onClick={handleTest}>{pending ? "检测中…" : "测试连接"}</Button> : null}</div>}
        />
        {testResult === "success" ? <p className="text-sm text-muted-foreground">连接测试成功，文本模型可用。</p> : null}
      </SettingsSection>

      <SettingsSection title="千问凭据" description="API Key 仅保存在当前 Windows 用户的凭据管理器中，界面不会读取或展示原值。">
        <form onSubmit={handleSave}>
          <FieldGroup>
            <Field data-invalid={Boolean(error)}>
              <FieldLabel htmlFor="qwen-api-key">API Key</FieldLabel>
              <Input
                id="qwen-api-key"
                type="password"
                value={apiKey}
                onChange={(event) => setApiKey(event.target.value)}
                placeholder={configured ? "输入新密钥以替换现有配置" : "输入新的 API Key"}
                autoComplete="off"
                aria-invalid={Boolean(error)}
                disabled={pending}
              />
              <FieldDescription>保存后输入框会立即清空；客户端只保留“已配置”状态。</FieldDescription>
              <FieldError>{error}</FieldError>
            </Field>
            <Field orientation="horizontal">
              <Button type="submit" disabled={pending || !apiKey.trim()}>{pending ? "处理中…" : configured ? "替换密钥" : "保存密钥"}</Button>
              {configured ? <Button type="button" variant="outline" disabled={pending} onClick={handleDelete}>移除密钥</Button> : null}
            </Field>
          </FieldGroup>
        </form>
      </SettingsSection>
    </SettingsPage>
  );
}
