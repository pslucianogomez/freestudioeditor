import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { convertFileSrc } from "@tauri-apps/api/core";
import { api, type MediaInfo, type Quality, type VideoFxRequest } from "../lib/api";
import type { Settings } from "../lib/settings";
import type { JobsApi } from "../lib/jobs";
import { AUDIO_EXT, VIDEO_EXT, baseName, extOf, formatDuration } from "../lib/format";
import { useIncoming, type Incoming } from "../lib/incoming";
import { Slider } from "../components/Slider";

const SPEEDS = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 2, 4];
const ASPECTS = [
  { v: "", label: "Original" },
  { v: "9:16", label: "Vertical 9:16", hint: "TikTok, Reels, Shorts" },
  { v: "1:1", label: "Cuadrado 1:1" },
  { v: "4:5", label: "Retrato 4:5", hint: "Instagram" },
  { v: "16:9", label: "Horizontal 16:9", hint: "YouTube" },
];
const ASPECT_MODES: { v: NonNullable<VideoFxRequest["aspectMode"]>; label: string; hint: string }[] = [
  { v: "blur", label: "Fondo desenfocado", hint: "El video entero, con el mismo video borroso de fondo" },
  { v: "pad", label: "Bandas negras", hint: "El video entero, con barras negras" },
  { v: "crop", label: "Recortar", hint: "Llena la pantalla recortando los costados" },
];
const PLAYABLE = ["mp4", "m4v", "webm", "mov"];

export function VideoEffectsPage({ settings, jobs, incoming }: { settings: Settings; jobs: JobsApi; incoming: Incoming | null }) {
  const [file, setFile] = useState<string | null>(null);
  const [info, setInfo] = useState<MediaInfo | null>(null);
  const [error, setError] = useState("");
  const [speed, setSpeed] = useState(1);
  const [audio, setAudio] = useState<VideoFxRequest["audio"]>("keep");
  const [music, setMusic] = useState("");
  const [musicVolume, setMusicVolume] = useState(0.3);
  const [aspect, setAspect] = useState("");
  const [aspectMode, setAspectMode] = useState<NonNullable<VideoFxRequest["aspectMode"]>>("blur");
  const [subtitles, setSubtitles] = useState("");
  const [stabilize, setStabilize] = useState(false);
  const [brightness, setBrightness] = useState(0);
  const [contrast, setContrast] = useState(1);
  const [saturation, setSaturation] = useState(1);
  const [quality, setQuality] = useState<Quality>("medium");

  const load = async (path: string) => {
    setError("");
    try {
      const i = await api.mediaInfo(path);
      if (!i.hasVideo) throw new Error(`${baseName(path)} no tiene video.`);
      setFile(path);
      setInfo(i);
    } catch (e) {
      setError(String(e));
    }
  };
  useIncoming(incoming, (paths) => paths[0] && load(paths[0]));

  const pickVideo = async () => {
    const r = await open({ multiple: false, filters: [{ name: "Video", extensions: VIDEO_EXT }] });
    if (typeof r === "string") load(r);
  };
  const pickMusic = async () => {
    const r = await open({ multiple: false, filters: [{ name: "Audio o video", extensions: [...AUDIO_EXT, ...VIDEO_EXT] }] });
    if (typeof r === "string") setMusic(r);
  };
  const pickSubs = async () => {
    const r = await open({ multiple: false, filters: [{ name: "Subtítulos", extensions: ["srt", "ass", "ssa", "vtt"] }] });
    if (typeof r === "string") setSubtitles(r);
  };

  const reset = () => {
    setSpeed(1);
    setAudio("keep");
    setMusic("");
    setAspect("");
    setSubtitles("");
    setStabilize(false);
    setBrightness(0);
    setContrast(1);
    setSaturation(1);
  };

  // Vista aproximada del color sobre el reproductor (el resultado final lo calcula FFmpeg).
  const cssFilter = `brightness(${1 + brightness}) contrast(${contrast}) saturate(${saturation})`;
  const needsMusic = audio === "replace" || audio === "mix";
  const changes = [
    speed !== 1 && `velocidad ×${speed}`,
    audio === "mute" && "sin audio",
    audio === "replace" && "audio reemplazado",
    audio === "mix" && "música mezclada",
    aspect && `formato ${aspect}`,
    subtitles && "subtítulos",
    stabilize && "estabilizado",
    (brightness !== 0 || contrast !== 1 || saturation !== 1) && "color",
  ].filter(Boolean) as string[];

  const exportVideo = async () => {
    if (!file) return;
    const id = crypto.randomUUID();
    await jobs.run({ id, type: "fx", title: `${baseName(file)} → ${changes.join(", ") || "copia"}` }, () =>
      api.videoFx({
        id,
        input: file,
        outDir: settings.outDir,
        speed,
        audio,
        audioFile: needsMusic ? music : undefined,
        musicVolume,
        aspect: aspect || undefined,
        aspectMode,
        subtitles: subtitles || undefined,
        stabilize,
        brightness,
        contrast,
        saturation,
        quality,
      }),
    );
  };

  return (
    <div className="panel">
      <h1>Efectos de video</h1>
      <p className="hint">Cambiá la velocidad, el audio, el formato (vertical, cuadrado), agregá subtítulos o corregí el color.</p>

      {!file ? (
        <>
          <button className="dropzone" onClick={pickVideo}>
            Elegí o arrastrá un video…
          </button>
          {error && <p className="error-text">{error}</p>}
        </>
      ) : (
        <>
          <div className="row end">
            <span>
              <strong title={file}>{baseName(file)}</strong>{" "}
              <span className="hint">
                {info?.width}×{info?.height} · {formatDuration(info?.duration)}
                {info && !info.hasAudio ? " · sin audio" : ""}
              </span>
            </span>
            <button className="link" onClick={pickVideo}>
              Abrir otro…
            </button>
          </div>
          {PLAYABLE.includes(extOf(file)) ? (
            <video className="fx-player" src={convertFileSrc(file)} controls style={{ filter: cssFilter }} />
          ) : (
            <div className="no-preview">Sin vista previa para este formato. Los efectos se aplican igual.</div>
          )}

          <section className="tool-section">
            <h3>Velocidad</h3>
            <div className="chips">
              {SPEEDS.map((s) => (
                <button key={s} className={speed === s ? "chip on" : "chip"} onClick={() => setSpeed(s)}>
                  {s === 1 ? "Normal" : `×${s}`}
                </button>
              ))}
            </div>
            {speed !== 1 && info?.duration ? (
              <p className="hint">Duración final: {formatDuration(info.duration / speed)}. El audio mantiene su tono.</p>
            ) : null}
          </section>

          <section className="tool-section">
            <h3>Audio</h3>
            <div className="segmented">
              <button className={audio === "keep" ? "on" : ""} onClick={() => setAudio("keep")}>
                Original
              </button>
              <button className={audio === "mute" ? "on" : ""} onClick={() => setAudio("mute")}>
                Quitar
              </button>
              <button className={audio === "replace" ? "on" : ""} onClick={() => setAudio("replace")}>
                Reemplazar por música
              </button>
              <button className={audio === "mix" ? "on" : ""} onClick={() => setAudio("mix")}>
                Agregar música de fondo
              </button>
            </div>
            {needsMusic && (
              <div className="row">
                <button onClick={pickMusic}>{music ? "Cambiar música…" : "Elegir música…"}</button>
                {music && <span className="path">{baseName(music)}</span>}
                {audio === "mix" && (
                  <Slider label="Volumen de la música" min={0} max={1} step={0.05} value={musicVolume} onChange={setMusicVolume} format={(v) => `${Math.round(v * 100)}%`} />
                )}
              </div>
            )}
            {audio === "keep" && info && !info.hasAudio && <p className="hint">Este video no tiene audio.</p>}
          </section>

          <section className="tool-section">
            <h3>Formato</h3>
            <div className="chips">
              {ASPECTS.map((a) => (
                <button key={a.v} className={aspect === a.v ? "chip on" : "chip"} onClick={() => setAspect(a.v)} title={a.hint}>
                  {a.label}
                </button>
              ))}
            </div>
            {aspect && (
              <div className="aspect-modes">
                {ASPECT_MODES.map((m) => (
                  <button key={m.v} className={aspectMode === m.v ? "mode-card on" : "mode-card"} onClick={() => setAspectMode(m.v)}>
                    <span className={`mode-pic ${m.v}`} aria-hidden>
                      <span />
                    </span>
                    <strong>{m.label}</strong>
                    <span className="hint">{m.hint}</span>
                  </button>
                ))}
              </div>
            )}
          </section>

          <section className="tool-section">
            <h3>Subtítulos en la imagen</h3>
            <div className="row">
              <button onClick={pickSubs}>{subtitles ? "Cambiar archivo…" : "Elegir archivo .srt / .ass / .vtt…"}</button>
              {subtitles && (
                <>
                  <span className="path">{baseName(subtitles)}</span>
                  <button className="link danger" onClick={() => setSubtitles("")}>
                    Quitar
                  </button>
                </>
              )}
            </div>
            <p className="hint">Quedan "quemados" en el video: se ven en cualquier reproductor y red social.</p>
          </section>

          <section className="tool-section">
            <h3>Imagen</h3>
            <div className="row">
              <Slider label="Brillo" min={-0.5} max={0.5} step={0.01} value={brightness} onChange={setBrightness} defaultValue={0} format={(v) => `${v > 0 ? "+" : ""}${Math.round(v * 100)}`} />
              <Slider label="Contraste" min={0.5} max={2} step={0.05} value={contrast} onChange={setContrast} defaultValue={1} format={(v) => `${Math.round(v * 100)}%`} />
              <Slider label="Saturación" min={0} max={3} step={0.05} value={saturation} onChange={setSaturation} defaultValue={1} format={(v) => `${Math.round(v * 100)}%`} />
            </div>
            <div className="row">
              <button className="link" onClick={() => setSaturation(0)}>
                Blanco y negro
              </button>
              <button
                className="link"
                onClick={() => {
                  setBrightness(0);
                  setContrast(1);
                  setSaturation(1);
                }}
              >
                Restablecer color
              </button>
              <label className="check">
                <input type="checkbox" checked={stabilize} onChange={(e) => setStabilize(e.target.checked)} />
                Estabilizar (reduce el temblor de la cámara)
              </label>
            </div>
          </section>

          <div className="row">
            <label>
              Calidad
              <select value={quality} onChange={(e) => setQuality(e.target.value as Quality)}>
                <option value="high">Alta</option>
                <option value="medium">Media</option>
                <option value="low">Baja</option>
              </select>
            </label>
            <button className="link" onClick={reset}>
              Restablecer todo
            </button>
          </div>
          <div className="row end">
            <span className="hint">
              {changes.length ? `Cambios: ${changes.join(", ")}` : "Sin cambios todavía"} · Guardar en: {settings.outDir || "…"}
            </span>
            <button className="primary" onClick={exportVideo} disabled={!settings.outDir || (needsMusic && !music)}>
              Exportar MP4
            </button>
          </div>
        </>
      )}
    </div>
  );
}
