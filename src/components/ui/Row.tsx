import type { ReactNode } from "react";

interface GroupProps {
  title: string;
  children: ReactNode;
}

export function Group({ title, children }: GroupProps) {
  return (
    <section className="mb-6">
      <h2 className="text-xs font-medium uppercase tracking-wide text-muted mb-2 px-1">
        {title}
      </h2>
      <div className="rounded-lg border border-border bg-surface divide-y divide-border">
        {children}
      </div>
    </section>
  );
}

interface RowProps {
  label: ReactNode;
  children?: ReactNode;
  caption?: string;
}

/** Title plus control. Only the Advanced page passes an InfoTip in `label`. */
export function Row({ label, children, caption }: RowProps) {
  return (
    <div className="px-3 py-2.5">
      <div className="flex items-center justify-between gap-4 min-h-7">
        <div className="flex items-center gap-1.5">{label}</div>
        <div className="flex items-center gap-2">{children}</div>
      </div>
      {caption && <div className="text-xs text-muted mt-1">{caption}</div>}
    </div>
  );
}

export function PageTitle({ children }: { children: ReactNode }) {
  return <h1 className="text-lg font-semibold mb-5">{children}</h1>;
}
