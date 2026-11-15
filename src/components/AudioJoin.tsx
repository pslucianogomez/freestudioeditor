import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "../lib/api";
import type { Settings } from "../lib/settings";
import type { JobsApi } from "../lib/jobs";
import { MEDIA_EXT, baseName, formatDuration } from "../lib/format";
import { useIncoming, type Incoming } from "../lib/incoming";
import { Slider } from "./Slider";

interface Track {
  key: string;
  path: string;
  duration?: number;
}

const FORMATS = ["mp3", "m4a", "ogg", "opus", "wav", "flac"];
const LOSSY = ["mp3", "m4a", "ogg", "opus"];

/** Une varios audios (o el audio de varios videos) en uno, en orden, con fundido cruzado opcional. */
export function AudioJoin({ settings, jobs, incoming }: { settings: Settings; jobs: JobsApi; incoming: Incoming | null }) {
  const [tracks, setTracks] = useState<Track[]>([]);
  const [crossfade, setCrossfade] = useState(0);
  const [format, setFormat] = useState("mp3");
  const [bitrate, setBitrate] = useState(192);
  const [error, setError] = useState("");

  const add = async (paths: string[]) => {
    setError("");
    for (const path of paths) {
      try {
        const i = await api.mediaInfo(path);
        if (!i.hasAudio) {
          setError(`${baseName(path)} no tiene audio.`);
          continue;
        }
        setTracks((t) => [...t, { key: crypto.randomUUID(), path, duration: i.duration }]);
      } catch (e) {
        setError(String(e));
      }
    }
  };
  useIncoming(incoming, add);

  const pick = async () => {
    const r = await open({ multiple: true, filters: [{ name: "Audio o video", extensions: MEDIA_EXT }] });
    if (r) add(Array.isArray(r) ? r : [r]);
  };

  const move = (i: number, dir: -1 | 1) =>
    setTracks((t) => {
      const j = i + dir;
      if (j < 0 || j >= t.length) return t;
      const c = [...t];
      [c[i], c[j]] = [c[j], c[i]];
      return c;
    });

  const total = tracks.reduce((a, t) => a + (t.duration ?? 0), 0) - crossfade * Math.max(0, tracks.length - 1);
  const shortest = Math.min(...tracks.map((t) => t.duration ?? Infinity));
  const tooShort = crossfade > 0 && shortest <= crossfade * 1.05;

  const run = async () => {
    const id = crypto.randomUUID();
    await jobs.run({ id, type: "audio", title: `Unir ${tracks.length} audios → ${format.toUpperCase()}` }, () =>
      api.joinAudio({
        id,
        inputs: tracks.map((t) => t.path),
        outDir: settings.outDir,
        format,
        bitrate: LOSSY.includes(format) ? bitrate : undefined,
        crossfade,
      }),
    );
  };

  return (
    <>
      <p className="hint">Agregá audios (o videos, se usa su audio), ordenalos y se unen en un solo archivo.</p>
      <button className="dropzone" onClick={pick}>
        {tracks.length === 0 ? "Elegí o arrastrá varios audios…" : "Agregar más…"}
      </button>
      {error && <p className="error-text">{error}</p>}
      {tracks.length > 0 && (
        <ol className="track-list">
          {tracks.map((t, i) => (
            <li key={t.key}>
              <span className="track-name" title={t.path}>
                {i + 1}. {baseName(t.path)}
              </span>
              <span className="hint">{formatDuration(t.duration)}</span>
              <button className="link" onClick={() => move(i, -1)} title="Subir">
                ↑
              </button>
              <button className="link" onClick={() => move(i, 1)} title="Bajar">
                ↓
              </button>
              <button className="link danger" onClick={() => setTracks((l) => l.filter((x) => x.key !== t.key))}>
                Quitar
              </button>
            </li>
          ))}
        </ol>
      )}
      <div className="row">
        <Slider
          label="Fundido entre pistas"
          min={0}
          max={10}
          step={0.5}
          value={crossfade}
          onChange={setCrossfade}
          defaultValue={0}
          format={(v) => (v === 0 ? "sin fundido" : `${v} s`)}
        />
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
      {tooShort && <p className="error-text">Alguna pista dura menos que el fundido: bajá los segundos.</p>}
      <div className="row end">
        <span className="hint">
          {tracks.length > 1 ? `Duración final: ${formatDuration(total)} · ` : ""}Guardar en: {settings.outDir || "…"}
        </span>
        <button className="primary" onClick={run} disabled={tracks.length < 2 || !settings.outDir || tooShort}>
          Unir {tracks.length > 1 ? `(${tracks.length})` : ""}
        </button>
      </div>
    </>
  );
}
