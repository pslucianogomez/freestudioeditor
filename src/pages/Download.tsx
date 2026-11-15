import { useEffect, useRef, useState } from "react";
import { readText } from "@tauri-apps/plugin-clipboard-manager";
import { api } from "../lib/api";
import type { Settings } from "../lib/settings";
import type { JobsApi } from "../lib/jobs";
import { formatSize } from "../lib/format";
import { PreviewCard, type PreviewState } from "../components/PreviewCard";
import { DEFAULT_SUBS, SubtitlePicker, subsLangs, type SubsChoice } from "../components/SubtitlePicker";

// Se usan cuando todavía no hay vista previa (o hay varios enlaces).
const DEFAULT_HEIGHTS = [2160, 1440, 1080, 720, 480, 360];
const AUDIO = ["mp3", "m4a", "opus", "flac", "wav"];

/** ¿El enlace parece una playlist o un canal? Solo para el valor inicial del checkbox. */
const looksLikePlaylist = (u: string) => /[?&]list=|\/playlist|\/@|\/channel\/|\/c\/|\/user\//i.test(u);

export function DownloadPage({ settings, jobs }: { settings: Settings; jobs: JobsApi }) {
  const [urls, setUrls] = useState("");
  const [mode, setMode] = useState<"video" | "audio">("video");
  const [quality, setQuality] = useState(1080); // 0 = la mejor
  const [audioFormat, setAudioFormat] = useState("mp3");
  const [subs, setSubs] = useState<SubsChoice>(DEFAULT_SUBS);
  const [playlist, setPlaylist] = useState(false);
  const [preview, setPreview] = useState<PreviewState>({ status: "idle" });
  const probeSeq = useRef(0);

  const list = urls
    .split(/\s+/)
    .map((u) => u.trim())
    .filter((u) => /^https?:\/\//i.test(u));
  const single = list.length === 1 ? list[0] : null;

  // Al pegar un enlace nuevo: checkbox de playlist según la forma del enlace, y se olvidan los idiomas elegidos.
  useEffect(() => {
    if (single) setPlaylist(looksLikePlaylist(single));
    setSubs((s) => ({ ...s, langs: [] }));
  }, [single]);

  // Vista previa: espera 500 ms después de que dejás de escribir y le pregunta a yt-dlp.
  useEffect(() => {
    if (!single) {
      setPreview({ status: "idle" });
      return;
    }
    const seq = ++probeSeq.current;
    setPreview({ status: "loading" });
    const t = setTimeout(() => {
      api
        .probe(single, playlist, settings.cookiesBrowser || undefined)
        .then((data) => {
          if (seq !== probeSeq.current) return; // llegó tarde: ya pegaste otro enlace
          setPreview({ status: "ok", data });
          const hs = data.heights.map((h) => h.height);
          // Si la calidad elegida no existe en este video, se pasa a la mejor que haya hasta 1080p.
          setQuality((q) => (q === 0 || hs.length === 0 || hs.includes(q) ? q : (hs.find((h) => h <= 1080) ?? hs[0])));
          // Preselecciona español o inglés si el autor los cargó.
          const manual = data.subs.filter((s) => !s.auto).map((s) => s.code);
          const pre = manual.filter((c) => /^(es|en)\b/.test(c)).slice(0, 2);
          setSubs((s) => ({ ...s, langs: pre }));
        })
        .catch((e) => {
          if (seq === probeSeq.current) setPreview({ status: "error", error: String(e) });
        });
    }, 500);
    return () => clearTimeout(t);
  }, [single, playlist, settings.cookiesBrowser]);

  const probe = preview.status === "ok" ? preview.data : null;
  const isPlaylist = probe?.kind === "playlist";
  const realHeights = probe && !isPlaylist && probe.heights.length > 0 ? probe.heights : null;
  // Lista de subtítulos: solo se conoce con la vista previa de un video suelto.
  const availableSubs = single && probe && !isPlaylist ? probe.subs : null;

  const qualityOptions = [
    { v: 0, label: "La mejor" },
    ...(realHeights
      ? realHeights.map((h) => ({
          v: h.height,
          label: [`${h.height}p`, h.fps && h.fps > 30 ? `${Math.round(h.fps)} fps` : "", formatSize(h.size)]
            .filter(Boolean)
            .join(" · "),
        }))
      : DEFAULT_HEIGHTS.map((h) => ({ v: h, label: h === 2160 ? "4K (2160p)" : `${h}p` }))),
  ];

  const paste = async () => {
    try {
      const t = (await readText()).trim();
      if (t) setUrls((u) => (u.trim() ? `${u.trim()}\n${t}` : t));
    } catch {
      /* portapapeles vacío o sin texto */
    }
  };

  const langs = subsLangs(subs, availableSubs);
  const subsReq =
    subs.mode !== "none" && langs.length > 0
      ? { langs, only: subs.mode === "only", format: subs.format, embed: subs.embed }
      : undefined;

  const start = async () => {
    for (const url of list) {
      const id = crypto.randomUUID();
      const title = single && probe ? probe.title : url;
      await jobs.run({ id, type: "download", title }, () =>
        api.download({
          id,
          url,
          mode,
          maxHeight: quality || undefined,
          audioFormat,
          outDir: settings.outDir,
          subs: subsReq,
          playlist: single ? playlist : looksLikePlaylist(url),
          cookiesBrowser: settings.cookiesBrowser || undefined,
        }),
      );
    }
    setUrls("");
  };

  const showPlaylistToggle = single ? isPlaylist || looksLikePlaylist(single) : false;
  const onlySubs = subs.mode === "only";

  return (
    <div className="panel">
      <h1>Descargar</h1>
      <p className="hint">Pegá uno o varios enlaces (uno por línea). Funciona con videos, playlists y canales.</p>
      <div className="paste-row">
        <textarea
          value={urls}
          onChange={(e) => setUrls(e.target.value)}
          placeholder="https://..."
          rows={3}
          spellCheck={false}
        />
        <button onClick={paste} title="Pegar desde el portapapeles">
          Pegar
        </button>
      </div>

      {single ? (
        <PreviewCard state={preview} />
      ) : (
        list.length > 1 && <p className="hint">{list.length} enlaces. Se descargan uno por uno.</p>
      )}

      {!onlySubs && (
        <div className="row">
          <div className="segmented" role="radiogroup" aria-label="Tipo">
            <button className={mode === "video" ? "on" : ""} onClick={() => setMode("video")}>
              Video
            </button>
            <button className={mode === "audio" ? "on" : ""} onClick={() => setMode("audio")}>
              Solo audio
            </button>
          </div>
          {mode === "video" ? (
            <label>
              Calidad
              <select value={quality} onChange={(e) => setQuality(Number(e.target.value))}>
                {qualityOptions.map((q) => (
                  <option key={q.v} value={q.v}>
                    {q.label}
                  </option>
                ))}
              </select>
            </label>
          ) : (
            <label>
              Formato
              <select value={audioFormat} onChange={(e) => setAudioFormat(e.target.value)}>
                {AUDIO.map((f) => (
                  <option key={f}>{f}</option>
                ))}
              </select>
              {probe && !isPlaylist && probe.audioSize ? <span>{formatSize(probe.audioSize)}</span> : null}
            </label>
          )}
        </div>
      )}
      <div className="row">
        {showPlaylistToggle && (
          <label className="check">
            <input type="checkbox" checked={playlist} onChange={(e) => setPlaylist(e.target.checked)} />
            Playlist / canal completo
          </label>
        )}
      </div>
      <SubtitlePicker value={subs} onChange={setSubs} available={availableSubs} videoMode={mode === "video"} />

      <div className="row end">
        <span className="hint">Guardar en: {settings.outDir || "…"}</span>
        <button
          className="primary"
          disabled={list.length === 0 || !settings.outDir || (subs.mode !== "none" && langs.length === 0)}
          onClick={start}
        >
          {onlySubs
            ? "Descargar subtítulos"
            : isPlaylist && playlist
              ? `Descargar ${probe?.count ?? ""} videos`
              : `Descargar${list.length > 1 ? ` (${list.length})` : ""}`}
        </button>
      </div>
      <p className="legal">Descargá solo contenido propio, con licencia libre o con permiso del autor.</p>
    </div>
  );
}
