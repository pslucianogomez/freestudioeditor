import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { convertFileSrc } from "@tauri-apps/api/core";
import { api } from "../lib/api";
import type { Settings } from "../lib/settings";
import type { JobsApi } from "../lib/jobs";
import { MEDIA_EXT, baseName, extOf, formatTime } from "../lib/format";
import { useIncoming, type Incoming } from "../lib/incoming";

const POINTS = 1600;
const FORMATS = ["mp3", "m4a", "ogg", "opus", "wav", "flac"];
const LOSSY = ["mp3", "m4a", "ogg", "opus"];

type Mode = "all" | "keep" | "delete";

function cssVar(name: string) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

export function AudioEditorPage({
  settings,
  jobs,
  incoming,
}: {
  settings: Settings;
  jobs: JobsApi;
  incoming: Incoming | null;
}) {
  const [file, setFile] = useState<string | null>(null);
  const [peaks, setPeaks] = useState<number[]>([]);
  const [duration, setDuration] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [sel, setSel] = useState<[number, number] | null>(null);
  const [playhead, setPlayhead] = useState(0);
  const [playing, setPlaying] = useState(false);

  const [mode, setMode] = useState<Mode>("all");
  const [fadeIn, setFadeIn] = useState(0);
  const [fadeOut, setFadeOut] = useState(0);
  const [volume, setVolume] = useState(0);
  const [normalize, setNormalize] = useState(false);
  const [format, setFormat] = useState("mp3");
  const [bitrate, setBitrate] = useState(192);

  const canvas = useRef<HTMLCanvasElement>(null);
  const audio = useRef<HTMLAudioElement>(null);
  const drag = useRef<number | null>(null);
  const stopAt = useRef<number | null>(null);

  const load = async (path: string) => {
    setFile(path);
    setPeaks([]);
    setSel(null);
    setError("");
    setLoading(true);
    setPlayhead(0);
    const ext = extOf(path);
    if (FORMATS.includes(ext)) setFormat(ext);
    try {
      const w = await api.waveform(path, POINTS);
      if (w.peaks.length === 0) throw new Error("El archivo no tiene audio.");
      setPeaks(w.peaks);
      setDuration(w.duration);
    } catch (e) {
      setError(String(e));
      setFile(null);
    } finally {
      setLoading(false);
    }
  };
  useIncoming(incoming, (paths) => paths[0] && load(paths[0]));

  const pick = async () => {
    const r = await open({ multiple: false, filters: [{ name: "Audio o video", extensions: MEDIA_EXT }] });
    if (typeof r === "string") load(r);
  };

  // Dibuja la forma de onda, la selección y el cabezal.
  useEffect(() => {
    const c = canvas.current;
    if (!c || peaks.length === 0) return;
    const dpr = window.devicePixelRatio || 1;
    const W = c.clientWidth;
    const H = c.clientHeight;
    c.width = W * dpr;
    c.height = H * dpr;
    const g = c.getContext("2d")!;
    g.scale(dpr, dpr);
    g.clearRect(0, 0, W, H);
    const x = (t: number) => (t / duration) * W;
    if (sel) {
      g.fillStyle = mode === "delete" ? "rgba(220,60,50,0.18)" : "rgba(47,111,222,0.18)";
      g.fillRect(x(sel[0]), 0, x(sel[1]) - x(sel[0]), H);
    }
    g.fillStyle = cssVar("--accent") || "#2f6fde";
    const bw = W / peaks.length;
    for (let i = 0; i < peaks.length; i++) {
      const h = Math.max(1, peaks[i] * (H - 8));
      g.fillRect(i * bw, (H - h) / 2, Math.max(1, bw - 0.5), h);
    }
    g.fillStyle = cssVar("--text") || "#000";
    g.fillRect(x(playhead) - 0.5, 0, 1.5, H);
  }, [peaks, duration, sel, playhead, mode]);

  // Cabezal sigue a la reproducción; corta al final de la selección si se pidió.
  useEffect(() => {
    if (!playing) return;
    let raf = 0;
    const tick = () => {
      const a = audio.current;
      if (a) {
        setPlayhead(a.currentTime);
        if (stopAt.current != null && a.currentTime >= stopAt.current) {
          a.pause();
          stopAt.current = null;
        }
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [playing]);

  const timeAt = (clientX: number) => {
    const r = canvas.current!.getBoundingClientRect();
    return Math.max(0, Math.min(duration, ((clientX - r.left) / r.width) * duration));
  };

  const onDown = (e: React.MouseEvent) => {
    drag.current = timeAt(e.clientX);
    setSel(null);
  };
  const onMove = (e: React.MouseEvent) => {
    if (drag.current == null) return;
    const t = timeAt(e.clientX);
    setSel([Math.min(drag.current, t), Math.max(drag.current, t)]);
  };
  const onUp = (e: React.MouseEvent) => {
    if (drag.current == null) return;
    const t = timeAt(e.clientX);
    if (Math.abs(t - drag.current) < duration / 400) {
      // Fue un clic: mueve el cabezal.
      setSel(null);
      setPlayhead(t);
      if (audio.current) audio.current.currentTime = t;
    } else if (mode === "all") {
      setMode("keep");
    }
    drag.current = null;
  };

  const playPause = () => {
    const a = audio.current;
    if (!a) return;
    stopAt.current = null;
    if (a.paused) {
      a.currentTime = playhead;
      a.play();
    } else a.pause();
  };

  const playSelection = () => {
    const a = audio.current;
    if (!a || !sel) return;
    a.currentTime = sel[0];
    stopAt.current = sel[1];
    a.play();
  };

  const resultDuration =
    mode === "keep" && sel ? sel[1] - sel[0] : mode === "delete" && sel ? duration - (sel[1] - sel[0]) : duration;

  const exportAudio = async () => {
    if (!file) return;
    const id = crypto.randomUUID();
    await jobs.run({ id, type: "audio", title: `${baseName(file)} → ${format.toUpperCase()} (editado)` }, () =>
      api.audioEdit({
        id,
        input: file,
        outDir: settings.outDir,
        format,
        bitrate: LOSSY.includes(format) ? bitrate : undefined,
        duration,
        mode,
        selStart: sel?.[0] ?? 0,
        selEnd: sel?.[1] ?? 0,
        fadeIn,
        fadeOut,
        volumeDb: volume,
        normalize,
      }),
    );
  };

  return (
    <div className="panel">
      <h1>Editor de audio</h1>
      {!file ? (
        <>
          <button className="dropzone" onClick={pick} disabled={loading}>
            {loading ? "Leyendo el archivo…" : "Elegí o arrastrá un archivo de audio (o un video para editar su audio)…"}
          </button>
          {error && <p className="error-text">{error}</p>}
        </>
      ) : (
        <>
          <div className="row end">
            <strong title={file}>{baseName(file)}</strong>
            <button className="link" onClick={pick}>
              Abrir otro…
            </button>
          </div>
          {loading ? (
            <div className="wave-loading">Leyendo la forma de onda…</div>
          ) : (
            <canvas
              ref={canvas}
              className="wave"
              onMouseDown={onDown}
              onMouseMove={onMove}
              onMouseUp={onUp}
              onMouseLeave={(e) => drag.current != null && onUp(e)}
            />
          )}
          <audio
            ref={audio}
            src={convertFileSrc(file)}
            onPlay={() => setPlaying(true)}
            onPause={() => setPlaying(false)}
            onEnded={() => setPlaying(false)}
          />
          <div className="row">
            <button onClick={playPause}>{playing ? "⏸ Pausa" : "▶ Reproducir"}</button>
            <button onClick={playSelection} disabled={!sel}>
              ▶ Escuchar selección
            </button>
            <span className="hint">
              {formatTime(playhead)} / {formatTime(duration)}
              {sel ? ` · selección ${formatTime(sel[0])} → ${formatTime(sel[1])}` : " · arrastrá sobre la onda para seleccionar"}
            </span>
          </div>

          <div className="segmented" role="radiogroup" aria-label="Qué exportar">
            <button className={mode === "all" ? "on" : ""} onClick={() => setMode("all")}>
              Todo el archivo
            </button>
            <button className={mode === "keep" ? "on" : ""} onClick={() => setMode("keep")} disabled={!sel}>
              Quedarme con la selección
            </button>
            <button className={mode === "delete" ? "on" : ""} onClick={() => setMode("delete")} disabled={!sel}>
              Borrar la selección
            </button>
          </div>

          <div className="row">
            <label>
              Fundido de entrada
              <input type="number" min={0} step={0.5} value={fadeIn} onChange={(e) => setFadeIn(Number(e.target.value))} style={{ width: 64 }} />s
            </label>
            <label>
              Fundido de salida
              <input type="number" min={0} step={0.5} value={fadeOut} onChange={(e) => setFadeOut(Number(e.target.value))} style={{ width: 64 }} />s
            </label>
            <label>
              Volumen
              <input type="range" min={-20} max={20} step={1} value={volume} onChange={(e) => setVolume(Number(e.target.value))} />
              <span style={{ width: 52 }}>{volume > 0 ? "+" : ""}{volume} dB</span>
            </label>
            <label className="check">
              <input type="checkbox" checked={normalize} onChange={(e) => setNormalize(e.target.checked)} />
              Normalizar volumen
            </label>
          </div>

          <div className="row">
            <label>
              Formato
              <select value={format} onChange={(e) => setFormat(e.target.value)}>
                {FORMATS.map((f) => (
                  <option key={f}>{f}</option>
                ))}
              </select>
            </label>
            {LOSSY.includes(format) && (
              <label>
                Bitrate
                <select value={bitrate} onChange={(e) => setBitrate(Number(e.target.value))}>
                  {[320, 256, 192, 128, 96].map((b) => (
                    <option key={b} value={b}>
                      {b} kbps
                    </option>
                  ))}
                </select>
              </label>
            )}
          </div>

          <div className="row end">
            <span className="hint">
              Duración final: {formatTime(resultDuration)} · Guardar en: {settings.outDir || "…"}
            </span>
            <button className="primary" onClick={exportAudio} disabled={!settings.outDir || (mode !== "all" && !sel)}>
              Exportar
            </button>
          </div>
        </>
      )}
    </div>
  );
}
