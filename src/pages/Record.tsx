import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { Settings } from "../lib/settings";
import type { JobsApi } from "../lib/jobs";
import { formatDuration, stamp } from "../lib/format";

export function RecordPage({ settings, jobs }: { settings: Settings; jobs: JobsApi }) {
  const [devices, setDevices] = useState<string[] | null>(null);
  const [device, setDevice] = useState("");
  const [fps, setFps] = useState(30);
  const [cursor, setCursor] = useState(true);

  useEffect(() => {
    api.listAudioDevices().then(setDevices).catch(() => setDevices([]));
  }, []);

  const active = jobs.jobs.find((j) => j.type === "record" && j.status === "running");

  const start = async () => {
    const id = crypto.randomUUID();
    const name = `Grabación ${stamp()}`;
    await jobs.run({ id, type: "record", title: name }, () =>
      api.recordScreen({ id, outDir: settings.outDir, name, fps, audioDevice: device || undefined, cursor }),
    );
  };

  return (
    <div className="panel">
      <h1>Grabar pantalla</h1>
      <p className="hint">
        Graba la pantalla completa en MP4. Podés minimizar FreeEditorStudio mientras grabás; para terminar, volvé y tocá
        “Detener”.
      </p>

      <div className="row">
        <label>
          Micrófono
          <select value={device} onChange={(e) => setDevice(e.target.value)} disabled={!!active}>
            <option value="">Sin audio</option>
            {(devices ?? []).map((d) => (
              <option key={d} value={d}>
                {d}
              </option>
            ))}
          </select>
        </label>
        <label>
          Cuadros/seg
          <select value={fps} onChange={(e) => setFps(Number(e.target.value))} disabled={!!active}>
            {[15, 30, 60].map((f) => (
              <option key={f}>{f}</option>
            ))}
          </select>
        </label>
        <label className="check">
          <input type="checkbox" checked={cursor} onChange={(e) => setCursor(e.target.checked)} disabled={!!active} />
          Mostrar el puntero
        </label>
      </div>
      {devices !== null && devices.length === 0 && (
        <p className="hint">No se encontraron micrófonos: se grabará solo la imagen.</p>
      )}
      <p className="hint">
        El sonido de la PC (lo que se escucha por los parlantes) solo se puede grabar si Windows tiene habilitada la
        entrada “Mezcla estéreo” (Stereo Mix); en ese caso aparece en la lista.
      </p>

      <div className="row end">
        <span className="hint">Guardar en: {settings.outDir || "…"}</span>
        {active ? (
          <button className="primary rec" onClick={() => api.stopRecording(active.id)}>
            ■ Detener ({formatDuration(active.elapsed ?? 0)})
          </button>
        ) : (
          <button className="primary" onClick={start} disabled={!settings.outDir}>
            ● Empezar a grabar
          </button>
        )}
      </div>
    </div>
  );
}
