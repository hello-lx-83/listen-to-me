import { useEffect, useState, type FormEvent } from "react";
import {
  AlertCircleIcon,
  CheckCircle2Icon,
  KeyRoundIcon,
  MicIcon,
  WandSparklesIcon,
} from "lucide-react";

import { SettingRow, SettingsSection } from "@/components/app/settings-section";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Spinner } from "@/components/ui/spinner";
import { SettingsPage } from "@/pages/settings/settings-page";
import { tauriClient } from "@/services/tauri-client";
import type { QwenModelSettings, QwenRewriteModel } from "@/shared/contracts";

const DEFAULT_MODELS: QwenModelSettings = {
  asrModel: "qwen-audio-3.0-asr-flash",
  rewriteModel: "qwen3.7-flash",
};

const REWRITE_MODELS: Array<{
  value: QwenRewriteModel;
  label: string;
}> = [
  {
    value: "qwen3.7-flash",
    label: "Qwen3.7 Flash · 推荐",
  },
  {
    value: "qwen3.7-plus",
    label: "Qwen3.7 Plus",
  },
  {
    value: "qwen3.7-max",
    label: "Qwen3.7 Max",
  },
  {
    value: "qwen3.6-flash",
    label: "Qwen3.6 Flash",
  },
  {
    value: "qwen3.6-plus",
    label: "Qwen3.6 Plus",
  },
  {
    value: "qwen3.5-flash",
    label: "Qwen3.5 Flash",
  },
  {
    value: "qwen3.5-plus",
    label: "Qwen3.5 Plus",
  },
];

type Capability = "asr" | "rewrite";
type TestResult = { status: "success" | "error"; message: string };

function testErrorMessage(error: unknown, capability: Capability) {
  const detail = error instanceof Error ? error.message : String(error);
  if (detail.includes("authentication failed") || detail.includes("not configured")) {
    return "API Key 无效，请重新配置。";
  }
  if (detail.includes("quota") || detail.includes("rate limit")) {
    return "额度或请求频率受限。";
  }
  if (detail.includes("HTTP status 404") || detail.includes("unsupported")) {
    return "模型不可用或尚未开通。";
  }
  if (detail.includes("network request failed")) {
    return "无法连接千问服务。";
  }
  if (detail.includes("HTTP status 400")) return "模型测试请求格式无效。";
  if (detail.includes("empty response") || detail.includes("could not be decoded")) {
    return capability === "asr"
      ? "没有识别到语音，请点击测试后立即说“测试语音识别”。"
      : "服务已连接，但返回内容无法用于完成测试。";
  }
  return capability === "asr" ? "语音识别测试失败。" : "文本整理测试失败。";
}

export function ModelsSettingsPage() {
  const [configured, setConfigured] = useState(false);
  const [apiKey, setApiKey] = useState("");
  const [models, setModels] = useState<QwenModelSettings>(DEFAULT_MODELS);
  const [loading, setLoading] = useState(true);
  const [credentialPending, setCredentialPending] = useState(false);
  const [modelSaving, setModelSaving] = useState(false);
  const [testing, setTesting] = useState<Record<Capability, boolean>>({
    asr: false,
    rewrite: false,
  });
  const [pageError, setPageError] = useState("");
  const [credentialError, setCredentialError] = useState("");
  const [testResults, setTestResults] = useState<Partial<Record<Capability, TestResult>>>({});

  useEffect(() => {
    void Promise.all([
      tauriClient.getQwenCredentialStatus(),
      tauriClient.getQwenModelSettings(),
    ])
      .then(([status, savedModels]) => {
        setConfigured(status.configured);
        setModels(savedModels);
      })
      .catch(() => setPageError("无法读取千问模型配置。"))
      .finally(() => setLoading(false));
  }, []);

  async function handleSave(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!apiKey.trim()) {
      setCredentialError("请输入 API Key。");
      return;
    }

    setCredentialPending(true);
    setPageError("");
    setCredentialError("");
    setTestResults({});
    try {
      const status = await tauriClient.saveQwenApiKey(apiKey);
      setConfigured(status.configured);
      setApiKey("");
    } catch {
      setCredentialError("保存失败，请确认当前 Windows 会话可以使用凭据管理器。");
    } finally {
      setCredentialPending(false);
    }
  }

  async function handleDelete() {
    if (!window.confirm("确定移除千问 API Key 吗？移除后语音输入将暂时不可用。")) return;

    setCredentialPending(true);
    setPageError("");
    setCredentialError("");
    setTestResults({});
    try {
      const status = await tauriClient.deleteQwenApiKey();
      setConfigured(status.configured);
      setApiKey("");
    } catch {
      setPageError("移除失败，请稍后重试。");
    } finally {
      setCredentialPending(false);
    }
  }

  async function handleRewriteModelChange(value: QwenRewriteModel | null) {
    if (!value || value === models.rewriteModel) return;

    setModelSaving(true);
    setPageError("");
    setTestResults((current) => ({ ...current, rewrite: undefined }));
    try {
      const saved = await tauriClient.updateQwenModelSettings({
        ...models,
        rewriteModel: value,
      });
      setModels(saved);
    } catch {
      setPageError("文本整理模型保存失败。");
    } finally {
      setModelSaving(false);
    }
  }

  async function handleTest(capability: Capability) {
    setTesting((current) => ({ ...current, [capability]: true }));
    setPageError("");
    setTestResults((current) => ({ ...current, [capability]: undefined }));
    try {
      let asrTranscript = "";
      if (capability === "asr") {
        asrTranscript = await tauriClient.testQwenAsrModel();
      } else {
        await tauriClient.testQwenRewriteModel(models.rewriteModel);
      }
      setTestResults((current) => ({
        ...current,
        [capability]: {
          status: "success",
          message: capability === "asr"
            ? `语音识别可用，识别结果：${asrTranscript}`
            : "文本整理可用。",
        },
      }));
    } catch (error) {
      setTestResults((current) => ({
        ...current,
        [capability]: { status: "error", message: testErrorMessage(error, capability) },
      }));
    } finally {
      setTesting((current) => ({ ...current, [capability]: false }));
    }
  }

  const controlsDisabled = loading || credentialPending || modelSaving;

  return (
    <SettingsPage title="模型与网络" description="千问模型与 API Key。">
      {pageError ? (
        <Alert variant="destructive">
          <AlertCircleIcon />
          <AlertTitle>配置未完成</AlertTitle>
          <AlertDescription>{pageError}</AlertDescription>
        </Alert>
      ) : null}

      <SettingsSection
        title="千问百炼"
        description="API Key 保存在 Windows 凭据管理器。"
      >
        <SettingRow
          title="连接状态"
          control={
            <Badge variant={configured ? "default" : "secondary"}>
              {configured ? <CheckCircle2Icon /> : <KeyRoundIcon />}
              {configured ? "已配置" : "未配置"}
            </Badge>
          }
        />
        <form onSubmit={handleSave}>
          <FieldGroup>
            <Field data-invalid={Boolean(credentialError)}>
              <FieldLabel htmlFor="qwen-api-key">API Key</FieldLabel>
              <Input
                id="qwen-api-key"
                type="password"
                value={apiKey}
                onChange={(event) => {
                  setApiKey(event.target.value);
                  setCredentialError("");
                }}
                placeholder={configured ? "••••••••••••••••" : "输入百炼 API Key"}
                autoComplete="off"
                aria-invalid={Boolean(credentialError)}
                disabled={credentialPending}
              />
              <FieldError>{credentialError}</FieldError>
            </Field>
            <Field orientation="horizontal">
              <Button type="submit" disabled={credentialPending || !apiKey.trim()}>
                {credentialPending ? <Spinner data-icon="inline-start" /> : null}
                {credentialPending ? "处理中…" : configured ? "替换密钥" : "保存密钥"}
              </Button>
              {configured ? (
                <Button type="button" variant="outline" disabled={credentialPending} onClick={handleDelete}>
                  移除密钥
                </Button>
              ) : null}
            </Field>
          </FieldGroup>
        </form>
      </SettingsSection>

      <SettingsSection
        title="语音识别"
        description="录音转文字。"
      >
        <SettingRow
          title="识别模型"
          description="固定使用支持即时热词和上下文增强的最新语音模型。"
          control={<Badge variant="secondary">Qwen Audio 3.0 ASR Flash</Badge>}
        />
        <SettingRow
          title="模型测试"
          description="点击后会录音 4 秒，请立即说“测试语音识别”。"
          control={
            <Button
              size="sm"
              variant="outline"
              disabled={!configured || controlsDisabled || testing.asr}
              onClick={() => void handleTest("asr")}
            >
              {testing.asr ? <Spinner data-icon="inline-start" /> : <MicIcon data-icon="inline-start" />}
              {testing.asr ? "正在录音，请说话…" : "测试语音识别"}
            </Button>
          }
        />
        <TestFeedback result={testResults.asr} />
      </SettingsSection>

      <SettingsSection
        title="文本整理"
        description="整理识别结果。"
      >
        <SettingRow
          title="整理模型"
          control={
            <Select
              value={models.rewriteModel}
              onValueChange={(value) => void handleRewriteModelChange(value as QwenRewriteModel | null)}
              disabled={controlsDisabled}
            >
              <SelectTrigger className="w-48"><SelectValue /></SelectTrigger>
              <SelectContent alignItemWithTrigger={false}>
                <SelectGroup>
                  {REWRITE_MODELS.map((model) => (
                    <SelectItem key={model.value} value={model.value}>{model.label}</SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
          }
        />
        <SettingRow
          title="模型测试"
          control={
            <Button
              size="sm"
              variant="outline"
              disabled={!configured || controlsDisabled || testing.rewrite}
              onClick={() => void handleTest("rewrite")}
            >
              {testing.rewrite ? <Spinner data-icon="inline-start" /> : <WandSparklesIcon data-icon="inline-start" />}
              {testing.rewrite ? "测试中…" : "测试文本整理"}
            </Button>
          }
        />
        <TestFeedback result={testResults.rewrite} />
      </SettingsSection>
    </SettingsPage>
  );
}

function TestFeedback({ result }: { result?: TestResult }) {
  if (!result) return null;
  const success = result.status === "success";
  return (
    <Alert variant={success ? "default" : "destructive"}>
      {success ? <CheckCircle2Icon /> : <AlertCircleIcon />}
      <AlertTitle>{success ? "测试通过" : "测试失败"}</AlertTitle>
      <AlertDescription>{result.message}</AlertDescription>
    </Alert>
  );
}
