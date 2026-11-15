/** 125 -> "2:05", 3725 -> "1:02:05" */
export function formatDuration(s?: number): string {
  if (s == null || !isFinite(s)) return "";
  const t = Math.round(s);
  const h = Math.floor(t / 3600);
  const m = Math.floor((t % 3600) / 60);
  const sec = String(t % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${sec}` : `${m}:${sec}`;
}

/** 83.25 -> "1:23.2" (con décimas, para editores) */
export function formatTime(s: number): string {
  if (!isFinite(s)) return "0:00.0";
  const m = Math.floor(s / 60);
  const rest = (s - m * 60).toFixed(1).padStart(4, "0");
  return `${m}:${rest}`;
}

/** "1:23.5" | "83.5" | "0:01:23" -> segundos, o null si no se entiende */
export function parseTime(t: string): number | null {
  const parts = t.trim().split(":").map(Number);
  if (parts.length === 0 || parts.length > 3 || parts.some((p) => !isFinite(p))) return null;
  return parts.reduce((acc, p) => acc * 60 + p, 0);
}

/** 72000000 -> "~69 MB" */
export function formatSize(bytes?: number): string {
  if (!bytes) return "";
  const mb = bytes / 1024 / 1024;
  return mb >= 1024 ? `~${(mb / 1024).toFixed(1)} GB` : `~${Math.max(1, Math.round(mb))} MB`;
}

/** Tamaño exacto de un archivo: 75776 -> "74 KB", 1258291 -> "1.2 MB" */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const kb = bytes / 1024;
  if (kb < 1024) return `${Math.round(kb)} KB`;
  const mb = kb / 1024;
  return mb < 1024 ? `${mb.toFixed(1)} MB` : `${(mb / 1024).toFixed(2)} GB`;
}

export const baseName = (p: string) => p.split(/[\\/]/).pop() ?? p;

export const extOf = (p: string) => (baseName(p).split(".").pop() ?? "").toLowerCase();

export const IMAGE_EXT = ["jpg", "jpeg", "png", "webp", "bmp", "tif", "tiff", "gif", "heic", "heif", "avif"];
export const AUDIO_EXT = ["mp3", "m4a", "aac", "wav", "flac", "ogg", "opus", "wma"];
export const VIDEO_EXT = ["mp4", "mkv", "webm", "avi", "mov", "wmv", "flv", "m4v", "mpg", "mpeg", "3gp", "ts", "vob"];
export const MEDIA_EXT = [...VIDEO_EXT, ...AUDIO_EXT];

/** Fecha para nombres de archivo: "2026-11-15 18-04-09" */
export function stamp(d = new Date()): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}-${p(d.getMinutes())}-${p(d.getSeconds())}`;
}
