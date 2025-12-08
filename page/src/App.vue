<script setup lang="ts">
import { ref, onMounted } from 'vue';
import type { Candidate, SearchResult } from './lib/types/search';
import { debounce } from 'es-toolkit';
import { ipc } from './lib/ipc';

// --- Vueの状態 ---
const searchQuery = ref('');
const searchResults = ref<Candidate[]>([]);
const selectedKey = ref<string | null>(null);

// --- Wryとの通信ロジック ---

/**
 * Rust側から結果を受け取るグローバル関数
 */
ipc.on("search_result", (result) => {
  try {
    searchResults.value = (result as SearchResult).candidates;
  } catch (e) {
    console.error(e);
  }
});

/**
 * 検索クエリをRustのバックエンドに送信する
 * @param query ユーザーが入力したクエリ
 */
const sendSearchQuery = (query: string) => {
  ipc.searchQuery(query).then((result) => { return result });
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
  selectedKey.value = candidate.name;
  console.log('Selected Key:', selectedKey.value);
  // 選択後、別のRPCコールでRust側に選択を通知するなどの処理も可能です。
};

/**
 * 候補がダブルクリックされたときに、そのキーを選択し、Wry側に通知する
 * @param candidate 選択された候補
 */

const confirmCandidate = (candidate: Candidate) => {
  selectedKey.value = candidate.name;
  ipc.confirmCandidate(candidate.name);
  console.log('Confirmed Key:', selectedKey.value);
};
// キーボードナビゲーション

onMounted(() => {
  sendSearchQuery('');
});

</script>

<template>
  <main class="page-root">
    <input type="text" name="search" id="search" @input="debouncedSendQuery" v-model="searchQuery">
    <div class="search-results">
      <div v-for="candidate in searchResults" :key="candidate.name" class="candidate-item"
        :class="{ selected: candidate.name === selectedKey }" @click="selectCandidate(candidate)"
        @dblclick="confirmCandidate(candidate)">
        {{ candidate.name }}
      </div>
    </div>
  </main>
</template>

<style lang="scss" scoped>
// colors
$bg: #333;
$fg: #ccc;
$text-bg: #111;
$selected-bg: #555;

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

.search-results {
  flex-grow: 1;
  overflow-y: auto;
}

.candidate-item {
  padding: 8px 1%;
  cursor: pointer;

  &:hover {
    background-color: lighten($bg, 10%);
  }

  &.selected {
    background-color: $selected-bg;
  }
}
</style>