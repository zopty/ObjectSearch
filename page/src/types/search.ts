/**
 * Rustから受信する検索結果の候補アイテムの型
 */
export interface Candidate {
  key: string;
  label: string;
}

/**
 * Rustから受信する検索結果全体の型
 */
export interface SearchResult {
  candidates: Candidate[];
}