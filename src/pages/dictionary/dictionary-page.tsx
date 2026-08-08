import { useEffect, useMemo, useState, type FormEvent } from "react";
import { BookOpenIcon, PencilIcon, PlusIcon, Trash2Icon } from "lucide-react";

import { PageHeader } from "@/components/app/page-header";
import { DataPagination } from "@/components/app/data-pagination";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty";
import { Field, FieldDescription, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Sheet, SheetContent, SheetDescription, SheetFooter, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { tauriClient } from "@/services/tauri-client";
import { DICTIONARY_CATEGORIES } from "@/shared/dictionary-config";
import type { DictionaryEntry, DictionaryEntryInput } from "@/shared/contracts";

const emptyDraft: DictionaryEntryInput = { source: "", replacement: "", category: "通用" };
const dictionaryDateFormatter = new Intl.DateTimeFormat("zh-CN");
const PAGE_SIZE = 10;

export function DictionaryPage() {
  const [entries, setEntries] = useState<DictionaryEntry[]>([]);
  const [draft, setDraft] = useState<DictionaryEntryInput>(emptyDraft);
  const [editorOpen, setEditorOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [formError, setFormError] = useState("");
  const [query, setQuery] = useState("");
  const [categoryFilter, setCategoryFilter] = useState("all");
  const [page, setPage] = useState(1);

  const filteredEntries = useMemo(() => {
    const keyword = query.trim().toLocaleLowerCase("zh-CN");
    return entries.filter((entry) => {
      const matchesCategory = categoryFilter === "all" || entry.category === categoryFilter;
      const matchesQuery = !keyword || [entry.source, entry.replacement, entry.category]
        .some((value) => value.toLocaleLowerCase("zh-CN").includes(keyword));
      return matchesCategory && matchesQuery;
    });
  }, [categoryFilter, entries, query]);
  const pageCount = Math.max(1, Math.ceil(filteredEntries.length / PAGE_SIZE));
  const currentPage = Math.min(page, pageCount);
  const pagedEntries = filteredEntries.slice((currentPage - 1) * PAGE_SIZE, currentPage * PAGE_SIZE);

  useEffect(() => {
    void tauriClient
      .listDictionary()
      .then(setEntries)
      .catch(() => setError("无法读取本地词典。"))
      .finally(() => setLoading(false));
  }, []);

  function openCreate() {
    setDraft(emptyDraft);
    setFormError("");
    setEditorOpen(true);
  }

  function openEdit(entry: DictionaryEntry) {
    setDraft({ id: entry.id, source: entry.source, replacement: entry.replacement, category: entry.category });
    setFormError("");
    setEditorOpen(true);
  }

  async function saveEntry(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!draft.source.trim() || !draft.replacement.trim() || !draft.category.trim()) {
      setFormError("请填写完整的识别文本、期望写法和分类。 ");
      return;
    }

    setSaving(true);
    setFormError("");
    try {
      const saved = await tauriClient.upsertDictionary(draft);
      setEntries((current) => {
        const next = current.filter((entry) => entry.id !== saved.id);
        return [...next, saved].sort((left, right) => left.category.localeCompare(right.category, "zh-CN") || left.source.localeCompare(right.source, "zh-CN"));
      });
      setEditorOpen(false);
    } catch {
      setFormError("保存失败；请检查是否已有相同的原始识别文本。 ");
    } finally {
      setSaving(false);
    }
  }

  async function deleteEntry(entry: DictionaryEntry) {
    if (!window.confirm(`确定删除词条“${entry.replacement}”吗？`)) return;
    try {
      await tauriClient.deleteDictionary(entry.id);
      setEntries((current) => current.filter((item) => item.id !== entry.id));
      if (draft.id === entry.id) setEditorOpen(false);
    } catch {
      setError("删除词条失败。 ");
    }
  }

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-6 p-8">
      <PageHeader title="词典" description="纠正常用名称、术语和专有词。" actions={<Button onClick={openCreate}><PlusIcon data-icon="inline-start" />添加词条</Button>} />
      {error ? <Alert variant="destructive"><AlertTitle>操作失败</AlertTitle><AlertDescription>{error}</AlertDescription></Alert> : null}

      {loading ? (
        <div className="flex min-h-48 items-center justify-center"><Spinner /></div>
      ) : entries.length === 0 ? (
        <Empty className="min-h-64 border">
          <EmptyHeader>
            <EmptyMedia variant="icon"><BookOpenIcon /></EmptyMedia>
            <EmptyTitle>还没有词典条目</EmptyTitle>
            <EmptyDescription>添加人名、产品名和专业术语，提高识别与整理结果的一致性。</EmptyDescription>
          </EmptyHeader>
          <EmptyContent><Button variant="outline" onClick={openCreate}><PlusIcon data-icon="inline-start" />添加第一条</Button></EmptyContent>
        </Empty>
      ) : (
        <div className="overflow-hidden rounded-xl border">
          <div className="flex flex-col gap-2 border-b p-3 sm:flex-row">
            <Input value={query} onChange={(event) => { setQuery(event.target.value); setPage(1); }} placeholder="检索原始识别或期望写法…" aria-label="检索词典" className="sm:max-w-sm" />
            <Select value={categoryFilter} onValueChange={(value) => { setCategoryFilter(value ?? "all"); setPage(1); }}>
              <SelectTrigger className="w-full sm:w-36"><SelectValue>{categoryFilter === "all" ? "全部分类" : categoryFilter}</SelectValue></SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectItem value="all">全部分类</SelectItem>
                  {DICTIONARY_CATEGORIES.map((category) => <SelectItem key={category} value={category}>{category}</SelectItem>)}
                </SelectGroup>
              </SelectContent>
            </Select>
          </div>
          {filteredEntries.length === 0 ? (
            <Empty className="min-h-48"><EmptyHeader><EmptyTitle>没有匹配的词条</EmptyTitle><EmptyDescription>尝试更换关键词或分类。</EmptyDescription></EmptyHeader></Empty>
          ) : (
            <>
              <Table>
                <TableHeader><TableRow><TableHead>原始识别</TableHead><TableHead>期望写法</TableHead><TableHead>分类</TableHead><TableHead>更新时间</TableHead><TableHead className="text-right">操作</TableHead></TableRow></TableHeader>
                <TableBody>
                  {pagedEntries.map((entry) => (
                    <TableRow key={entry.id}>
                      <TableCell>{entry.source}</TableCell>
                      <TableCell className="font-medium">{entry.replacement}</TableCell>
                      <TableCell><Badge variant="secondary">{entry.category}</Badge></TableCell>
                      <TableCell>{dictionaryDateFormatter.format(new Date(entry.updatedAt))}</TableCell>
                      <TableCell className="text-right">
                        <div className="flex justify-end gap-1">
                          <Button variant="ghost" size="icon-sm" aria-label="编辑" onClick={() => openEdit(entry)}><PencilIcon /></Button>
                          <Button variant="ghost" size="icon-sm" aria-label="删除" onClick={() => deleteEntry(entry)}><Trash2Icon /></Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
              <DataPagination page={currentPage} pageCount={pageCount} total={filteredEntries.length} pageSize={PAGE_SIZE} onPageChange={setPage} />
            </>
          )}
        </div>
      )}

      <Sheet open={editorOpen} onOpenChange={setEditorOpen}>
        <SheetContent>
          <SheetHeader>
            <SheetTitle>{draft.id ? "编辑词条" : "添加词条"}</SheetTitle>
            <SheetDescription>词条会作为语音识别上下文，并在整理前执行确定性纠正。</SheetDescription>
          </SheetHeader>
          <form className="flex min-h-0 flex-1 flex-col" onSubmit={saveEntry}>
            <FieldGroup className="px-4">
              <Field data-invalid={Boolean(formError)}>
                <FieldLabel htmlFor="dictionary-source">原始识别</FieldLabel>
                <Input id="dictionary-source" value={draft.source} onChange={(event) => setDraft({ ...draft, source: event.target.value })} placeholder="例如：扣得克斯" aria-invalid={Boolean(formError)} disabled={saving} />
                <FieldDescription>模型可能识别出的写法。</FieldDescription>
              </Field>
              <Field data-invalid={Boolean(formError)}>
                <FieldLabel htmlFor="dictionary-replacement">期望写法</FieldLabel>
                <Input id="dictionary-replacement" value={draft.replacement} onChange={(event) => setDraft({ ...draft, replacement: event.target.value })} placeholder="例如：Codex" aria-invalid={Boolean(formError)} disabled={saving} />
              </Field>
              <Field data-invalid={Boolean(formError)}>
                <FieldLabel htmlFor="dictionary-category">分类</FieldLabel>
                <Select value={draft.category} onValueChange={(value) => value && setDraft({ ...draft, category: value })} disabled={saving}>
                  <SelectTrigger id="dictionary-category" className="w-full" aria-invalid={Boolean(formError)}><SelectValue>{draft.category}</SelectValue></SelectTrigger>
                  <SelectContent>
                    <SelectGroup>
                      {DICTIONARY_CATEGORIES.map((category) => <SelectItem key={category} value={category}>{category}</SelectItem>)}
                    </SelectGroup>
                  </SelectContent>
                </Select>
                <FieldError>{formError}</FieldError>
              </Field>
            </FieldGroup>
            <SheetFooter>
              <Button type="submit" disabled={saving}>{saving ? <><Spinner data-icon="inline-start" />保存中…</> : "保存词条"}</Button>
              <Button type="button" variant="outline" onClick={() => setEditorOpen(false)} disabled={saving}>取消</Button>
            </SheetFooter>
          </form>
        </SheetContent>
      </Sheet>
    </div>
  );
}
