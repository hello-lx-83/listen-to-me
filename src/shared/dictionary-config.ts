export const DICTIONARY_CATEGORIES = ["通用", "产品", "人名", "专业术语"] as const;

export type DictionaryCategory = (typeof DICTIONARY_CATEGORIES)[number];
