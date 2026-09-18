// Generates the app icon and the menu bar (tray) template icons.
// Run: bun scripts/generate-icons.ts && bun tauri icon src-tauri/icons/app-icon.png
//
// The mark is the brand one, traced from `brand-assets/Audibl icon.jpg`: seven
// square-ended bars in a symmetric arch, the middle bar lifted clear of the
// baseline. The bar table matches `MARK_BARS` in src/components/ui/icons.tsx,
// so the sidebar, the app icon and the tray icon are one drawing.
//
// Tray icons are macOS template images: alpha only, painted pure black, and
// tinted by the system. State therefore has to read from silhouette alone, so
// the three are deliberately different shapes rather than a subtle family.
import fs from "fs";
import path from "path";
import zlib from "zlib";
import { fileURLToPath } from "url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");

type Rgba = [number, number, number, number];
type Shader = (x: number, y: number) => Rgba | null;

function crc32(buf: Buffer): number {
  let c = ~0;
  for (let i = 0; i < buf.length; i++) {
    c ^= buf[i];
    for (let k = 0; k < 8; k++) c = c & 1 ? (c >>> 1) ^ 0xedb88320 : c >>> 1;
  }
  return ~c >>> 0;
}

function chunk(type: string, data: Buffer): Buffer {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}

// Renders with 4x4 supersampling so edges are anti-aliased.
function renderPng(size: number, shade: Shader): Buffer {
  const ss = 4;
  const raw = Buffer.alloc((size * 4 + 1) * size);
  for (let y = 0; y < size; y++) {
    raw[y * (size * 4 + 1)] = 0;
    for (let x = 0; x < size; x++) {
      let r = 0, g = 0, b = 0, a = 0;
      for (let sy = 0; sy < ss; sy++) {
        for (let sx = 0; sx < ss; sx++) {
          const px = shade((x + (sx + 0.5) / ss) / size, (y + (sy + 0.5) / ss) / size);
          if (px) {
            const pa = px[3] / 255;
            r += px[0] * pa;
            g += px[1] * pa;
            b += px[2] * pa;
            a += pa;
          }
        }
      }
      const o = y * (size * 4 + 1) + 1 + x * 4;
      const n = ss * ss;
      raw[o] = a ? Math.round(r / a) : 0;
      raw[o + 1] = a ? Math.round(g / a) : 0;
      raw[o + 2] = a ? Math.round(b / a) : 0;
      raw[o + 3] = Math.round((a / n) * 255);
    }
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", zlib.deflateSync(raw)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

// Signed distance to a rounded rectangle centred at (cx, cy).
function roundedRect(x: number, y: number, cx: number, cy: number, hw: number, hh: number, r: number): boolean {
  const dx = Math.max(Math.abs(x - cx) - (hw - r), 0);
  const dy = Math.max(Math.abs(y - cy) - (hh - r), 0);
  return dx * dx + dy * dy <= r * r;
}

// The mark, drawn in a 0..1 box. `scale` is the side of its square 96-unit
// grid; the bars span 90 of those 96 units, so a little air is built in.
// Each entry is one bar's [top, bottom] as a fraction of that grid.
const MARK_BARS: [number, number][] = [
  [0.5, 1],
  [0.25, 1],
  [0, 0.75],
  [0, 0.5],
  [0, 0.75],
  [0.25, 1],
  [0.5, 1],
];
const MARK_BAR_W = 7.5 / 96;
const MARK_PITCH = 13.75 / 96;

function mark(x: number, y: number, scale: number, cy = 0.5): boolean {
  const left = 0.5 - (90 / 96) * scale * 0.5;
  const top = cy - scale / 2;
  for (let i = 0; i < MARK_BARS.length; i++) {
    const bx = left + i * MARK_PITCH * scale;
    if (x < bx || x > bx + MARK_BAR_W * scale) continue;
    const [t, b] = MARK_BARS[i];
    if (y >= top + t * scale && y <= top + b * scale) return true;
  }
  return false;
}

const BLACK: Rgba = [0, 0, 0, 255];

// The brand asset is a black mark on white, so the Dock icon inverts the app's
// dark palette: warm off-white tile, ink mark. It is the one surface the
// dark-only rule does not own.
const PAPER_TOP: Rgba = [250, 247, 241, 255];
const PAPER_BOTTOM: Rgba = [232, 226, 215, 255];
const INK: Rgba = [22, 20, 18, 255];

const appIcon: Shader = (x, y) => {
  if (!roundedRect(x, y, 0.5, 0.5, 0.44, 0.44, 0.2)) return null;
  if (mark(x, y, 0.46)) return INK;
  const t = y;
  return [
    Math.round(PAPER_TOP[0] + (PAPER_BOTTOM[0] - PAPER_TOP[0]) * t),
    Math.round(PAPER_TOP[1] + (PAPER_BOTTOM[1] - PAPER_TOP[1]) * t),
    Math.round(PAPER_TOP[2] + (PAPER_BOTTOM[2] - PAPER_TOP[2]) * t),
    255,
  ];
};

const trayIdle: Shader = (x, y) => (mark(x, y, 0.82) ? BLACK : null);

// A filled dot: the one shape nobody misreads as anything but recording.
const trayRecording: Shader = (x, y) => {
  const dx = x - 0.5;
  const dy = y - 0.5;
  return dx * dx + dy * dy <= 0.3 * 0.3 ? BLACK : null;
};

// An ellipsis: working on it.
const trayProcessing: Shader = (x, y) => {
  for (const cx of [0.19, 0.5, 0.81]) {
    const dx = x - cx;
    const dy = y - 0.5;
    if (dx * dx + dy * dy <= 0.125 * 0.125) return BLACK;
  }
  return null;
};

const write = (rel: string, png: Buffer) => {
  fs.writeFileSync(path.join(root, rel), png);
  console.log(`wrote ${rel}`);
};

write("src-tauri/icons/app-icon.png", renderPng(1024, appIcon));
write("src-tauri/resources/tray_idle.png", renderPng(64, trayIdle));
write("src-tauri/resources/tray_recording.png", renderPng(64, trayRecording));
write("src-tauri/resources/tray_processing.png", renderPng(64, trayProcessing));
