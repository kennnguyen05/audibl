import { useTranslation } from "react-i18next";

export type Page = "general" | "dictionary" | "history" | "advanced";

const PAGES: Page[] = ["general", "dictionary", "history", "advanced"];

interface SidebarProps {
  active: Page;
  onSelect: (page: Page) => void;
}

export function Sidebar({ active, onSelect }: SidebarProps) {
  const { t } = useTranslation();
  return (
    <nav className="w-44 shrink-0 bg-sidebar border-r border-border p-3 flex flex-col gap-1">
      <div className="px-2 pt-1 pb-3 text-sm font-semibold">Audible</div>
      {PAGES.map((page) => (
        <button
          key={page}
          type="button"
          onClick={() => onSelect(page)}
          className={`text-left px-2 py-1.5 rounded-md ${
            active === page
              ? "bg-control font-medium"
              : "text-muted hover:bg-control/60"
          }`}
        >
          {t(`sidebar.${page}`)}
        </button>
      ))}
    </nav>
  );
}
