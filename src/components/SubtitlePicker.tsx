import type { SubLang } from "../lib/api";

export interface SubsChoice {
  mode: "none" | "with" | "only";
  langs: string[];
  format: "srt" | "vtt";
  embed: boolean;
  /** Texto libre cuando no hay vista previa (playlist o varios enlaces) */
  manual: string;
}

export const DEFAULT_SUBS: SubsChoice = { mode: "none", langs: [], format: "srt", embed: false, manual: "es,en" };

/** Idiomas efectivos a pedir: los elegidos de la lista, o el texto libre. */
export function subsLangs(c: SubsChoice, available: SubLang[] | null): string[] {
  if (available && available.length > 0) return c.langs;
  return c.manual
    .split(/[,\s]+/)
    .map((x) => x.trim())
    .filter(Boolean);
}

export function SubtitlePicker({
  value,
  onChange,
  available,
  videoMode,
}: {
  value: SubsChoice;
  onChange: (v: SubsChoice) => void;
  /** null = no se sabe (sin vista previa); [] = el video no tiene subtítulos */
  available: SubLang[] | null;
  videoMode: boolean;
}) {
  const set = (p: Partial<SubsChoice>) => onChange({ ...value, ...p });
  const toggle = (code: string) =>
    set({ langs: value.langs.includes(code) ? value.langs.filter((c) => c !== code) : [...value.langs, code] });

  const manual = available?.filter((s) => !s.auto) ?? [];
  const auto = available?.filter((s) => s.auto) ?? [];
  const nameOf = (code: string) => available?.find((s) => s.code === code)?.name ?? code;

  return (
    <div className="subs">
      <label>
        Subtítulos
        <select value={value.mode} onChange={(e) => set({ mode: e.target.value as SubsChoice["mode"] })}>
          <option value="none">No</option>
          <option value="with">{videoMode ? "Con el video" : "Con el audio"}</option>
          <option value="only">Solo los subtítulos</option>
        </select>
      </label>

      {value.mode !== "none" && (
        <div className="subs-body">
          {available === null ? (
            <label>
              Idiomas
              <input value={value.manual} onChange={(e) => set({ manual: e.target.value })} placeholder="es,en" size={12} />
              <span className="hint">códigos separados por coma</span>
            </label>
          ) : available.length === 0 ? (
            <span className="hint">Este video no tiene subtítulos.</span>
          ) : (
            <>
              <div className="chips">
                {manual.map((s) => (
                  <button key={s.code} className={value.langs.includes(s.code) ? "chip on" : "chip"} onClick={() => toggle(s.code)}>
                    {s.name}
                  </button>
                ))}
                {value.langs
                  .filter((c) => !manual.some((m) => m.code === c))
                  .map((c) => (
                    <button key={c} className="chip on" onClick={() => toggle(c)} title="Quitar">
                      {nameOf(c)} (auto) ×
                    </button>
                  ))}
              </div>
              {auto.length > 0 && (
                <select value="" onChange={(e) => e.target.value && toggle(e.target.value)}>
                  <option value="">+ Agregar generado automáticamente…</option>
                  {auto
                    .filter((s) => !value.langs.includes(s.code))
                    .map((s) => (
                      <option key={s.code} value={s.code}>
                        {s.name}
                      </option>
                    ))}
                </select>
              )}
            </>
          )}
          <label>
            Formato
            <select value={value.format} onChange={(e) => set({ format: e.target.value as "srt" | "vtt" })}>
              <option value="srt">SRT</option>
              <option value="vtt">VTT</option>
            </select>
          </label>
          {value.mode === "with" && videoMode && (
            <label className="check">
              <input type="checkbox" checked={value.embed} onChange={(e) => set({ embed: e.target.checked })} />
              Incrustar en el video
            </label>
          )}
        </div>
      )}
    </div>
  );
}
