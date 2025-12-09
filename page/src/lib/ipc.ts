import { getBridge } from "./bridge.ts";
import type { SearchResult } from "./types/search.ts";

const bridge = getBridge();

const onceWaiters = new Map<string, Array<(data: unknown) => void>>();
const subscribers = new Map<string, Array<(data: unknown) => void>>();

bridge.onMessage((type, data) => {
    const onceList = onceWaiters.get(type);
    if (onceList && onceList.length) {
        const callbacks = onceList.splice(0, onceList.length);
        onceWaiters.delete(type);
        for (const cb of callbacks) {
            try {
                cb(data);
            } catch (e) {
                console.error(e);
            }
        }
    }

    const subs = subscribers.get(type);
    if (subs && subs.length) {
        for (const cb of subs) {
            try {
                cb(data);
            } catch (e) {
                console.error(e);
            }
        }
    }
});

function waitFor<T = unknown>(type: string): Promise<T> {
    return new Promise<T>((resolve) => {
        const list = onceWaiters.get(type) ?? [];
        list.push((data) => resolve(data as T));
        onceWaiters.set(type, list);
    });
}

export const ipc = {
    async searchQuery(query: String): Promise<SearchResult> {
        console.log("searching...");
        bridge.send("search", query);
        const res = await waitFor<SearchResult>("search_result");
        console.log("result:");
        console.log(res);
        return res;
    },

    async reloadList(): Promise<boolean> {
        console.log("reloading...");
        bridge.send("reload", undefined);
        const res = await waitFor<{ completed: boolean }>("reload_result");
        console.log("result:");
        console.log(res);
        return res.completed ?? false;
    },

    confirmCandidate(name: String): void {
        bridge.send("select", name);
    },

    on(type: string, cb: (data: unknown) => void): void {
        const list = subscribers.get(type) ?? [];
        list.push(cb);
        subscribers.set(type, list);
    },
};
