import { useEffect, useState } from "react";
import { ArrowRightIcon, BookOpenIcon, CheckCircle2Icon, HistoryIcon, MicIcon, Settings2Icon } from "lucide-react";
import { Link } from "react-router-dom";

import { PageHeader } from "@/components/app/page-header";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Kbd } from "@/components/ui/kbd";
import { Skeleton } from "@/components/ui/skeleton";
import { tauriClient } from "@/services/tauri-client";
import type { AppSettings, DashboardOverview } from "@/shared/contracts";
import { REWRITE_MODE_LABELS } from "@/shared/rewrite-mode-config";

interface HomeData {
  overview: DashboardOverview;
  settings: AppSettings;
}

export function HomePage() {
  const [data, setData] = useState<HomeData | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    void Promise.all([tauriClient.getDashboardOverview(), tauriClient.getSettings()])
      .then(([overview, settings]) => setData({ overview, settings }))
      .catch(() => setError("无法读取本地运行状态。"));
  }, []);

  const ready = data?.overview.qwenConfigured ?? false;

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-6 p-8">
      <PageHeader
        title="语音输入"
        description="在任意输入框中按住右 Alt 开始说话。"
        actions={!data ? <Skeleton className="h-8 w-24" /> : !ready ? <Button render={<Link to="/settings/models" />}><Settings2Icon data-icon="inline-start" />配置模型</Button> : undefined}
      />
      {error ? <Alert variant="destructive"><AlertTitle>状态读取失败</AlertTitle><AlertDescription>{error}</AlertDescription></Alert> : null}

      <Card className="bg-muted/20">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <span className="grid size-8 place-items-center rounded-lg bg-primary text-primary-foreground"><MicIcon className="size-4" /></span>
            {ready ? "可以开始了" : "还差一步"}
          </CardTitle>
          <CardDescription>{ready ? "长按说话，松开后会自动识别、整理并输入。" : "配置千问 API Key 后即可在任何应用中使用。"}</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <p className="mb-2 text-xs text-muted-foreground">按住说话</p>
            <Kbd className="h-10 px-4 text-base">Right Alt</Kbd>
          </div>
          <div className="flex flex-wrap gap-2">
            {ready ? <Badge><CheckCircle2Icon />模型已连接</Badge> : <Badge variant="secondary">等待配置</Badge>}
            {data ? <Badge variant="secondary">{REWRITE_MODE_LABELS[data.settings.rewriteMode]}</Badge> : <Skeleton className="h-6 w-20" />}
          </div>
        </CardContent>
        <CardFooter className="justify-between gap-4 text-xs text-muted-foreground">
          <span>轻点右 Alt 切换智能整理与原样转写</span>
          <Button variant="ghost" size="sm" render={<Link to="/settings/speech" />}>调整模式<ArrowRightIcon data-icon="inline-end" /></Button>
        </CardFooter>
      </Card>

      <div className="grid gap-4 sm:grid-cols-2">
        <Link to="/history" className="rounded-xl outline-none focus-visible:ring-3 focus-visible:ring-ring/50">
          <Card className="h-full transition-colors hover:bg-muted/30">
            <CardHeader>
              <CardTitle className="flex items-center gap-2"><HistoryIcon />历史记录</CardTitle>
              <CardDescription>{data ? `自动保留 ${data.settings.historyRetentionDays} 天，最多 1000 条。` : "仅保存在当前设备。"}</CardDescription>
            </CardHeader>
            <CardContent className="flex items-end justify-between">
              <p className="text-3xl font-semibold tabular-nums">{data?.overview.historyCount ?? "—"}</p>
              <ArrowRightIcon className="size-4 text-muted-foreground" />
            </CardContent>
          </Card>
        </Link>
        <Link to="/dictionary" className="rounded-xl outline-none focus-visible:ring-3 focus-visible:ring-ring/50">
          <Card className="h-full transition-colors hover:bg-muted/30">
            <CardHeader>
              <CardTitle className="flex items-center gap-2"><BookOpenIcon />词典</CardTitle>
              <CardDescription>让人名、产品名和专业术语识别得更准。</CardDescription>
            </CardHeader>
            <CardContent className="flex items-end justify-between">
              <p className="text-3xl font-semibold tabular-nums">{data?.overview.dictionaryCount ?? "—"}</p>
              <ArrowRightIcon className="size-4 text-muted-foreground" />
            </CardContent>
          </Card>
        </Link>
      </div>
    </div>
  );
}
