import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { api } from "../lib/api";
import type { Job } from "../lib/jobs";
import { formatDuration } from "../lib/format";
import { ResultInfo } from "./ResultInfo";

const STATUS: Record<Job["status"], string> = {
  running: "En curso",
  done: "Listo",
  error: "Error",
  cancelled: "Cancelado",
};

export function JobList({ jobs, onClear }: { jobs: Job[]; onClear: () => void }) {
  if (jobs.length === 0) {
    return <p className="empty">Todavía no hay trabajos. Lo que descargues, conviertas o grabes aparece acá.</p>;
  }
  return (
    <section className="jobs">
      <header>
        <h2>Trabajos</h2>
        <button className="link" onClick={onClear}>
          Limpiar terminados
        </button>
      </header>
      <ul>
        {jobs.map((j) => (
          <li key={j.id} className={`job ${j.status}`}>
            <div className="job-head">
              <span className="job-title" title={j.title}>
                {j.title}
              </span>
              <span className={`badge ${j.status}`}>
                {j.type === "record" && j.status === "running" ? "Grabando" : STATUS[j.status]}
              </span>
            </div>
            {j.status === "running" &&
              (j.type === "record" ? (
                <div className="job-meta">
                  <span className="rec-dot" /> {formatDuration(j.elapsed ?? 0)}
                  <button className="link" onClick={() => api.stopRecording(j.id)}>
                    Detener y guardar
                  </button>
                </div>
              ) : (
                <>
                  <div className="bar">
                    <div style={{ width: `${Math.min(100, j.percent)}%` }} />
                  </div>
                  <div className="job-meta">
                    {j.percent.toFixed(1)}%{j.speed ? ` · ${j.speed}` : ""}
                    {j.eta ? ` · quedan ${j.eta}` : ""}
                    <button className="link" onClick={() => api.cancel(j.id)}>
                      Cancelar
                    </button>
                  </div>
                </>
              ))}
            {j.status === "done" && j.output && (
              <div className="job-meta">
                <ResultInfo path={j.output} inputSize={j.inputSize} />
                <span className="path">{j.output}</span>
                <button className="link" onClick={() => revealItemInDir(j.output!)}>
                  Abrir carpeta
                </button>
              </div>
            )}
            {j.status === "error" && <pre className="error">{j.error || "Falló sin mensaje."}</pre>}
          </li>
        ))}
      </ul>
    </section>
  );
}
