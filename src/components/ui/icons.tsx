// Hand-drawn so the stroke weight stays matched to the type. A handful of
// glyphs does not justify an icon-library dependency.
import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement> & { size?: number };

function Icon({ size = 16, children, ...props }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      {...props}
    >
      {children}
    </svg>
  );
}

/* Navigation ------------------------------------------------------------- */

export function SlidersIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M2 4.5h4M9.5 4.5H14M2 11.5h4.5M10 11.5H14" />
      <circle cx="7.75" cy="4.5" r="1.75" />
      <circle cx="8.25" cy="11.5" r="1.75" />
    </Icon>
  );
}

export function BookIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M2.5 3a1 1 0 0 1 1-1H7a1.5 1.5 0 0 1 1.5 1.5v10A1.25 1.25 0 0 0 7.25 12H2.5V3Z" />
      <path d="M13.5 3a1 1 0 0 0-1-1H9a1.5 1.5 0 0 0-1.5 1.5v10A1.25 1.25 0 0 1 8.75 12h4.75V3Z" />
    </Icon>
  );
}

export function ClockIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <circle cx="8" cy="8" r="6" />
      <path d="M8 4.75V8l2.25 1.5" />
    </Icon>
  );
}

export function GearIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <circle cx="8" cy="8" r="3.3" />
      <path d="M12.2 8h1.8M2 8h1.8M8 12.2v1.8M8 2v1.8M10.97 10.97l1.27 1.27M3.76 3.76l1.27 1.27M10.97 5.03l1.27-1.27M3.76 12.24l1.27-1.27" />
    </Icon>
  );
}

/* Actions ---------------------------------------------------------------- */

export function CheckIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M3 8.5 6.25 11.75 13 5" />
    </Icon>
  );
}

export function CloseIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M4 4l8 8M12 4l-8 8" />
    </Icon>
  );
}

export function PlusIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M8 3.25v9.5M3.25 8h9.5" />
    </Icon>
  );
}

export function PlusCircleIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <circle cx="8" cy="8" r="6" />
      <path d="M8 5.25v5.5M5.25 8h5.5" />
    </Icon>
  );
}

export function TrashIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M2.75 4.25h10.5M6.5 4.25V3a.75.75 0 0 1 .75-.75h1.5A.75.75 0 0 1 9.5 3v1.25" />
      <path d="M4.25 4.25 4.75 13a.75.75 0 0 0 .75.7h5a.75.75 0 0 0 .75-.7l.5-8.75" />
      <path d="M6.75 7v4M9.25 7v4" />
    </Icon>
  );
}

export function PencilIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M11.4 2.35a1.63 1.63 0 0 1 2.3 2.3L5.6 12.75l-3.1.8.8-3.1z" />
      <path d="M10.25 3.5l2.3 2.3" />
    </Icon>
  );
}

export function CopyIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <rect x="5.75" y="5.75" width="7.5" height="7.5" rx="1.5" />
      <path d="M10.25 3.75A1.5 1.5 0 0 0 8.75 2.25h-4.5a2 2 0 0 0-2 2v4.5a1.5 1.5 0 0 0 1.5 1.5" />
    </Icon>
  );
}

export function ChevronDownIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M4 6l4 4 4-4" />
    </Icon>
  );
}

export function ResetIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M13.25 8a5.25 5.25 0 1 1-1.6-3.77" />
      <path d="M13.5 2.25v3h-3" />
    </Icon>
  );
}

export function MicIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <rect x="6" y="1.75" width="4" height="7.5" rx="2" />
      <path d="M3.5 7.5a4.5 4.5 0 0 0 9 0M8 12v2.25" />
    </Icon>
  );
}

export function KeyboardIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <rect x="1.5" y="4" width="13" height="8" rx="1.75" />
      <path d="M4.25 6.75h.01M6.75 6.75h.01M9.25 6.75h.01M11.75 6.75h.01M5.5 9.5h5" />
    </Icon>
  );
}

export function GlobeIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <circle cx="8" cy="8" r="6" />
      <path d="M2 8h12M8 2c1.8 1.8 2.7 3.9 2.7 6S9.8 12.2 8 14c-1.8-1.8-2.7-3.9-2.7-6S6.2 3.8 8 2Z" />
    </Icon>
  );
}

export function DownloadIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M8 2v7.5M5 6.75 8 9.75l3-3" />
      <path d="M2.5 11.25v1.5a1.25 1.25 0 0 0 1.25 1.25h8.5a1.25 1.25 0 0 0 1.25-1.25v-1.5" />
    </Icon>
  );
}

/* The brand mark, traced from `brand-assets/Audibl icon.jpg`: seven
   square-ended bars in a symmetric arch, the middle one lifted clear of the
   baseline. Same table as `MARK_BARS` in scripts/generate-icons.ts, so the
   sidebar, the app icon and the tray icon are one drawing. */
const MARK_BARS: [number, number][] = [
  [0.5, 1],
  [0.25, 1],
  [0, 0.75],
  [0, 0.5],
  [0, 0.75],
  [0.25, 1],
  [0.5, 1],
];
const MARK_BAR_W = 7.5;
const MARK_PITCH = 13.75;

export function AudiblMark({ size = 18, ...props }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 96 96"
      fill="currentColor"
      aria-hidden="true"
      {...props}
    >
      {MARK_BARS.map(([top, bottom], i) => (
        <rect
          key={i}
          x={3 + i * MARK_PITCH}
          y={top * 96}
          width={MARK_BAR_W}
          height={(bottom - top) * 96}
        />
      ))}
    </svg>
  );
}

/* The full wordmark, traced from `brand-assets/Audibl full logo.png`: the
   same bar-and-pitch grammar as MARK_BARS above, stretched to 36 bars that
   spell "Audibl" as a waveform. Same viewBox height (96) and per-bar geometry
   convention as AudiblMark, so both scale from one `size` prop cleanly. */
const WORDMARK_BARS: [number, number][] = [
  [0.491, 0.986],
  [0.256, 0.986],
  [0.003, 0.737],
  [0.003, 0.495],
  [0.003, 0.737],
  [0.256, 0.986],
  [0.491, 0.986],
  [0.0, 0.74],
  [0.0, 0.983],
  [0.256, 0.99],
  [0.256, 0.986],
  [0.256, 0.99],
  [0.0, 0.983],
  [0.0, 0.74],
  [0.007, 0.99],
  [0.007, 0.99],
  [0.007, 0.99],
  [0.007, 0.99],
  [0.007, 0.997],
  [0.007, 0.99],
  [0.253, 0.744],
  [0.0, 0.983],
  [0.0, 0.983],
  [0.0, 0.99],
  [0.007, 0.99],
  [0.007, 0.99],
  [0.007, 0.997],
  [0.007, 0.99],
  [0.007, 0.997],
  [0.007, 0.99],
  [0.131, 0.872],
  [0.007, 0.99],
  [0.007, 0.99],
  [0.007, 0.997],
  [0.626, 0.997],
  [0.626, 1.0],
];
// Cropped to the ink, with no side padding, so the first and last bars sit
// exactly on whatever inset the caller lays the wordmark out in.
const WORDMARK_WIDTH = (WORDMARK_BARS.length - 1) * MARK_PITCH + MARK_BAR_W;

/** Sized by `size` (height) or, when given, by `width` (height follows). */
export function AudiblWordmark({
  size = 18,
  width: fixedWidth,
  ...props
}: Omit<IconProps, "width"> & { width?: number }) {
  const height = fixedWidth ? (fixedWidth / WORDMARK_WIDTH) * 96 : size;
  const width = fixedWidth ?? (WORDMARK_WIDTH / 96) * size;
  return (
    <svg
      width={width}
      height={height}
      viewBox={`3 0 ${WORDMARK_WIDTH} 96`}
      fill="currentColor"
      aria-hidden="true"
      {...props}
    >
      {WORDMARK_BARS.map(([top, bottom], i) => (
        <rect
          key={i}
          x={3 + i * MARK_PITCH}
          y={top * 96}
          width={MARK_BAR_W}
          height={(bottom - top) * 96}
        />
      ))}
    </svg>
  );
}
