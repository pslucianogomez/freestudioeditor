import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { convertFileSrc } from "@tauri-apps/api/core";
import { api, type Quality } from "../lib/api";
import type { Settings } from "../lib/settings";
import type { JobsApi } from "../lib/jobs";
import { VIDEO_EXT, baseName, formatDuration, formatTime, parseTime } from "../lib/format";
import { useIncoming, type Incoming } from "../lib/incoming";

interface EditClip {
  key: string;
  path: string;
  duration: number;
  start: number;
  end: number;
}

/** Formatos que el reproductor integrado puede mostrar (el resto igual se edita, sin vista previa). */
const PLAYABLE = ["mp4", "m4v", "webm", "mov"];

function TimeInput({ value, onChange }: { value: number; onChange: (v: number) => void }) {
  const [text, setText] = useState(formatTime(value));
  useEffect(() => setText(formatTime(value)), [value]);
  return (
    <input
      className="time"
      value={text}
      onChange={(e) => setText(e.target.value)}
      onBlur={() => {
        const t = parseTime(text);
        if (t != null) onChange(t);
        else setText(formatTime(value));
      }}
    />
  );
}

export function VideoEditorPage({
  settings,
  jobs,
  incoming,
}: {
  settings: Settings;
  jobs: JobsApi;
  incoming: Incoming | null;
}) {
  const [clips, setClips] = useState<EditClip[]>([]);
  const [sel, setSel] = useState<string | null>(null);
  const [height, setHeight] = useState(0);
  const [quality, setQuality] = useState<Quality>("medium");
  const [error, setError] = useState("");
  const video = useRef<HTMLVideoElement>(null);

  const selected = clips.find((c) => c.key === sel) ?? null;
  const playable = selected ? PLAYABLE.includes(baseName(selected.path).split(".").pop()!.toLowerCase()) : false;

  const addFiles = async (paths: string[]) => {
    setError("");
    for (const path of paths) {
      try {
        const info = await api.mediaInfo(path);
        if (!info.hasVideo) {
          setError(`${baseName(path)} no tiene video.`);
          continue;
        }
        const d = info.duration ?? 0;
        const clip = { key: crypto.randomUUID(), path, duration: d, start: 0, end: d };
        setClips((c) => [...c, clip]);
        setSel((s) => s ?? clip.key);
      } catch (e) {
        setError(String(e));
      }
    }
  };
  useIncoming(incoming, addFiles);

  const pick = async () => {
    const r = await open({ multiple: true, filters: [{ name: "Video", extensions: VIDEO_EXT }] });
    if (r) addFiles(Array.isArray(r) ? r : [r]);
  };

  const update = (key: string, p: Partial<EditClip>) =>
    setClips((cs) =>
      cs.map((c) => {
        if (c.key !== key) return c;
        const n = { ...c, ...p };
        n.start = Math.max(0, Math.min(n.start, n.duration));
        n.end = Math.max(n.start, Math.min(n.end, n.duration));
        return n;
      }),
    );

  const move = (key: string, dir: -1 | 1) =>
    setClips((cs) => {
      const i = cs.findIndex((c) => c.key === key);
      const j = i + dir;
      if (j < 0 || j >= cs.length) return cs;
      const copy = [...cs];
      [copy[i], copy[j]] = [copy[j], copy[i]];
      return copy;
    });

  const duplicate = (c: EditClip) => {
    const n = { ...c, key: crypto.randomUUID() };
    setClips((cs) => {
      const i = cs.findIndex((x) => x.key === c.key);
      return [...cs.slice(0, i + 1), n, ...cs.slice(i + 1)];
    });
    setSel(n.key);
  };

  const remove = (key: string) => {
    setClips((cs) => cs.filter((c) => c.key !== key));
    if (sel === key) setSel(null);
  };

  /** Divide el tramo en el punto actual del reproductor: sirve para sacar una parte del medio. */
  const split = (c: EditClip) => {
    const t = video.current?.currentTime ?? 0;
    if (t <= c.start + 0.1 || t >= c.end - 0.1) {
      setError("Mové el reproductor a un punto dentro del tramo para dividirlo.");
      return;
    }
    const second = { ...c, key: crypto.randomUUID(), start: t };
    setClips((cs) => {
      const i = cs.findIndex((x) => x.key === c.key);
      const first = { ...c, end: t };
      return [...cs.slice(0, i), first, second, ...cs.slice(i + 1)];
    });
    setError("");
  };

  // Al elegir un tramo, el reproductor salta a su inicio.
  useEffect(() => {
    if (selected && video.current) video.current.currentTime = selected.start;
  }, [sel]); // eslint-disable-line react-hooks/exhaustive-deps

  const total = clips.reduce((acc, c) => acc + (c.end - c.start), 0);

  const exportVideo = async () => {
    const id = crypto.randomUUID();
    await jobs.run({ id, type: "edit", title: `Edición de ${clips.length} tramo(s) → MP4` }, () =>
      api.joinClips({
        id,
        clips: clips.map((c) => ({ path: c.path, start: c.start, end: c.end })),
        outDir: settings.outDir,
        height: height || undefined,
        quality,
      }),
    );
  };

  return (
    <div className="panel">
      <h1>Editor de video</h1>
      <p className="hint">
        Agregá uno o varios videos, recortá cada tramo y ordenalos. Para sacar una parte del medio: dividí el tramo en
        dos y ajustá los bordes. Al exportar se unen en un solo MP4.
      </p>

      <div className="editor">
        <div className="player">
          {selected ? (
            playable ? (
              <video ref={video} src={convertFileSrc(selected.path)} controls />
            ) : (
              <div className="no-preview">Sin vista previa para este formato. Igual podés editarlo.</div>
            )
          ) : (
            <button className="dropzone" onClick={pick}>
              Elegí o arrastrá videos…
            </button>
          )}
          {selected && (
            <div className="row">
              <button onClick={() => update(selected.key, { start: video.current?.currentTime ?? 0 })} disabled={!playable}>
                ⟦ Marcar inicio
              </button>
              <button onClick={() => update(selected.key, { end: video.current?.currentTime ?? selected.end })} disabled={!playable}>
                Marcar fin ⟧
              </button>
              <button onClick={() => split(selected)} disabled={!playable}>
                ✂ Dividir aquí
              </button>
              <button
                onClick={() => {
                  if (video.current) {
                    video.current.currentTime = selected.start;
                    video.current.play();
                  }
                }}
                disabled={!playable}
              >
                ▶ Ver tramo
              </button>
            </div>
          )}
        </div>

        <div className="clips">
          <div className="clips-head">
            <strong>Tramos</strong>
            <button className="link" onClick={pick}>
              + Agregar videos
            </button>
          </div>
          {clips.length === 0 && <p className="hint">Todavía no agregaste videos.</p>}
          <ol>
            {clips.map((c, i) => (
              <li key={c.key} className={c.key === sel ? "clip on" : "clip"} onClick={() => setSel(c.key)}>
                <div className="clip-name" title={c.path}>
                  {i + 1}. {baseName(c.path)}
                </div>
                <div className="clip-range">
                  <TimeInput value={c.start} onChange={(v) => update(c.key, { start: v })} />
                  →
                  <TimeInput value={c.end} onChange={(v) => update(c.key, { end: v })} />
                  <span className="hint">({formatDuration(c.end - c.start)})</span>
                </div>
                <div className="clip-actions">
                  <button className="link" onClick={() => move(c.key, -1)} title="Subir">
                    ↑
                  </button>
                  <button className="link" onClick={() => move(c.key, 1)} title="Bajar">
                    ↓
                  </button>
                  <button className="link" onClick={() => duplicate(c)}>
                    Duplicar
                  </button>
                  <button className="link danger" onClick={() => remove(c.key)}>
                    Quitar
                  </button>
                </div>
              </li>
            ))}
          </ol>
        </div>
      </div>

      {error && <p className="error-text">{error}</p>}

      <div className="row">
        <label>
          Resolución
          <select value={height} onChange={(e) => setHeight(Number(e.target.value))}>
            <option value={0}>La del primer tramo</option>
            {[2160, 1080, 720, 480].map((h) => (
              <option key={h} value={h}>
                {h}p
              </option>
            ))}
          </select>
        </label>
        <label>
          Calidad
          <select value={quality} onChange={(e) => setQuality(e.target.value as Quality)}>
            <option value="high">Alta</option>
            <option value="medium">Media</option>
            <option value="low">Baja</option>
          </select>
        </label>
      </div>
      <div className="row end">
        <span className="hint">
          Duración final: {formatDuration(total) || "0:00"} · Guardar en: {settings.outDir || "…"}
        </span>
        <button className="primary" disabled={clips.length === 0 || !settings.outDir} onClick={exportVideo}>
          Exportar MP4
        </button>
      </div>
    </div>
  );
}
