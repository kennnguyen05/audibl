import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { commands, type HistoryEntry } from "@/bindings";

interface HistoryState {
  entries: HistoryEntry[];
}

export const useHistoryStore = create<HistoryState>(() => ({ entries: [] }));

let initialized = false;

export const initHistoryStore = async () => {
  if (initialized) return;
  initialized = true;
  useHistoryStore.setState({ entries: await commands.getHistory() });
  await listen<HistoryEntry[]>("history-changed", (event) => {
    useHistoryStore.setState({ entries: event.payload });
  });
};

export const deleteHistoryEntry = async (timestamp: number) => {
  useHistoryStore.setState({
    entries: await commands.deleteHistoryEntry(timestamp),
  });
};
