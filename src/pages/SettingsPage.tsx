import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "../lib/api";
import type { Settings } from "../lib/settings";

const BROWSERS = [
  { v: "", label: "No usar (recomendado)" },
  { v: "firefox", label: "Firefox" },
  { v: "chrome", label: "Chrome" },
  { v: "edge", label: "Edge" },
  { v: "brave", label: "Brave" },
];

export function SettingsPage({ settings, update }: { settings: Settings; update: (p: Partial<Settings>) => void }) {
  const [versions, setVersions] = useState<{ ytDlp: string; ffmpeg: string } | null>(null);
  const [updating, setUpdating] = useState(false);
  const [updateLog, setUpdateLog] = useState("");

  useEffect(() => {
    api.versions().then(setVersions).catch(() => {});
  }, []);

  const pickDir = async () => {
    const d = await open({ directory: true, defaultPath: settings.outDir || undefined });
    if (typeof d === "string") update({ outDir: d });
  };

  const doUpdate = async () => {
    setUpdating(true);
    setUpdateLog(await api.updateYtDlp().catch((e) => String(e)));
    setVersions(await api.versions().catch(() => versions));
    setUpdating(false);
  };

  return (
    <div className="panel">
      <h1>Ajustes</h1>

      <section className="setting">
        <h3>Carpeta de salida</h3>
        <div className="row">
          <code className="path">{settings.outDir || "…"}</code>
          <button onClick={pickDir}>Cambiar…</button>
        </div>
      </section>

      <section className="setting">
        <h3>Cookies del navegador</h3>
        <p className="hint">
          Solo hace falta para videos privados, con restricción de edad o de miembros, o si el sitio pide
          “confirmar que no sos un bot”. La app lee la sesión del navegador elegido; nunca pide tu contraseña. Firefox
          es el que mejor funciona en Windows. Si lo usás, considerá una cuenta secundaria.
        </p>
        <select value={settings.cookiesBrowser} onChange={(e) => update({ cookiesBrowser: e.target.value })}>
          {BROWSERS.map((b) => (
            <option key={b.v} value={b.v}>
              {b.label}
            </option>
          ))}
        </select>
      </section>

      <section className="setting">
        <h3>Motores</h3>
        <p className="hint">
          yt-dlp: {versions?.ytDlp || "—"}
          <br />
          FFmpeg: {versions?.ffmpeg || "—"}
        </p>
        <button onClick={doUpdate} disabled={updating}>
          {updating ? "Actualizando…" : "Actualizar yt-dlp"}
        </button>
        {updateLog && <pre className="log">{updateLog}</pre>}
      </section>
    </div>
  );
}
