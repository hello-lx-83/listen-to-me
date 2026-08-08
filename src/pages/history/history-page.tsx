import { useEffect, useMemo, useState } from "react";
import { ClipboardIcon, HistoryIcon, Trash2Icon } from "lucide-react";

import { PageHeader } from "@/components/app/page-header";
import { DataPagination } from "@/components/app/data-pagination";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty";
import { Input } from "@/components/ui/input";
import { Sheet, SheetContent, SheetDescription, SheetFooter, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { tauriClient } from "@/services/tauri-client";
import type { HistoryRecord, RewriteMode } from "@/shared/contracts";

const modeLabels: Record<RewriteMode, string> = {
  raw: "原样",
  clean: "智能清理",
  article: "整理成文",
  structured: "结构化",
};

const historyDateFormatter = new Intl.DateTimeFormat("zh-CN", {
  month: "2-digit",
  day: "2-digit",
  hour: "2-digit",
  minute: "2-digit",
});
const PAGE_SIZE = 8;

export function HistoryPage() {
  const [records, setRecords] = useState<HistoryRecord[]>([]);
  const [selected, setSelected] = useState<HistoryRecord | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [query, setQuery] = useState("");
  const [page, setPage] = useState(1);

  const filteredRecords = useMemo(() => {
    const keyword = query.trim().toLocaleLowerCase("zh-CN");
    if (!keyword) return records;
    return records.filter((record) => [record.transcript, record.output, modeLabels[record.mode]]
      .some((value) => value.toLocaleLowerCase("zh-CN").includes(keyword)));
  }, [query, records]);
  const pageCount = Math.max(1, Math.ceil(filteredRecords.length / PAGE_SIZE));
  const currentPage = Math.min(page, pageCount);
  const pagedRecords = filteredRecords.slice((currentPage - 1) * PAGE_SIZE, currentPage * PAGE_SIZE);

  useEffect(() => {
    void loadHistory();
  }, []);

  async function loadHistory() {
    setLoading(true);
    setError("");
    try {
      setRecords(await tauriClient.listHistory(1_000));
    } catch {
      setError("无法读取本地历史记录。 ");
    } finally {
      setLoading(false);
    }
  }

  async function deleteRecord(record: HistoryRecord) {
    if (!window.confirm("确定删除这条历史记录吗？")) return;
    try {
      await tauriClient.deleteHistory(record.id);
      setRecords((current) => current.filter((item) => item.id !== record.id));
      if (selected?.id === record.id) setSelected(null);
    } catch {
      setError("删除历史记录失败。 ");
    }
  }

  async function clearAll() {
    if (!records.length || !window.confirm("确定清空全部历史记录吗？此操作无法撤销。")) return;
    try {
      await tauriClient.clearHistory();
      setRecords([]);
      setSelected(null);
    } catch {
      setError("清空历史记录失败。 ");
    }
  }

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-6 p-8">
      <PageHeader
        title="历史记录"
        description="查看原始识别文本和整理后的结果。"
        actions={records.length ? <Button variant="outline" onClick={clearAll}><Trash2Icon data-icon="inline-start" />清空</Button> : undefined}
      />

      {error ? <Alert variant="destructive"><AlertTitle>操作失败</AlertTitle><AlertDescription>{error}</AlertDescription></Alert> : null}

      {loading ? (
        <div className="flex min-h-48 items-center justify-center"><Spinner /></div>
      ) : records.length === 0 ? (
        <Empty className="min-h-64 border">
          <EmptyHeader>
            <EmptyMedia variant="icon"><HistoryIcon /></EmptyMedia>
            <EmptyTitle>还没有历史记录</EmptyTitle>
            <EmptyDescription>完成一次语音输入后，原始识别和整理结果会保存在这里。</EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : (
        <div className="overflow-hidden rounded-xl border">
          <div className="flex items-center border-b p-3">
            <Input
              value={query}
              onChange={(event) => { setQuery(event.target.value); setPage(1); }}
              placeholder="检索识别原文、整理结果或模式…"
              aria-label="检索历史记录"
              className="max-w-sm"
            />
          </div>
          {filteredRecords.length === 0 ? (
            <Empty className="min-h-48">
              <EmptyHeader><EmptyTitle>没有匹配的历史记录</EmptyTitle><EmptyDescription>尝试更换检索关键词。</EmptyDescription></EmptyHeader>
            </Empty>
          ) : (
            <>
              <Table>
                <TableHeader>
                  <TableRow><TableHead>时间</TableHead><TableHead>模式</TableHead><TableHead>结果摘要</TableHead><TableHead className="text-right">操作</TableHead></TableRow>
                </TableHeader>
                <TableBody>
                  {pagedRecords.map((record) => (
                    <TableRow key={record.id}>
                      <TableCell>{formatDate(record.createdAt)}</TableCell>
                      <TableCell><Badge variant="secondary">{modeLabels[record.mode]}</Badge></TableCell>
                      <TableCell className="max-w-md truncate">{record.output}</TableCell>
                      <TableCell className="text-right">
                        <div className="flex justify-end gap-1">
                          <Button variant="ghost" size="sm" onClick={() => setSelected(record)}>查看</Button>
                          <Button variant="ghost" size="icon-sm" aria-label="删除" onClick={() => deleteRecord(record)}><Trash2Icon /></Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
              <DataPagination page={currentPage} pageCount={pageCount} total={filteredRecords.length} pageSize={PAGE_SIZE} onPageChange={setPage} />
            </>
          )}
        </div>
      )}

      <Sheet open={Boolean(selected)} onOpenChange={(open) => { if (!open) setSelected(null); }}>
        <SheetContent className="sm:max-w-lg">
          <SheetHeader>
            <SheetTitle>输入详情</SheetTitle>
            <SheetDescription>{selected ? `${formatDate(selected.createdAt)} · ${modeLabels[selected.mode]}` : ""}</SheetDescription>
          </SheetHeader>
          {selected ? (
            <div className="flex flex-1 flex-col gap-5 overflow-y-auto px-4">
              <section className="flex flex-col gap-2">
                <h3 className="text-sm font-medium">原始识别</h3>
                <p className="whitespace-pre-wrap text-sm text-muted-foreground">{selected.transcript}</p>
              </section>
              <section className="flex flex-col gap-2">
                <h3 className="text-sm font-medium">整理结果</h3>
                <p className="whitespace-pre-wrap text-sm">{selected.output}</p>
              </section>
            </div>
          ) : null}
          <SheetFooter>
            <Button onClick={() => selected && navigator.clipboard.writeText(selected.output)}><ClipboardIcon data-icon="inline-start" />复制结果</Button>
            <Button variant="outline" onClick={() => selected && deleteRecord(selected)}><Trash2Icon data-icon="inline-start" />删除记录</Button>
          </SheetFooter>
        </SheetContent>
      </Sheet>
    </div>
  );
}

function formatDate(timestamp: number) {
  return historyDateFormatter.format(new Date(timestamp));
}
