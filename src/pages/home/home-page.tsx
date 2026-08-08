import { useEffect, useState } from "react";
import { AudioLinesIcon, BookOpenIcon, CloudIcon, HistoryIcon, MicIcon } from "lucide-react";

import { PageHeader } from "@/components/app/page-header";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Kbd } from "@/components/ui/kbd";
import { Skeleton } from "@/components/ui/skeleton";
import { tauriClient } from "@/services/tauri-client";

export function HomePage() {
  const [overview, setOverview] = useState<Awaited<ReturnType<typeof tauriClient.getDashboardOverview>> | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    void tauriClient.getDashboardOverview()
      .then(setOverview)
      .catch(() => setError("无法读取本地运行状态。"));
  }, []);

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-6 p-8">
      <PageHeader title="首页" description="按住右 Alt，在任意输入框中开始语音输入。" />
      {error ? <Alert variant="destructive"><AlertTitle>状态读取失败</AlertTitle><AlertDescription>{error}</AlertDescription></Alert> : null}

      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2"><MicIcon />语音输入</CardTitle>
            <CardDescription>长按开始录音，松开后自动转写、整理并写入当前输入框。</CardDescription>
          </CardHeader>
          <CardContent className="flex items-center justify-between gap-4">
            {overview ? <Badge variant={overview.qwenConfigured ? "default" : "secondary"}>{overview.qwenConfigured ? "可以使用" : "需要配置模型"}</Badge> : <Skeleton className="h-6 w-20" />}
            <p className="text-sm text-muted-foreground">快捷键 <Kbd>Right Alt</Kbd></p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2"><CloudIcon />模型线路</CardTitle>
            <CardDescription>当前使用千问云端语音识别与低延迟文本整理。</CardDescription>
          </CardHeader>
          <CardContent className="flex items-center justify-between gap-4">
            <Badge variant="secondary">云端</Badge>
            <span className="flex items-center gap-2 text-sm text-muted-foreground"><AudioLinesIcon />非流式处理</span>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2"><HistoryIcon />历史记录</CardTitle>
            <CardDescription>仅保存在当前设备中的文本记录。</CardDescription>
          </CardHeader>
          <CardContent><p className="text-2xl font-medium tabular-nums">{overview?.historyCount ?? "—"}</p></CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2"><BookOpenIcon />词典</CardTitle>
            <CardDescription>参与识别上下文和确定性术语纠正。</CardDescription>
          </CardHeader>
          <CardContent><p className="text-2xl font-medium tabular-nums">{overview?.dictionaryCount ?? "—"}</p></CardContent>
        </Card>
      </div>
    </div>
  );
}
