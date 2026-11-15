// Baja yt-dlp y FFmpeg a src-tauri/binaries con el nombre que espera Tauri
// (<nombre>-<target-triple>.exe). Uso: npm run fetch-binaries
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const out = join(root, "src-tauri", "binaries");
mkdirSync(out, { recursive: true });

if (process.platform !== "win32") {
  console.error("Por ahora el script solo arma binarios para Windows x64.");
  process.exit(1);
}
const triple = "x86_64-pc-windows-msvc";

const SOURCES = {
  ytdlp: "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe",
  // Build GPL de BtbN (incluye x264/x265, por eso el proyecto es GPL-3.0).
  ffmpeg: "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip",
};

async function fetchTo(url, dest) {
  console.log(`↓ ${url}`);
  const res = await fetch(url, { redirect: "follow" });
  if (!res.ok) throw new Error(`${res.status} ${res.statusText} en ${url}`);
  writeFileSync(dest, Buffer.from(await res.arrayBuffer()));
  const sha = createHash("sha256").update(readFileSync(dest)).digest("hex");
  console.log(`  sha256 ${sha}`);
}

const force = process.argv.includes("--force");
const ytdlp = join(out, `yt-dlp-${triple}.exe`);
const ffmpeg = join(out, `ffmpeg-${triple}.exe`);

if (force || !existsSync(ytdlp)) await fetchTo(SOURCES.ytdlp, ytdlp);
else console.log("yt-dlp ya está (usá --force para rebajar)");

if (force || !existsSync(ffmpeg)) {
  const tmp = mkdtempSync(join(tmpdir(), "ffmpeg-"));
  const zip = join(tmp, "ffmpeg.zip");
  await fetchTo(SOURCES.ffmpeg, zip);
  // tar de Windows 10+ (bsdtar) descomprime .zip; se usa el del sistema y no el de Git, que no entiende .zip ni rutas "C:".
  const tar = join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");
  execFileSync(tar, ["-xf", zip, "-C", tmp], { stdio: "inherit" });
  // copiar (no mover): en GitHub la carpeta temporal y el proyecto están en discos distintos.
  copyFileSync(join(tmp, "ffmpeg-master-latest-win64-gpl", "bin", "ffmpeg.exe"), ffmpeg);
  rmSync(tmp, { recursive: true, force: true });
} else console.log("ffmpeg ya está (usá --force para rebajar)");

console.log("Listo:", out);
