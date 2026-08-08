import { useEffect, useMemo, useState, type FormEvent } from "react";
import { BookOpenIcon, CheckIcon, FolderCogIcon, PencilIcon, PlusIcon, Trash2Icon, XIcon } from "lucide-react";

import { PageHeader } from "@/components/app/page-header";
import { DataPagination } from "@/components/app/data-pagination";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty";
import { Field, FieldError, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Sheet, SheetContent, SheetDescription, SheetFooter, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { Spinner } from "@/components/ui/spinner";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { tauriClient } from "@/services/tauri-client";
import type { DictionaryCategory, DictionaryEntry, DictionaryEntryInput } from "@/shared/contracts";

const emptyDraft: DictionaryEntryInput = { source: "", replacement: "", category: "通用" };
const dictionaryDateFormatter = new Intl.DateTimeFormat("zh-CN");
const PAGE_SIZE = 10;

export function DictionaryPage() {
  const [entries, setEntries] = useState<DictionaryEntry[]>([]);
  const [categories, setCategories] = useState<DictionaryCategory[]>([]);
  const [draft, setDraft] = useState<DictionaryEntryInput>(emptyDraft);
  const [editorOpen, setEditorOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [formError, setFormError] = useState("");
  const [query, setQuery] = useState("");
  const [categoryFilter, setCategoryFilter] = useState("all");
  const [categoryManagerOpen, setCategoryManagerOpen] = useState(false);
  const [newCategory, setNewCategory] = useState("");
  const [editingCategory, setEditingCategory] = useState<string | null>(null);
  const [categoryName, setCategoryName] = useState("");
  const [categoryError, setCategoryError] = useState("");
  const [categorySaving, setCategorySaving] = useState(false);
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
    void Promise.all([tauriClient.listDictionary(), tauriClient.listDictionaryCategories()])
      .then(([dictionary, dictionaryCategories]) => {
        setEntries(dictionary);
        setCategories(dictionaryCategories);
      })
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
    if (!draft.replacement.trim() || !draft.category.trim()) {
      setFormError("请填写标准写法和分类。");
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
      void refreshCategories();
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
      void refreshCategories();
      if (draft.id === entry.id) setEditorOpen(false);
    } catch {
      setError("删除词条失败。 ");
    }
  }

  async function refreshCategories() {
    setCategories(await tauriClient.listDictionaryCategories());
  }

  async function createCategory(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!newCategory.trim()) return;
    setCategorySaving(true);
    setCategoryError("");
    try {
      await tauriClient.createDictionaryCategory(newCategory);
      setNewCategory("");
      await refreshCategories();
    } catch {
      setCategoryError("分类名称为空、过长或已经存在。");
    } finally {
      setCategorySaving(false);
    }
  }

  function startRename(category: DictionaryCategory) {
    setEditingCategory(category.name);
    setCategoryName(category.name);
    setCategoryError("");
  }

  async function renameCategory(category: DictionaryCategory) {
    if (!categoryName.trim() || categoryName.trim() === category.name) {
      setEditingCategory(null);
      return;
    }
    setCategorySaving(true);
    setCategoryError("");
    try {
      const renamed = await tauriClient.renameDictionaryCategory(category.name, categoryName);
      setEntries((current) => current.map((entry) => entry.category === category.name ? { ...entry, category: renamed.name } : entry));
      setDraft((current) => current.category === category.name ? { ...current, category: renamed.name } : current);
      setCategoryFilter((current) => current === category.name ? renamed.name : current);
      setEditingCategory(null);
      await refreshCategories();
    } catch {
      setCategoryError("无法重命名；请检查名称是否已经存在。");
    } finally {
      setCategorySaving(false);
    }
  }

  async function deleteCategory(category: DictionaryCategory) {
    const message = category.entryCount
      ? `删除“${category.name}”后，其中 ${category.entryCount} 个词条会移到“通用”。继续吗？`
      : `确定删除分类“${category.name}”吗？`;
    if (!window.confirm(message)) return;
    setCategorySaving(true);
    setCategoryError("");
    try {
      await tauriClient.deleteDictionaryCategory(category.name);
      setEntries((current) => current.map((entry) => entry.category === category.name ? { ...entry, category: "通用" } : entry));
      setDraft((current) => current.category === category.name ? { ...current, category: "通用" } : current);
      setCategoryFilter((current) => current === category.name ? "all" : current);
      await refreshCategories();
    } catch {
      setCategoryError("无法删除这个分类。");
    } finally {
      setCategorySaving(false);
    }
  }

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-6 p-8">
      <PageHeader title="词典" description="管理专有词和常见误识别。" actions={<div className="flex gap-2"><Button variant="outline" onClick={() => setCategoryManagerOpen(true)}><FolderCogIcon data-icon="inline-start" />管理分类</Button><Button onClick={openCreate}><PlusIcon data-icon="inline-start" />添加词条</Button></div>} />
      {error ? <Alert variant="destructive"><AlertTitle>操作失败</AlertTitle><AlertDescription>{error}</AlertDescription></Alert> : null}

      {loading ? (
        <div className="flex min-h-48 items-center justify-center"><Spinner /></div>
      ) : entries.length === 0 ? (
        <Empty className="min-h-64 border">
          <EmptyHeader>
            <EmptyMedia variant="icon"><BookOpenIcon /></EmptyMedia>
            <EmptyTitle>还没有词典条目</EmptyTitle>
            <EmptyDescription>先添加 Agent、MCP、产品名或人名。它们会参与识别，并在整理前后再次校正。</EmptyDescription>
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
                  {categories.map((category) => <SelectItem key={category.name} value={category.name}>{category.name}</SelectItem>)}
                </SelectGroup>
              </SelectContent>
            </Select>
          </div>
          {filteredEntries.length === 0 ? (
            <Empty className="min-h-48"><EmptyHeader><EmptyTitle>没有匹配的词条</EmptyTitle><EmptyDescription>尝试更换关键词或分类。</EmptyDescription></EmptyHeader></Empty>
          ) : (
            <>
              <Table>
                <TableHeader><TableRow><TableHead>标准写法</TableHead><TableHead>可能听成</TableHead><TableHead>分类</TableHead><TableHead>更新时间</TableHead><TableHead className="text-right">操作</TableHead></TableRow></TableHeader>
                <TableBody>
                  {pagedEntries.map((entry) => (
                    <TableRow key={entry.id}>
                      <TableCell className="font-medium">{entry.replacement}</TableCell>
                      <TableCell className="max-w-xs text-muted-foreground">{entry.source === entry.replacement ? "仅作为识别热词" : entry.source}</TableCell>
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
            <SheetDescription className="sr-only">填写词条信息</SheetDescription>
          </SheetHeader>
          <form className="flex min-h-0 flex-1 flex-col" onSubmit={saveEntry}>
            <FieldGroup className="px-4">
              <Field data-invalid={Boolean(formError)}>
                <FieldLabel htmlFor="dictionary-replacement">标准写法</FieldLabel>
                <Input id="dictionary-replacement" value={draft.replacement} onChange={(event) => setDraft({ ...draft, replacement: event.target.value })} placeholder="例如：Agent" aria-invalid={Boolean(formError)} disabled={saving} autoFocus />
              </Field>
              <Field>
                <FieldLabel htmlFor="dictionary-source">可能听成（可选）</FieldLabel>
                <Input id="dictionary-source" value={draft.source} onChange={(event) => setDraft({ ...draft, source: event.target.value })} placeholder="例如：智能体、诶真特、A gent" disabled={saving} />
              </Field>
              <Field data-invalid={Boolean(formError)}>
                <FieldLabel htmlFor="dictionary-category">分类</FieldLabel>
                <Select value={draft.category} onValueChange={(value) => value && setDraft({ ...draft, category: value })} disabled={saving}>
                  <SelectTrigger id="dictionary-category" className="w-full" aria-invalid={Boolean(formError)}><SelectValue>{draft.category}</SelectValue></SelectTrigger>
                  <SelectContent>
                    <SelectGroup>
                      {categories.map((category) => <SelectItem key={category.name} value={category.name}>{category.name}</SelectItem>)}
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

      <Sheet open={categoryManagerOpen} onOpenChange={setCategoryManagerOpen}>
        <SheetContent>
          <SheetHeader><SheetTitle>分类管理</SheetTitle><SheetDescription className="sr-only">添加、重命名或删除词典分类</SheetDescription></SheetHeader>
          <div className="flex min-h-0 flex-1 flex-col gap-4 px-4">
            <form className="flex gap-2" onSubmit={createCategory}>
              <Input value={newCategory} onChange={(event) => setNewCategory(event.target.value)} placeholder="新分类名称" maxLength={24} disabled={categorySaving} />
              <Button type="submit" disabled={categorySaving || !newCategory.trim()}><PlusIcon />添加</Button>
            </form>
            {categoryError ? <p role="alert" className="text-sm text-destructive">{categoryError}</p> : null}
            <div className="divide-y overflow-y-auto rounded-lg border">
              {categories.map((category) => (
                <div key={category.name} className="flex min-h-12 items-center gap-2 px-3 py-2">
                  {editingCategory === category.name ? (
                    <>
                      <Input value={categoryName} onChange={(event) => setCategoryName(event.target.value)} maxLength={24} disabled={categorySaving} autoFocus onKeyDown={(event) => { if (event.key === "Enter") { event.preventDefault(); void renameCategory(category); } }} />
                      <Button variant="ghost" size="icon-sm" aria-label="保存分类名称" onClick={() => void renameCategory(category)} disabled={categorySaving}><CheckIcon /></Button>
                      <Button variant="ghost" size="icon-sm" aria-label="取消编辑分类" onClick={() => setEditingCategory(null)} disabled={categorySaving}><XIcon /></Button>
                    </>
                  ) : (
                    <>
                      <span className="min-w-0 flex-1 truncate font-medium">{category.name}</span>
                      <span className="text-xs tabular-nums text-muted-foreground">{category.entryCount} 个词条</span>
                      {category.name === "通用" ? <Badge variant="secondary">默认</Badge> : (
                        <>
                          <Button variant="ghost" size="icon-sm" aria-label={`重命名${category.name}`} onClick={() => startRename(category)}><PencilIcon /></Button>
                          <Button variant="ghost" size="icon-sm" aria-label={`删除${category.name}`} onClick={() => void deleteCategory(category)}><Trash2Icon /></Button>
                        </>
                      )}
                    </>
                  )}
                </div>
              ))}
            </div>
          </div>
        </SheetContent>
      </Sheet>
    </div>
  );
}
