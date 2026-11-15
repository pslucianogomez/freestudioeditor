import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { convertFileSrc } from "@tauri-apps/api/core";
import { api, type PhotoEditRequest, type Position } from "../lib/api";
import type { Settings } from "../lib/settings";
import type { JobsApi } from "../lib/jobs";
import { IMAGE_EXT, baseName, extOf, formatBytes } from "../lib/format";
import { useIncoming, type Incoming } from "../lib/incoming";
import { Slider } from "../components/Slider";

/** Formatos que se pueden guardar igual que el original; el resto (HEIC, TIFF...) se guarda como JPG. */
const VIEWABLE = ["jpg", "jpeg", "png", "webp", "bmp"];

const CROP_RATIOS: { label: string; r: number | null }[] = [
  { label: "Libre", r: null },
  { label: "1:1", r: 1 },
  { label: "4:3", r: 4 / 3 },
  { label: "3:4", r: 3 / 4 },
  { label: "16:9", r: 16 / 9 },
  { label: "9:16", r: 9 / 16 },
];

const LOOKS: { label: string; b: number; c: number; s: number; sh: number }[] = [
  { label: "Original", b: 0, c: 1, s: 1, sh: 0 },
  { label: "Vívido", b: 0.02, c: 1.15, s: 1.4, sh: 0.6 },
  { label: "Blanco y negro", b: 0, c: 1.1, s: 0, sh: 0 },
  { label: "Suave", b: 0.04, c: 0.9, s: 0.85, sh: 0 },
  { label: "Alto contraste", b: 0, c: 1.4, s: 1.1, sh: 0.8 },
];

const POSITIONS: { v: Position; label: string }[] = [
  { v: "tl", label: "↖" },
  { v: "tr", label: "↗" },
  { v: "center", label: "●" },
  { v: "bl", label: "↙" },
  { v: "br", label: "↘" },
];

type Rect = [number, number, number, number];

function PositionPicker({ value, onChange }: { value: Position; onChange: (p: Position) => void }) {
  return (
    <div className="pos-picker" role="radiogroup" aria-label="Posición">
      {POSITIONS.map((p) => (
        <button key={p.v} className={value === p.v ? "on" : ""} onClick={() => onChange(p.v)} title={p.v}>
          {p.label}
        </button>
      ))}
    </div>
  );
}

export function PhotoEditorPage({ settings, jobs, incoming }: { settings: Settings; jobs: JobsApi; incoming: Incoming | null }) {
  const [file, setFile] = useState<string | null>(null);
  const [natural, setNatural] = useState<{ w: number; h: number; size: number } | null>(null);
  const [baseSrc, setBaseSrc] = useState("");
  const [error, setError] = useState("");

  const [crop, setCrop] = useState<Rect | null>(null);
  const [ratio, setRatio] = useState<number | null>(null);
  const [degrees, setDegrees] = useState(0);
  const [flipH, setFlipH] = useState(false);
  const [flipV, setFlipV] = useState(false);
  const [brightness, setBrightness] = useState(0);
  const [contrast, setContrast] = useState(1);
  const [saturation, setSaturation] = useState(1);
  const [sharpen, setSharpen] = useState(0);

  const [textOn, setTextOn] = useState(false);
  const [text, setText] = useState("© Tu nombre");
  const [textSize, setTextSize] = useState(5);
  const [textColor, setTextColor] = useState("#ffffff");
  const [textPos, setTextPos] = useState<Position>("br");
  const [textOpacity, setTextOpacity] = useState(0.85);

  const [logo, setLogo] = useState("");
  const [logoSize, setLogoSize] = useState(20);
  const [logoPos, setLogoPos] = useState<Position>("tr");
  const [logoOpacity, setLogoOpacity] = useState(0.8);

  const [format, setFormat] = useState<PhotoEditRequest["format"]>("same");
  const [quality, setQuality] = useState(90);

  const [preview, setPreview] = useState<{ src: string; loading: boolean; error: string }>({ src: "", loading: false, error: "" });
  const stage = useRef<HTMLDivElement>(null);
  const dragStart = useRef<[number, number] | null>(null);
  const previewSeq = useRef(0);

  const resetEdits = () => {
    setCrop(null);
    setDegrees(0);
    setFlipH(false);
    setFlipV(false);
    setBrightness(0);
    setContrast(1);
    setSaturation(1);
    setSharpen(0);
    setTextOn(false);
    setLogo("");
  };

  const load = async (path: string) => {
    setError("");
    try {
      const i = await api.fileInfo(path);
      if (!i.width || !i.height) throw new Error(`No se pudo leer ${baseName(path)} como imagen.`);
      resetEdits();
      setFile(path);
      setNatural({ w: i.width, h: i.height, size: i.size });
      // La imagen de la izquierda también la genera FFmpeg: así se ve exactamente igual a como
      // la procesa (fotos de celular con rotación EXIF, HEIC, TIFF...) y el recorte cae donde debe.
      setBaseSrc("");
      const p = await api.photoPreview(buildReq("base", path, true));
      setBaseSrc(convertFileSrc(p));
    } catch (e) {
      setError(String(e));
    }
  };
  useIncoming(incoming, (paths) => paths[0] && load(paths[0]));

  const pick = async () => {
    const r = await open({ multiple: false, filters: [{ name: "Imágenes", extensions: IMAGE_EXT }] });
    if (typeof r === "string") load(r);
  };
  const pickLogo = async () => {
    const r = await open({ multiple: false, filters: [{ name: "Imagen (PNG con transparencia ideal)", extensions: IMAGE_EXT }] });
    if (typeof r === "string") setLogo(r);
  };

  const rotate = ({ 0: undefined, 90: "cw", 180: "180", 270: "ccw" } as const)[degrees as 0 | 90 | 180 | 270];

  function buildReq(id: string, input = file!, plain = false): PhotoEditRequest {
    if (plain) {
      return { id, input, outDir: settings.outDir, format: "jpg", quality: 90, flipH: false, flipV: false, brightness: 0, contrast: 1, saturation: 1, sharpen: 0 };
    }
    return {
      id,
      input,
      outDir: settings.outDir,
      format,
      quality,
      crop: crop && crop[2] > 1 && crop[3] > 1 ? (crop.map(Math.floor) as Rect) : undefined,
      rotate,
      flipH,
      flipV,
      brightness,
      contrast,
      saturation,
      sharpen,
      text: textOn && text.trim() ? { text, sizePct: textSize, position: textPos, opacity: textOpacity, color: textColor } : undefined,
      logo: logo ? { path: logo, widthPct: logoSize, position: logoPos, opacity: logoOpacity } : undefined,
    };
  }

  // Vista previa exacta: se regenera medio segundo después del último cambio.
  const deps = [file, crop, degrees, flipH, flipV, brightness, contrast, saturation, sharpen, textOn, text, textSize, textColor, textPos, textOpacity, logo, logoSize, logoPos, logoOpacity];
  useEffect(() => {
    if (!file) return;
    const seq = ++previewSeq.current;
    setPreview((p) => ({ ...p, loading: true, error: "" }));
    const t = setTimeout(() => {
      api
        .photoPreview(buildReq("preview"))
        .then((path) => seq === previewSeq.current && setPreview({ src: convertFileSrc(path), loading: false, error: "" }))
        .catch((e) => seq === previewSeq.current && setPreview((p) => ({ ...p, loading: false, error: String(e) })));
    }, 500);
    return () => clearTimeout(t);
  }, deps); // eslint-disable-line react-hooks/exhaustive-deps

  // ---- recorte: arrastrar sobre la imagen original
  const toNatural = (clientX: number, clientY: number): [number, number] => {
    const r = stage.current!.querySelector("img")!.getBoundingClientRect();
    const x = ((clientX - r.left) / r.width) * natural!.w;
    const y = ((clientY - r.top) / r.height) * natural!.h;
    return [Math.max(0, Math.min(natural!.w, x)), Math.max(0, Math.min(natural!.h, y))];
  };
  const onDown = (e: React.MouseEvent) => {
    e.preventDefault();
    dragStart.current = toNatural(e.clientX, e.clientY);
  };
  const onMove = (e: React.MouseEvent) => {
    if (!dragStart.current || !natural) return;
    const [sx, sy] = dragStart.current;
    const [cx, cy] = toNatural(e.clientX, e.clientY);
    let w = Math.abs(cx - sx);
    let h = Math.abs(cy - sy);
    if (ratio) {
      // Con proporción fija manda el lado que más se arrastró.
      if (w / h > ratio) h = w / ratio;
      else w = h * ratio;
    }
    let x = cx < sx ? sx - w : sx;
    let y = cy < sy ? sy - h : sy;
    x = Math.max(0, Math.min(x, natural.w - w));
    y = Math.max(0, Math.min(y, natural.h - h));
    w = Math.min(w, natural.w);
    h = Math.min(h, natural.h);
    setCrop([x, y, w, h]);
  };
  const onUp = () => {
    dragStart.current = null;
    setCrop((c) => (c && (c[2] < 8 || c[3] < 8) ? null : c));
  };

  const applyLook = (l: (typeof LOOKS)[number]) => {
    setBrightness(l.b);
    setContrast(l.c);
    setSaturation(l.s);
    setSharpen(l.sh);
  };

  const exportPhoto = async () => {
    if (!file) return;
    const id = crypto.randomUUID();
    await jobs.run({ id, type: "photo", title: `${baseName(file)} → editada`, inputSize: natural?.size }, () => api.photoEdit(buildReq(id)));
  };

  const pct = (v: number, total: number) => `${(v / total) * 100}%`;
  const finalSize = (() => {
    if (!natural) return "";
    let [w, h] = crop ? [Math.round(crop[2]), Math.round(crop[3])] : [natural.w, natural.h];
    if (degrees === 90 || degrees === 270) [w, h] = [h, w];
    return `${w}×${h}`;
  })();

  if (!file || !natural) {
    return (
      <div className="panel">
        <h1>Editor de fotos</h1>
        <p className="hint">Recortá, rotá, ajustá el color y agregá una marca de agua con texto o tu logo.</p>
        <button className="dropzone" onClick={pick}>
          Elegí o arrastrá una foto…
        </button>
        {error && <p className="error-text">{error}</p>}
      </div>
    );
  }

  return (
    <div className="panel">
      <div className="row end">
        <h1>Editor de fotos</h1>
        <span>
          <strong title={file}>{baseName(file)}</strong>{" "}
          <span className="hint">
            {natural.w}×{natural.h} · {formatBytes(natural.size)}
          </span>{" "}
          <button className="link" onClick={pick}>
            Abrir otra…
          </button>
        </span>
      </div>

      <div className="photo-panes">
        <figure>
          <figcaption>Original — arrastrá para recortar</figcaption>
          <div className="crop-stage" ref={stage} onMouseDown={onDown} onMouseMove={onMove} onMouseUp={onUp} onMouseLeave={onUp}>
            {baseSrc && <img src={baseSrc} alt="" draggable={false} />}
            {crop && (
              <div
                className="crop-rect"
                style={{ left: pct(crop[0], natural.w), top: pct(crop[1], natural.h), width: pct(crop[2], natural.w), height: pct(crop[3], natural.h) }}
              />
            )}
          </div>
        </figure>
        <figure>
          <figcaption>
            Resultado {finalSize && `· ${finalSize}`} {preview.loading && <span className="hint">· actualizando…</span>}
          </figcaption>
          <div className="result-stage">{preview.src ? <img src={preview.src} alt="" /> : <div className="hint">Generando…</div>}</div>
          {preview.error && <p className="error-text">{preview.error}</p>}
        </figure>
      </div>

      <section className="tool-section">
        <h3>Recortar y rotar</h3>
        <div className="row">
          <div className="chips">
            {CROP_RATIOS.map((c) => (
              <button key={c.label} className={ratio === c.r ? "chip on" : "chip"} onClick={() => { setRatio(c.r); setCrop(null); }}>
                {c.label}
              </button>
            ))}
          </div>
          {crop && (
            <button className="link danger" onClick={() => setCrop(null)}>
              Quitar recorte
            </button>
          )}
        </div>
        <div className="row">
          <button onClick={() => setDegrees((d) => (d + 270) % 360)} title="Girar a la izquierda">
            ↺ Izquierda
          </button>
          <button onClick={() => setDegrees((d) => (d + 90) % 360)} title="Girar a la derecha">
            ↻ Derecha
          </button>
          <label className="check">
            <input type="checkbox" checked={flipH} onChange={(e) => setFlipH(e.target.checked)} />
            Espejo horizontal
          </label>
          <label className="check">
            <input type="checkbox" checked={flipV} onChange={(e) => setFlipV(e.target.checked)} />
            Espejo vertical
          </label>
        </div>
      </section>

      <section className="tool-section">
        <h3>Color</h3>
        <div className="chips">
          {LOOKS.map((l) => (
            <button key={l.label} className="chip" onClick={() => applyLook(l)}>
              {l.label}
            </button>
          ))}
        </div>
        <div className="row">
          <Slider label="Brillo" min={-0.5} max={0.5} step={0.01} value={brightness} onChange={setBrightness} defaultValue={0} format={(v) => `${v > 0 ? "+" : ""}${Math.round(v * 100)}`} />
          <Slider label="Contraste" min={0.5} max={2} step={0.05} value={contrast} onChange={setContrast} defaultValue={1} format={(v) => `${Math.round(v * 100)}%`} />
          <Slider label="Saturación" min={0} max={3} step={0.05} value={saturation} onChange={setSaturation} defaultValue={1} format={(v) => `${Math.round(v * 100)}%`} />
          <Slider label="Nitidez" min={0} max={2} step={0.1} value={sharpen} onChange={setSharpen} defaultValue={0} format={(v) => v.toFixed(1)} />
        </div>
      </section>

      <section className="tool-section">
        <h3>
          <label className="check">
            <input type="checkbox" checked={textOn} onChange={(e) => setTextOn(e.target.checked)} />
            Texto (marca de agua)
          </label>
        </h3>
        {textOn && (
          <div className="row">
            <input value={text} onChange={(e) => setText(e.target.value)} placeholder="© Tu nombre" style={{ minWidth: 200 }} />
            <label>
              Color
              <input type="color" value={textColor} onChange={(e) => setTextColor(e.target.value)} />
            </label>
            <Slider label="Tamaño" min={1} max={20} step={0.5} value={textSize} onChange={setTextSize} format={(v) => `${v}%`} />
            <Slider label="Opacidad" min={0.1} max={1} step={0.05} value={textOpacity} onChange={setTextOpacity} format={(v) => `${Math.round(v * 100)}%`} />
            <PositionPicker value={textPos} onChange={setTextPos} />
          </div>
        )}
      </section>

      <section className="tool-section">
        <h3>Logo (marca de agua)</h3>
        <div className="row">
          <button onClick={pickLogo}>{logo ? "Cambiar logo…" : "Elegir logo…"}</button>
          {logo && (
            <>
              <span className="path">{baseName(logo)}</span>
              <Slider label="Ancho" min={5} max={60} step={1} value={logoSize} onChange={setLogoSize} format={(v) => `${v}%`} />
              <Slider label="Opacidad" min={0.1} max={1} step={0.05} value={logoOpacity} onChange={setLogoOpacity} format={(v) => `${Math.round(v * 100)}%`} />
              <PositionPicker value={logoPos} onChange={setLogoPos} />
              <button className="link danger" onClick={() => setLogo("")}>
                Quitar
              </button>
            </>
          )}
        </div>
      </section>

      <div className="row">
        <label>
          Formato
          <select value={format} onChange={(e) => setFormat(e.target.value as PhotoEditRequest["format"])}>
            <option value="same">El mismo{VIEWABLE.includes(extOf(file)) ? "" : " (JPG)"}</option>
            <option value="jpg">JPG</option>
            <option value="png">PNG</option>
            <option value="webp">WebP</option>
          </select>
        </label>
        <Slider label="Calidad" min={30} max={100} step={1} value={quality} onChange={setQuality} />
        <button className="link" onClick={resetEdits}>
          Restablecer todo
        </button>
      </div>
      <div className="row end">
        <span className="hint">Guardar en: {settings.outDir || "…"}</span>
        <button className="primary" onClick={exportPhoto} disabled={!settings.outDir}>
          Guardar foto
        </button>
      </div>
    </div>
  );
}
