<script setup lang="ts">
import { ref, onMounted } from 'vue';
import type { Candidate, SearchResult } from './types/search';
// lodash-esなどのライブラリから debounce 関数をインポートするか、自作します
import { debounce } from 'es-toolkit';

// --- Vueの状態 ---
const searchQuery = ref('');
const searchResults = ref<Candidate[]>([]);
const selectedKey = ref<string | null>(null);

// --- Wryとの通信ロジック ---

/**
 * Rust側から結果を受け取るグローバル関数
 * この関数はWryのRustコードによって呼び出されます (window.receive_search_results)
 */
(window as any).receive_search_results = (result: SearchResult) => {
  console.log('Received results from Rust:', result);
  searchResults.value = result.candidates;
};


/**
 * 検索クエリをRustのバックエンドに送信する
 * @param query ユーザーが入力したクエリ
 */
const sendSearchQuery = (query: string) => {

  if ((window as any).rpc && (window as any).rpc.call) {
    (window as any).rpc.call(
      "search_request",
      { query: query }
    ).catch((error: any) => {
      console.error('RPC call error:', error);
    });
  } else {
    console.warn('Wry RPC object not found. Running in development mode?');
  }
};

// 入力イベントをデバウンス処理
const debouncedSendQuery = debounce((event: Event) => {
  const query = (event.target as HTMLInputElement).value;
  sendSearchQuery(query);
}, 200);


/**
 * 候補がクリックされたときに、そのキーを選択する
 * @param candidate 選択された候補
 */
const selectCandidate = (candidate: Candidate) => {
  selectedKey.value = candidate.key;
  console.log('Selected Key:', selectedKey.value);
  // 選択後、別のRPCコールでRust側に選択を通知するなどの処理も可能です。
};

// コンポーネントがマウントされたら、初期データをロードするために空のクエリを送信
onMounted(() => {
  sendSearchQuery('');
});
</script>

<template>
  <main class="page-root">
    <input type="text" name="search" id="search">

  </main>
</template>

<style lang="scss" scoped>
// colors
$bg: #333;
$fg: #ccc;
$text-bg: #111;

.page-root {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: $bg;
  color: $fg;
  font-family:
    ui-sans-serif,
    system-ui,
    -apple-system,
    Segoe UI,
    Roboto,
    Noto Sans,
    Arial,
    "Apple Color Emoji",
    "Segoe UI Emoji";
}

input#search {
  outline: none;
  background: $text-bg;
  color: $fg;
  width: 100%;
  padding: 1%;
  border: none;
}
</style>
