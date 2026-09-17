import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Sidebar, type Page } from "./components/Sidebar";

function App() {
  const { t } = useTranslation();
  const [page, setPage] = useState<Page>("general");

  return (
    <div className="h-full flex">
      <Sidebar active={page} onSelect={setPage} />
      <main className="flex-1 overflow-y-auto p-6">
        <h1 className="text-lg font-semibold mb-4">{t(`sidebar.${page}`)}</h1>
      </main>
    </div>
  );
}

export default App;
