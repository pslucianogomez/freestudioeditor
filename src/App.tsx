import { useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { DownloadPage } from "./pages/Download";
import { ConvertPage } from "./pages/Convert";
import { VideoEditorPage } from "./pages/VideoEditor";
import { AudioEditorPage } from "./pages/AudioEditor";
import { RecordPage } from "./pages/Record";
import { PhotosPage } from "./pages/Photos";
import { PhotoEditorPage } from "./pages/PhotoEditor";
import { VideoEffectsPage } from "./pages/VideoEffects";
import { SettingsPage } from "./pages/SettingsPage";
import { JobList } from "./components/JobList";
import { useJobs } from "./lib/jobs";
import { useSettings } from "./lib/settings";
import { AUDIO_EXT, IMAGE_EXT, extOf } from "./lib/format";
import type { Incoming } from "./lib/incoming";

type Tab = "download" | "convert" | "video" | "fx" | "audio" | "photoedit" | "photos" | "record" | "settings";

const TABS: { id: Tab; label: string }[] = [
  { id: "download", label: "Descargar" },
  { id: "convert", label: "Convertir" },
  { id: "video", label: "Editor de video" },
  { id: "fx", label: "Efectos de video" },
  { id: "audio", label: "Editor de audio" },
  { id: "photoedit", label: "Editor de fotos" },
  { id: "photos", label: "Fotos por lote" },
  { id: "record", label: "Grabar pantalla" },
  { id: "settings", label: "Ajustes" },
];

/** Herramientas que aceptan archivos soltados. */
const DROP_TARGETS: Tab[] = ["convert", "video", "fx", "audio", "photoedit", "photos"];

/** A qué herramienta van los archivos soltados, según dónde estás y qué soltaste. */
function routeDrop(tab: Tab, paths: string[]): Tab {
  const allImages = paths.every((p) => IMAGE_EXT.includes(extOf(p)));
  if (allImages) return tab === "photoedit" || (tab !== "photos" && paths.length === 1) ? "photoedit" : "photos";
  if (tab === "photos" || tab === "photoedit") return "convert";
  if (DROP_TARGETS.includes(tab)) return tab;
  const allAudio = paths.every((p) => AUDIO_EXT.includes(extOf(p)));
  return allAudio && paths.length === 1 ? "audio" : "convert";
}

export default function App() {
  const [tab, setTab] = useState<Tab>("download");
  const [settings, update] = useSettings();
  const jobs = useJobs();
  const [dragging, setDragging] = useState(false);
  const [incoming, setIncoming] = useState<{ tab: Tab; data: Incoming } | null>(null);
  const tabRef = useRef(tab);
  tabRef.current = tab;
  const running = jobs.jobs.filter((j) => j.status === "running").length;

  // Arrastrar y soltar archivos desde el Explorador de Windows a cualquier parte de la ventana.
  useEffect(() => {
    const un = getCurrentWebview().onDragDropEvent((e) => {
      const p = e.payload;
      if (p.type === "enter" || p.type === "over") setDragging(true);
      else if (p.type === "leave") setDragging(false);
      else if (p.type === "drop") {
        setDragging(false);
        if (p.paths.length === 0) return;
        const target = routeDrop(tabRef.current, p.paths);
        setTab(target);
        setIncoming({ tab: target, data: { paths: p.paths, seq: Date.now() } });
      }
    });
    return () => {
      un.then((f) => f());
    };
  }, []);

  const inc = (t: Tab) => (incoming?.tab === t ? incoming.data : null);

  return (
    <div className="app">
      <nav className="sidebar">
        <div className="logo">FreeEditorStudio</div>
        {TABS.map((t) => (
          <button key={t.id} className={tab === t.id ? "nav on" : "nav"} onClick={() => setTab(t.id)}>
            {t.label}
          </button>
        ))}
        {running > 0 && <div className="running">{running} en curso</div>}
      </nav>
      <main>
        {tab === "download" && <DownloadPage settings={settings} jobs={jobs} />}
        {tab === "convert" && <ConvertPage settings={settings} jobs={jobs} incoming={inc("convert")} />}
        {tab === "video" && <VideoEditorPage settings={settings} jobs={jobs} incoming={inc("video")} />}
        {tab === "fx" && <VideoEffectsPage settings={settings} jobs={jobs} incoming={inc("fx")} />}
        {tab === "audio" && <AudioEditorPage settings={settings} jobs={jobs} incoming={inc("audio")} />}
        {tab === "photoedit" && <PhotoEditorPage settings={settings} jobs={jobs} incoming={inc("photoedit")} />}
        {tab === "record" && <RecordPage settings={settings} jobs={jobs} />}
        {tab === "photos" && <PhotosPage settings={settings} jobs={jobs} incoming={inc("photos")} />}
        {tab === "settings" && <SettingsPage settings={settings} update={update} />}
        {tab !== "settings" && <JobList jobs={jobs.jobs} onClear={jobs.clearFinished} />}
      </main>
      {dragging && (
        <div className="drop-overlay">
          <div>Soltá los archivos acá</div>
        </div>
      )}
    </div>
  );
}
