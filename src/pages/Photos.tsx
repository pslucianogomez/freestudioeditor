import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { convertFileSrc } from "@tauri-apps/api/core";
import { api, type FileInfo, type PhotoRequest } from "../lib/api";
import type { Settings } from "../lib/settings";
import type { JobsApi } from "../lib/jobs";
import { IMAGE_EXT, baseName, extOf, formatBytes } from "../lib/format";
import { useIncoming, type Incoming } from "../lib/incoming";

type Format = PhotoRequest["format"];

/** Dimensiones que va a tener la foto con las opciones actuales (igual que calcula FFmpeg). */
function predict(info: FileInfo | undefined, by: "width" | "percent", width: number, percent: number) {
  if (!info?.width || !info?.height) return null;
  const w = by === "width" ? width : Math.round((info.width * percent) / 100);
  const h = Math.round((info.height * w) / info.width);
  return { w, h, upscale: w > info.width };
}

export function PhotosPage({ settings, jobs, incoming }: { settings: Settings; jobs: JobsApi; incoming: Incoming | null }) {
  const [files, setFiles] = useState<string[]>([]);
  const [infos, setInfos] = useState<Record<string, FileInfo>>({});
  const [by, setBy] = useState<"width" | "percent">("width");
  const [width, setWidth] = useState(1280);
  const [percent, setPercent] = useState(50);
  const [format, setFormat] = useState<Format>("same");
  const [quality, setQuality] = useState(85);

  const addFiles = (paths: string[]) => {
    setFiles((f) => [...f, ...paths.filter((p) => !f.includes(p))]);
    for (const p of paths) {
      api
        .fileInfo(p)
        .then((i) => setInfos((m) => ({ ...m, [p]: i })))
        .catch(() => {});
    }
  };
  useIncoming(incoming, addFiles);

  const pick = async () => {
    const r = await open({ multiple: true, filters: [{ name: "Imágenes", extensions: IMAGE_EXT }] });
    if (r) addFiles(Array.isArray(r) ? r : [r]);
  };

  const run = async () => {
    for (const input of files) {
      const id = crypto.randomUUID();
      const out = format === "same" ? extOf(input).replace("jpeg", "jpg") : format;
      const size = by === "width" ? `${width}px` : `${percent}%`;
      await jobs.run(
        { id, type: "photo", title: `${baseName(input)} → ${size} ${out.toUpperCase()}`, inputSize: infos[input]?.size },
        () =>
          api.photo({
            id,
            input,
            outDir: settings.outDir,
            format,
            width: by === "width" ? width : undefined,
            percent: by === "percent" ? percent : undefined,
            quality,
          }),
      );
    }
    // La lista queda cargada: podés probar otras opciones y volver a procesar.
  };

  // La calidad solo afecta a JPG y WebP.
  const targets = new Set(files.map((f) => (format === "same" ? extOf(f).replace("jpeg", "jpg") : format)));
  const qualityApplies = [...targets].some((t) => t === "jpg" || t === "webp");

  return (
    <div className="panel">
      <h1>Fotos</h1>
      <p className="hint">Cambiá el tamaño o el formato de muchas fotos de una vez.</p>
      <div className="row">
        <button className="dropzone grow" onClick={pick}>
          {files.length === 0 ? "Elegí o arrastrá fotos…" : "Agregar más fotos…"}
        </button>
        {files.length > 0 && (
          <button className="link" onClick={() => setFiles([])}>
            Vaciar lista
          </button>
        )}
      </div>
      {files.length > 0 && (
        <div className="thumbs">
          {files.map((f) => {
            const info = infos[f];
            const p = predict(info, by, width, percent);
            return (
              <figure key={f} title={f}>
                <img src={convertFileSrc(f)} alt="" />
                <figcaption>
                  <div className="thumb-name">
                    <span>{baseName(f)}</span>
                    <button className="link danger" onClick={() => setFiles((l) => l.filter((x) => x !== f))} title="Quitar">
                      ×
                    </button>
                  </div>
                  {info && (
                    <div>
                      {info.width}×{info.height} · {formatBytes(info.size)}
                    </div>
                  )}
                  {p && (
                    <div className={p.upscale ? "bigger" : "smaller"}>
                      → {p.w}×{p.h}
                      {p.upscale ? " (se agranda)" : ""}
                    </div>
                  )}
                </figcaption>
              </figure>
            );
          })}
        </div>
      )}

      <div className="row">
        <div className="segmented">
          <button className={by === "width" ? "on" : ""} onClick={() => setBy("width")}>
            Por ancho
          </button>
          <button className={by === "percent" ? "on" : ""} onClick={() => setBy("percent")}>
            Por porcentaje
          </button>
        </div>
        {by === "width" ? (
          <label>
            Ancho
            <input type="number" min={16} max={10000} value={width} onChange={(e) => setWidth(Number(e.target.value))} style={{ width: 84 }} />
            px
          </label>
        ) : (
          <label>
            Tamaño
            <input type="number" min={1} max={400} value={percent} onChange={(e) => setPercent(Number(e.target.value))} style={{ width: 70 }} />%
          </label>
        )}
        <label>
          Formato
          <select value={format} onChange={(e) => setFormat(e.target.value as Format)}>
            <option value="same">El mismo</option>
            <option value="jpg">JPG</option>
            <option value="png">PNG</option>
            <option value="webp">WebP</option>
          </select>
        </label>
        <label title={qualityApplies ? "" : "PNG no tiene calidad: siempre es sin pérdida"}>
          Calidad
          <input
            type="range"
            min={30}
            max={100}
            value={quality}
            onChange={(e) => setQuality(Number(e.target.value))}
            disabled={files.length > 0 && !qualityApplies}
          />
          {qualityApplies || files.length === 0 ? quality : "no aplica a PNG"}
        </label>
      </div>
      <p className="hint">
        Menos calidad = archivo más liviano. Por debajo de 70 empiezan a notarse defectos en JPG y WebP. El tamaño final de
        cada foto aparece en la lista de trabajos al terminar.
      </p>

      <div className="row end">
        <span className="hint">Guardar en: {settings.outDir || "…"}</span>
        <button className="primary" onClick={run} disabled={files.length === 0 || !settings.outDir}>
          Procesar {files.length > 1 ? `(${files.length})` : ""}
        </button>
      </div>
    </div>
  );
}
