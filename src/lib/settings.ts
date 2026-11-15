import { useEffect, useState } from "react";
import { downloadDir } from "@tauri-apps/api/path";

export interface Settings {
  outDir: string;
  cookiesBrowser: string; // "" = no usar cookies
}

const KEY = "freeeditorstudio.settings";

function load(): Partial<Settings> {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? "{}");
  } catch {
    return {};
  }
}

export function useSettings() {
  const [settings, setSettings] = useState<Settings>({ outDir: "", cookiesBrowser: "", ...load() });

  useEffect(() => {
    if (!settings.outDir) {
      downloadDir()
        .then((d) => setSettings((s) => ({ ...s, outDir: d })))
        .catch(() => {});
    }
  }, [settings.outDir]);

  const update = (patch: Partial<Settings>) =>
    setSettings((s) => {
      const next = { ...s, ...patch };
      try {
        localStorage.setItem(KEY, JSON.stringify(next));
      } catch {
        /* sin almacenamiento: se usa la sesión */
      }
      return next;
    });

  return [settings, update] as const;
}
