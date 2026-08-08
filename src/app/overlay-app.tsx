import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  AudioLinesIcon,
  CircleAlertIcon,
  FileTextIcon,
  TextCursorInputIcon,
  WandSparklesIcon,
  XIcon,
} from "lucide-react";
import { useEffect, useLayoutEffect, useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Kbd } from "@/components/ui/kbd";
import { Progress } from "@/components/ui/progress";
import { cn } from "@/lib/utils";
import type { VoiceSessionState } from "@/shared/contracts";

interface StageMetric {
  stage: "recording" | "transcribing" | "rewriting";
  elapsedMs: number;
}

function formatDuration(elapsedMs?: number) {
  if (elapsedMs === undefined) return undefined;
  return elapsedMs < 1_000 ? `${elapsedMs}ms` : `${(elapsedMs / 1_000).toFixed(1)}s`;
}

const stateContent = {
  arming: {
    icon: AudioLinesIcon,
    title: "准备语音输入",
    description: "继续按住右 Alt",
    badge: "准备中",
  },
  recording: {
    icon: AudioLinesIcon,
    title: "正在聆听",
    description: "松开右 Alt 完成",
    badge: "录音中",
  },
  transcribing: {
    icon: FileTextIcon,
    title: "正在转写",
    description: "正在识别你的语音",
    badge: "识别中",
  },
  rewriting: {
    icon: WandSparklesIcon,
    title: "正在整理",
    description: "正在清理和组织表达",
    badge: "智能清理",
  },
  injecting: {
    icon: TextCursorInputIcon,
    title: "正在输入",
    description: "即将写入当前输入框",
    badge: "写入中",
  },
  failed: {
    icon: CircleAlertIcon,
    title: "处理失败",
    description: "打开客户端查看详细信息",
    badge: "失败",
  },
  idle: {
    icon: AudioLinesIcon,
    title: "准备语音输入",
    description: "按住右 Alt 开始说话",
    badge: "待机",
  },
} satisfies Record<VoiceSessionState, {
  icon: typeof AudioLinesIcon;
  title: string;
  description: string;
  badge: string;
}>;

export function OverlayApp() {
  const [state, setState] = useState<VoiceSessionState>("recording");
  const [errorMessage, setErrorMessage] = useState("");
  const [inputLevel, setInputLevel] = useState(0);
  const [metrics, setMetrics] = useState<Partial<Record<StageMetric["stage"], number>>>({});
  const [isPointerOver, setIsPointerOver] = useState(false);
  const content = stateContent[state];
  const StateIcon = content.icon;

  useLayoutEffect(() => {
    document.documentElement.classList.add("voice-overlay-page", "dark");
    document.documentElement.style.colorScheme = "dark";
    return () => {
      document.documentElement.classList.remove("voice-overlay-page", "dark");
      document.documentElement.style.removeProperty("color-scheme");
    };
  }, []);

  useEffect(() => {
    const unlisten = listen<VoiceSessionState>(
      "voice://state-changed",
      (event) => {
        setState(event.payload);
        if (event.payload !== "failed") setErrorMessage("");
        if (event.payload === "arming" || event.payload === "recording") setMetrics({});
      },
    );
    const unlistenError = listen<string>("voice://error", (event) => setErrorMessage(event.payload));
    const unlistenLevel = listen<number>("voice://input-level", (event) => setInputLevel(event.payload));
    const unlistenMetric = listen<StageMetric>("voice://stage-metric", (event) => {
      setMetrics((current) => ({ ...current, [event.payload.stage]: event.payload.elapsedMs }));
    });

    return () => {
      void unlisten.then((dispose) => dispose());
      void unlistenError.then((dispose) => dispose());
      void unlistenLevel.then((dispose) => dispose());
      void unlistenMetric.then((dispose) => dispose());
    };
  }, []);

  useEffect(() => {
    if (state !== "failed" || isPointerOver) return;
    const timer = window.setTimeout(() => {
      void getCurrentWindow().hide();
    }, 10_000);
    return () => window.clearTimeout(timer);
  }, [isPointerOver, state]);

  const description = state === "rewriting" && metrics.transcribing !== undefined
    ? `识别 ${formatDuration(metrics.transcribing)} · 正在整理`
    : state === "injecting" && metrics.transcribing !== undefined
      ? `识别 ${formatDuration(metrics.transcribing)} · 整理 ${formatDuration(metrics.rewriting) ?? "完成"}`
      : content.description;

  return (
    <main className="flex min-h-svh items-center justify-center bg-transparent">
      <section
        aria-live="polite"
        className="flex min-h-svh w-full items-center gap-3 overflow-hidden rounded-2xl bg-background px-4 py-3"
        onPointerEnter={() => setIsPointerOver(true)}
        onPointerLeave={() => setIsPointerOver(false)}
      >
        <div className={cn(
          "grid size-9 shrink-0 place-items-center rounded-xl",
          state === "failed" ? "bg-destructive/10 text-destructive" : "bg-primary/10 text-primary",
        )}>
          <StateIcon className="size-4" />
        </div>
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <p className="truncate text-sm font-medium">{content.title}</p>
          <p className={cn("text-xs text-muted-foreground", state === "failed" ? "line-clamp-2" : "truncate")}>
            {state === "failed" && errorMessage ? errorMessage : description}
          </p>
          {state === "recording" ? (
            <Progress value={Math.max(2, inputLevel * 100)} aria-label="麦克风输入音量" className="mt-0.5" />
          ) : null}
        </div>
        {state !== "failed" ? (
          <Badge variant="secondary">
            {state === "recording" ? <Kbd>R Alt</Kbd> : content.badge}
          </Badge>
        ) : null}
        {state === "failed" ? (
          <Button variant="ghost" size="icon-lg" aria-label="关闭失败提示" onClick={() => void getCurrentWindow().hide()}>
            <XIcon />
          </Button>
        ) : null}
      </section>
    </main>
  );
}
