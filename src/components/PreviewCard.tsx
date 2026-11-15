import type { Probe } from "../lib/api";
import { formatDuration } from "../lib/format";

export type PreviewState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "ok"; data: Probe }
  | { status: "error"; error: string };

export function PreviewCard({ state }: { state: PreviewState }) {
  if (state.status === "idle") return null;
  if (state.status === "loading") {
    return <div className="preview muted">Buscando información del enlace…</div>;
  }
  if (state.status === "error") {
    return <div className="preview error-box">No se pudo leer el enlace: {state.error}</div>;
  }
  const p = state.data;
  return (
    <div className="preview">
      {p.thumbnail ? <img src={p.thumbnail} alt="" /> : <div className="thumb-empty" />}
      <div className="preview-info">
        <div className="preview-title">{p.title}</div>
        <div className="hint">
          {[p.uploader, p.kind === "playlist" ? `Playlist · ${p.count ?? "?"} videos` : formatDuration(p.duration)]
            .filter(Boolean)
            .join(" · ")}
        </div>
        {p.kind === "playlist" && p.entries.length > 0 && (
          <ol className="entries">
            {p.entries.map((t, i) => (
              <li key={i}>{t}</li>
            ))}
            {(p.count ?? 0) > p.entries.length && <li className="more">y {(p.count ?? 0) - p.entries.length} más…</li>}
          </ol>
        )}
      </div>
    </div>
  );
}
