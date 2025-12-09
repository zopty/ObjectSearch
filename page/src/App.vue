<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue';
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
const navigateResults = (event: KeyboardEvent) => {
    const candidates = searchResults.value;
    if (candidates.length === 0) return;

    let currentIndex = selectedKey.value
        ? candidates.findIndex(c => c.name === selectedKey.value)
        : -1;

    if (event.key === 'ArrowDown') {
        currentIndex = (currentIndex + 1) % candidates.length;
    } else if (event.key === 'ArrowUp') {
        currentIndex = (currentIndex - 1 + candidates.length) % candidates.length;
    } else if (event.key === 'Enter') {
        if (selectedKey.value) {
            const confirmedCandidate = candidates.find(c => c.name === selectedKey.value);
            if (confirmedCandidate) {
                confirmCandidate(confirmedCandidate);
            }
        }
        return;
    }

    selectedKey.value = candidates[currentIndex]?.name ?? selectedKey.value;
    // スクロールして選択された要素を表示
    const selectedElement = document.querySelector('.candidate-item.selected');
    if (selectedElement) {
        selectedElement.scrollIntoView({ behavior: 'smooth', block: 'center' });
    } else {
        console.log("element not found");
    }
};

const reload = () => {
    ipc.reloadList().then((completed) => { if (completed) sendSearchQuery('') });
}

onMounted(() => {
    window.addEventListener('keydown', (event) => {
        // Prevent default scrolling behavior for arrow keys
        if (['ArrowUp', 'ArrowDown'].includes(event.key)) {
            event.preventDefault();
        }
        navigateResults(event);
    });

    sendSearchQuery('');
});

</script>

<template>
    <main class="page-root">
        <button id="reload" @click="reload">Reload</button>
        <input type="text" name="search" id="search" placeholder="検索..." @input="debouncedSendQuery"
            v-model="searchQuery">
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
$hover-bg: #444;

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

button#reload {
    outline: none;
    border: none;
    background: $bg;
    color: $fg;
    padding: 8px 1%;

    &:hover {
        background: $hover-bg;
    }

    &:active {
        background: $selected-bg;
    }
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
    overflow-y: auto;
}

.candidate-item {
    padding: 5px 1%;
    cursor: pointer;

    &:hover {
        background-color: $hover-bg;
    }

    &.selected {
        background-color: $selected-bg;
    }
}
</style>