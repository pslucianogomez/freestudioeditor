import { useEffect, useState } from "react";
import { api, type FileInfo } from "../lib/api";
import { formatBytes } from "../lib/format";

/** Muestra el tamaño final de un archivo, y cuánto cambió respecto del original si se conoce. */
export function ResultInfo({ path, inputSize }: { path: string; inputSize?: number }) {
  const [info, setInfo] = useState<FileInfo | null>(null);

  useEffect(() => {
    api.fileInfo(path).then(setInfo).catch(() => setInfo(null));
  }, [path]);

  if (!info) return null;
  const dims = info.width && info.height ? ` · ${info.width}×${info.height}` : "";
  if (!inputSize) return <span className="result">{formatBytes(info.size)}{dims}</span>;
  const pct = Math.round(((info.size - inputSize) / inputSize) * 100);
  return (
    <span className="result">
      {formatBytes(inputSize)} → <strong>{formatBytes(info.size)}</strong>{" "}
      <span className={pct <= 0 ? "smaller" : "bigger"}>
        ({pct > 0 ? "+" : ""}
        {pct}%)
      </span>
      {dims}
    </span>
  );
}
