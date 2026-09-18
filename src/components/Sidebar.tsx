import { useTranslation } from "react-i18next";
import {
  AudiblWordmark,
  BookIcon,
  ClockIcon,
  GearIcon,
  SlidersIcon,
} from "./ui/icons";

export type Page = "general" | "dictionary" | "history" | "advanced";

const PAGES: { page: Page; Icon: typeof SlidersIcon }[] = [
  { page: "general", Icon: SlidersIcon },
  { page: "dictionary", Icon: BookIcon },
  { page: "history", Icon: ClockIcon },
  { page: "advanced", Icon: GearIcon },
];

interface SidebarProps {
  active: Page;
  onSelect: (page: Page) => void;
}

export function Sidebar({ active, onSelect }: SidebarProps) {
  const { t } = useTranslation();

  return (
    // The window has no titlebar, so the rail runs to the top edge and the
    // pt-11 inset clears the traffic lights floating over it.
    <nav
      data-tauri-drag-region
      className="flex w-[220px] shrink-0 flex-col border-r border-border bg-sidebar px-3 pb-4 pt-11"
    >
      {/* The block is 60px tall — the PageTitle's 36px line plus its mb-6 —
          so the first tab's top lines up with the top of the first card on
          every page. The wordmark itself is scaled to 0.7x the tab column's
          width and centered within it. */}
      <div data-tauri-drag-region className="h-[60px] text-text">
        <AudiblWordmark className="pointer-events-none mx-auto h-auto w-[70%]" />
        <span className="sr-only">Audibl</span>
      </div>

      <div className="flex flex-col gap-0.5">
        {PAGES.map(({ page, Icon }) => {
          const selected = active === page;
          return (
            <button
              key={page}
              type="button"
              onClick={() => onSelect(page)}
              aria-current={selected ? "page" : undefined}
              className={`group flex items-center gap-2.5 rounded-control px-2.5 py-2 text-left text-base transition-colors ${
                selected
                  ? "bg-control font-medium text-text"
                  : "text-muted hover:bg-control/50 hover:text-text"
              }`}
            >
              <Icon
                size={16}
                className={selected ? "text-text" : "text-muted group-hover:text-text"}
              />
              {t(`sidebar.${page}`)}
            </button>
          );
        })}
      </div>

      <div className="mt-auto px-2.5 text-xs text-muted/70">
        {t("sidebar.slogan")}
      </div>
    </nav>
  );
}
