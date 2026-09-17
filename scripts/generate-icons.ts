// Generates the placeholder app icon and the menu bar (tray) template icons.
// Run: bun scripts/generate-icons.ts && bun tauri icon src-tauri/icons/app-icon.png
// The artwork is intentionally simple; it will be replaced in the redesign.
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

const BAR_HEIGHTS = [0.3, 0.6, 0.9, 0.6, 0.3];

function bars(x: number, y: number, scale: number, cy = 0.5): boolean {
  const width = 0.09 * scale;
  const gap = 0.07 * scale;
  const total = BAR_HEIGHTS.length * width + (BAR_HEIGHTS.length - 1) * gap;
  const start = 0.5 - total / 2 + width / 2;
  return BAR_HEIGHTS.some((h, i) =>
    roundedRect(x, y, start + i * (width + gap), cy, width / 2, (h * scale) / 2, width / 2),
  );
}

const BLACK: Rgba = [0, 0, 0, 255];

const appIcon: Shader = (x, y) => {
  if (!roundedRect(x, y, 0.5, 0.5, 0.41, 0.41, 0.09)) return null;
  if (bars(x, y, 0.62)) return [255, 255, 255, 255];
  const t = y;
  return [Math.round(40 + 30 * t), Math.round(44 + 20 * t), Math.round(52 + 40 * t), 255];
};

const trayIdle: Shader = (x, y) => (bars(x, y, 0.95) ? BLACK : null);

const trayRecording: Shader = (x, y) => {
  const dx = x - 0.5;
  const dy = y - 0.5;
  const d = Math.sqrt(dx * dx + dy * dy);
  return d <= 0.22 || (d >= 0.36 && d <= 0.45) ? BLACK : null;
};

const trayProcessing: Shader = (x, y) => {
  for (const cx of [0.2, 0.5, 0.8]) {
    const dx = x - cx;
    const dy = y - 0.5;
    if (dx * dx + dy * dy <= 0.1 * 0.1) return BLACK;
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
