import type { RewriteMode } from "@/shared/contracts";

export const CORE_REWRITE_MODES = [
  { value: "clean", label: "智能整理", description: "清理口头禅、重复和明显错字；多个事项会自动分段。" },
  { value: "raw", label: "原样转写", description: "只做识别和词典校正，保留原本说法。" },
] as const satisfies ReadonlyArray<{ value: RewriteMode; label: string; description: string }>;

export const REWRITE_MODE_LABELS: Record<RewriteMode, string> = {
  raw: "原样转写",
  clean: "智能整理",
  article: "书面整理（旧版）",
  structured: "要点整理",
};
