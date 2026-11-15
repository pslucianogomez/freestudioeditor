import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, type Quality } from "../lib/api";
import type { Settings } from "../lib/settings";
import type { JobsApi } from "../lib/jobs";
import { MEDIA_EXT, baseName } from "../lib/format";
import { useIncoming, type Incoming } from "../lib/incoming";

const GROUPS: { name: string; presets: { id: string; label: string }[] }[] = [
  {
    name: "Video",
    presets: [
      { id: "mp4", label: "MP4 (H.264)" },
      { id: "webm", label: "WebM (VP9)" },
      { id: "mkv", label: "MKV" },
    ],
  },
  {
    name: "Audio",
    presets: [
      { id: "mp3", label: "MP3" },
      { id: "m4a", label: "M4A (AAC)" },
      { id: "ogg", label: "OGG" },
      { id: "opus", label: "OPUS" },
      { id: "wav", label: "WAV" },
      { id: "flac", label: "FLAC" },
    ],
  },
  {
    name: "Imagen",
    presets: [
      { id: "gif", label: "GIF animado" },
      { id: "jpg", label: "Fotogramas JPG" },
    ],
  },
];

/** Atajos: eligen formato, resolución y calidad de una vez. */
const DEVICES: { label: string; hint: string; preset: string; height: number; quality: Quality }[] = [
  { label: "WhatsApp / liviano", hint: "MP4 720p, archivo chico", preset: "mp4", height: 720, quality: "low" },
  { label: "Celular", hint: "MP4 1080p", preset: "mp4", height: 1080, quality: "medium" },
  { label: "TV / máxima calidad", hint: "MP4 resolución original", preset: "mp4", height: 0, quality: "high" },
  { label: "Web", hint: "WebM 1080p", preset: "webm", height: 1080, quality: "medium" },
];

const ROTATE = [
  { v: "", label: "Sin rotar" },
  { v: "cw", label: "90° derecha" },
  { v: "ccw", label: "90° izquierda" },
  { v: "180", label: "180°" },
  { v: "hflip", label: "Espejo horizontal" },
  { v: "vflip", label: "Espejo vertical" },
];

const HEIGHTS = [0, 2160, 1440, 1080, 720, 480, 360];
const LOSSY_AUDIO = ["mp3", "m4a", "ogg", "opus"];
const AUDIO_PRESETS = [...LOSSY_AUDIO, "wav", "flac"];
const VIDEO_PRESETS = ["mp4", "webm", "mkv"];

export function ConvertPage({
  settings,
  jobs,
  incoming,
}: {
  settings: Settings;
  jobs: JobsApi;
  incoming: Incoming | null;
}) {
  const [files, setFiles] = useState<string[]>([]);
  const [preset, setPreset] = useState("mp4");
  const [height, setHeight] = useState(0);
  const [quality, setQuality] = useState<Quality>("medium");
  const [bitrate, setBitrate] = useState(192);
  const [rotate, setRotate] = useState("");
  const [start, setStart] = useState("");
  const [end, setEnd] = useState("");
  const [frameEvery, setFrameEvery] = useState(1);
  const [gifWidth, setGifWidth] = useState(480);
  const [gifFps, setGifFps] = useState(12);

  const addFiles = (paths: string[]) => setFiles((f) => [...f, ...paths.filter((p) => !f.includes(p))]);
  useIncoming(incoming, addFiles);

  const pick = async () => {
    const sel = await open({ multiple: true, filters: [{ name: "Video y audio", extensions: MEDIA_EXT }] });
    if (sel) addFiles(Array.isArray(sel) ? sel : [sel]);
  };

  const applyDevice = (d: (typeof DEVICES)[number]) => {
    setPreset(d.preset);
    setHeight(d.height);
    setQuality(d.quality);
  };

  const run = async () => {
    for (const input of files) {
      const id = crypto.randomUUID();
      const inputSize = await api
        .fileInfo(input)
        .then((i) => i.size)
        .catch(() => undefined);
      await jobs.run({ id, type: "convert", title: `${baseName(input)} → ${preset.toUpperCase()}`, inputSize }, () =>
        api.convert({
          id,
          input,
          preset,
          outDir: settings.outDir,
          rotate: rotate || undefined,
          start: start || undefined,
          end: end || undefined,
          frameEvery: preset === "jpg" ? frameEvery : undefined,
          height: height || undefined,
          quality,
          audioBitrate: LOSSY_AUDIO.includes(preset) ? bitrate : undefined,
          gifWidth: preset === "gif" ? gifWidth : undefined,
          gifFps: preset === "gif" ? gifFps : undefined,
        }),
      );
    }
    // La lista queda cargada para poder probar otras opciones.
  };

  const isAudio = AUDIO_PRESETS.includes(preset);
  const isVideo = VIDEO_PRESETS.includes(preset);

  return (
    <div className="panel">
      <h1>Convertir</h1>
      <button className="dropzone" onClick={pick}>
        {files.length === 0 ? "Elegí o arrastrá archivos de video o audio…" : "Agregar más archivos…"}
      </button>
      {files.length > 0 && (
        <button className="link" style={{ alignSelf: "flex-end" }} onClick={() => setFiles([])}>
          Vaciar lista
        </button>
      )}
      {files.length > 0 && (
        <ul className="file-list">
          {files.map((f) => (
            <li key={f}>
              <span title={f}>{baseName(f)}</span>
              <button className="link" onClick={() => setFiles((l) => l.filter((x) => x !== f))}>
                Quitar
              </button>
            </li>
          ))}
        </ul>
      )}

      <fieldset className="presets">
        <legend>Atajos</legend>
        {DEVICES.map((d) => (
          <button
            key={d.label}
            className={preset === d.preset && height === d.height && quality === d.quality ? "chip on" : "chip"}
            onClick={() => applyDevice(d)}
            title={d.hint}
          >
            {d.label}
          </button>
        ))}
      </fieldset>

      {GROUPS.map((g) => (
        <fieldset key={g.name} className="presets">
          <legend>{g.name}</legend>
          {g.presets.map((p) => (
            <button key={p.id} className={preset === p.id ? "chip on" : "chip"} onClick={() => setPreset(p.id)}>
              {p.label}
            </button>
          ))}
        </fieldset>
      ))}

      <div className="row">
        {(isVideo || preset === "jpg") && (
          <label>
            Resolución
            <select value={height} onChange={(e) => setHeight(Number(e.target.value))}>
              {HEIGHTS.map((h) => (
                <option key={h} value={h}>
                  {h === 0 ? "Original" : `${h}p`}
                </option>
              ))}
            </select>
          </label>
        )}
        {isVideo && (
          <label>
            Calidad
            <select value={quality} onChange={(e) => setQuality(e.target.value as Quality)}>
              <option value="high">Alta (archivo grande)</option>
              <option value="medium">Media</option>
              <option value="low">Baja (archivo chico)</option>
            </select>
          </label>
        )}
        {LOSSY_AUDIO.includes(preset) && (
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
        {preset === "gif" && (
          <>
            <label>
              Ancho
              <select value={gifWidth} onChange={(e) => setGifWidth(Number(e.target.value))}>
                {[320, 480, 640, 800].map((w) => (
                  <option key={w} value={w}>
                    {w}px
                  </option>
                ))}
              </select>
            </label>
            <label>
              Cuadros/seg
              <select value={gifFps} onChange={(e) => setGifFps(Number(e.target.value))}>
                {[8, 10, 12, 15, 20, 24].map((f) => (
                  <option key={f}>{f}</option>
                ))}
              </select>
            </label>
          </>
        )}
        {preset === "jpg" && (
          <label>
            Un fotograma cada
            <input
              type="number"
              min={0.1}
              step={0.5}
              value={frameEvery}
              onChange={(e) => setFrameEvery(Number(e.target.value))}
              style={{ width: 70 }}
            />
            s
          </label>
        )}
      </div>

      <div className="row">
        {!isAudio && (
          <label>
            Rotar
            <select value={rotate} onChange={(e) => setRotate(e.target.value)}>
              {ROTATE.map((r) => (
                <option key={r.v} value={r.v}>
                  {r.label}
                </option>
              ))}
            </select>
          </label>
        )}
        <label>
          Desde
          <input value={start} onChange={(e) => setStart(e.target.value)} placeholder="00:00:00" size={9} />
        </label>
        <label>
          Hasta
          <input value={end} onChange={(e) => setEnd(e.target.value)} placeholder="fin" size={9} />
        </label>
      </div>

      <div className="row end">
        <span className="hint">Guardar en: {settings.outDir || "…"}</span>
        <button className="primary" disabled={files.length === 0 || !settings.outDir} onClick={run}>
          Convertir {files.length > 1 ? `(${files.length})` : ""}
        </button>
      </div>
    </div>
  );
}
