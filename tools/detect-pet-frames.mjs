import { spawn } from "node:child_process";
import { readdir, writeFile } from "node:fs/promises";
import { join } from "node:path";

const PETS_DIR = "static/pets";
const SHEET_W = 1536;
const SHEET_H = 1872;
const COLS = 8;
const ROWS = 9;
const FRAME_W = SHEET_W / COLS;
const FRAME_H = SHEET_H / ROWS;
const ALPHA_THRESHOLD = 8;
const FILL_THRESHOLD = 0.04;

function decodeWebp(path) {
  return new Promise((resolve, reject) => {
    const proc = spawn("ffmpeg", [
      "-loglevel",
      "error",
      "-i",
      path,
      "-f",
      "rawvideo",
      "-pix_fmt",
      "rgba",
      "-",
    ]);
    const chunks = [];
    proc.stdout.on("data", (c) => chunks.push(c));
    proc.stderr.on("data", () => {});
    proc.on("error", reject);
    proc.on("close", (code) => {
      if (code !== 0) {
        reject(new Error(`ffmpeg exited ${code}`));
        return;
      }
      resolve(Buffer.concat(chunks));
    });
  });
}

function frameHasContent(rgba, frameX, frameY) {
  let nonEmpty = 0;
  const total = FRAME_W * FRAME_H;
  for (let dy = 0; dy < FRAME_H; dy++) {
    const y = frameY + dy;
    const rowOffset = y * SHEET_W * 4;
    for (let dx = 0; dx < FRAME_W; dx++) {
      const x = frameX + dx;
      const alphaIdx = rowOffset + x * 4 + 3;
      if (rgba[alphaIdx] > ALPHA_THRESHOLD) nonEmpty++;
    }
  }
  return nonEmpty / total > FILL_THRESHOLD;
}

function defaultName(row) {
  const names = ["idle", "walk_right", "walk_left", "wave", "tiny", "sad", "sleep", "happy", "extra"];
  return names[row] ?? `row_${row}`;
}

function defaultFps(row) {
  if (row === 0) return 6;
  return 8;
}

async function analyzePet(slug, sheetPath) {
  const rgba = await decodeWebp(sheetPath);
  const expected = SHEET_W * SHEET_H * 4;
  if (rgba.length !== expected) {
    throw new Error(`size mismatch for ${slug}: got ${rgba.length}, expected ${expected}`);
  }
  const animations = [];
  for (let row = 0; row < ROWS; row++) {
    let count = 0;
    for (let col = 0; col < COLS; col++) {
      const has = frameHasContent(rgba, col * FRAME_W, row * FRAME_H);
      if (has) count = col + 1;
    }
    if (count === 0) continue;
    animations.push({
      name: defaultName(row),
      row,
      frame_count: count,
      fps: defaultFps(row),
    });
  }
  return animations;
}

async function main() {
  const entries = await readdir(PETS_DIR, { withFileTypes: true });
  const result = {};
  for (const entry of entries) {
    if (!entry.isDirectory()) continue;
    const slug = entry.name;
    const sheet = join(PETS_DIR, slug, "spritesheet.webp");
    try {
      const anims = await analyzePet(slug, sheet);
      result[slug] = anims;
      console.log(`${slug}: ${anims.map((a) => `${a.name}=${a.frame_count}`).join(", ")}`);
    } catch (err) {
      console.error(`${slug}: ${err.message}`);
    }
  }
  const out = join(PETS_DIR, "animations.json");
  await writeFile(out, JSON.stringify(result, null, 2));
  console.log(`wrote ${out}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
