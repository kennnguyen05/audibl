import type { ReactNode } from "react";

interface GroupProps {
  title?: ReactNode;
  /** Sits opposite the title: an Add button, a count, a filter. */
  action?: ReactNode;
  children: ReactNode;
}

/** A card of divided rows. Cards carry no shadow — depth is the glass itself:
 *  a translucent pane over the shader background plus a hairline edge. */
export function Group({ title, action, children }: GroupProps) {
  return (
    <section className="mb-7 last:mb-0">
      {(title || action) && (
        <header className="mb-2.5 flex min-h-8 items-center justify-between gap-4 px-0.5">
          {title ? (
            <h2 className="text-base font-semibold text-text">{title}</h2>
          ) : (
            <span />
          )}
          {action}
        </header>
      )}
      <div className="divide-y divide-hairline overflow-hidden rounded-card border border-hairline glass-card">
        {children}
      </div>
    </section>
  );
}

interface RowProps {
  label: ReactNode;
  /** The muted line under the label. Prefer this over a hover tooltip. */
  caption?: ReactNode;
  children?: ReactNode;
  /** Stacks the control under the label, full width, for wide controls. */
  stacked?: boolean;
}

export function Row({ label, caption, children, stacked }: RowProps) {
  return (
    <div className="px-5 py-3.5">
      <div
        className={
          stacked ? "flex flex-col gap-3" : "flex items-center justify-between gap-6"
        }
      >
        <div className="min-w-0">
          <div className="flex items-center gap-1.5 text-base text-text">
            {label}
          </div>
          {caption && (
            <div className="mt-0.5 text-sm leading-snug text-muted">{caption}</div>
          )}
        </div>
        {children && (
          <div
            className={
              stacked
                ? "w-full"
                : "flex shrink-0 items-center gap-2"
            }
          >
            {children}
          </div>
        )}
      </div>
    </div>
  );
}

/** Invitation to act, shown in place of an empty card body. */
export function EmptyRow({ children }: { children: ReactNode }) {
  return (
    <p className="px-5 py-6 text-center text-sm text-muted">{children}</p>
  );
}

/* .title-glow in index.css carries both the neon and the dark field under it. */
export function PageTitle({ children }: { children: ReactNode }) {
  return (
    <h1 className="title-glow mb-6 text-2xl font-normal tracking-tight">
      {children}
    </h1>
  );
}

/** Page title with an action opposite it, per the reference's Dictionary. */
export function PageHeader({
  title,
  action,
}: {
  title: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="mb-6 flex items-center justify-between gap-4">
      <h1 className="title-glow text-2xl font-normal tracking-tight">
        {title}
      </h1>
      {action}
    </div>
  );
}
