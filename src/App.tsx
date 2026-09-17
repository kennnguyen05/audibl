import { useEffect, useState } from "react";
import { Sidebar, type Page } from "./components/Sidebar";
import { General } from "./pages/General";
import { Dictionary } from "./pages/Dictionary";
import { History } from "./pages/History";
import { Advanced } from "./pages/Advanced";
import { initSettingsStore, useSettingsStore } from "./stores/settings";

const PAGES: Record<Page, () => JSX.Element | null> = {
  general: General,
  dictionary: Dictionary,
  history: History,
  advanced: Advanced,
};

function App() {
  const [page, setPage] = useState<Page>("general");
  const settings = useSettingsStore((s) => s.settings);

  useEffect(() => {
    initSettingsStore();
  }, []);

  if (!settings) return null;

  const ActivePage = PAGES[page];
  return (
    <div className="h-full flex">
      <Sidebar active={page} onSelect={setPage} />
      <main className="flex-1 overflow-y-auto px-6 py-5">
        <ActivePage />
      </main>
    </div>
  );
}

export default App;
