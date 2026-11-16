//! Descargas con yt-dlp y vista previa de enlaces.

use crate::jobs::{emit, ev, ffmpeg_path, output, spawn_job};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;

// ---------------------------------------------------------------- descarga

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsOptions {
    /// Códigos de idioma: ["es", "en"]
    pub langs: Vec<String>,
    /// Bajar solo los subtítulos, sin el video
    pub only: bool,
    /// srt | vtt
    pub format: String,
    /// Incrustarlos dentro del video
    pub embed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub id: String,
    pub url: String,
    /// "video" | "audio"
    pub mode: String,
    /// Altura máxima (2160, 1080, 720...). None = la mejor disponible.
    pub max_height: Option<u32>,
    /// mp3 | m4a | opus | flac | wav
    pub audio_format: Option<String>,
    pub out_dir: String,
    pub subs: Option<SubsOptions>,
    pub playlist: bool,
    /// Navegador del que leer cookies (opcional): firefox | chrome | edge | brave
    pub cookies_browser: Option<String>,
}

fn s(v: &str) -> String {
    v.to_string()
}

pub fn download_args(r: &DownloadRequest) -> Vec<String> {
    let mut a: Vec<String> = vec![
        s("--newline"),
        s("--progress"),
        s("--no-simulate"),
        s("--color"),
        s("never"),
        s("--encoding"),
        s("utf-8"),
        s("--ffmpeg-location"),
        ffmpeg_path().to_string_lossy().into_owned(),
        s("--progress-template"),
        s("download:OMPROG:%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s"),
        s("--print"),
        s("video:OMTITLE:%(title)s"),
        s("--print"),
        s("after_move:OMFILE:%(filepath)s"),
        s("-P"),
        r.out_dir.clone(),
    ];
    if r.playlist {
        a.extend([s("--yes-playlist"), s("-o"), s("%(playlist_title|.)s/%(title)s [%(id)s].%(ext)s")]);
    } else {
        a.extend([s("--no-playlist"), s("-o"), s("%(title)s [%(id)s].%(ext)s")]);
    }
    if r.mode == "audio" {
        let fmt = r.audio_format.clone().unwrap_or_else(|| s("mp3"));
        a.extend([s("-x"), s("--audio-format"), fmt, s("--audio-quality"), s("0")]);
        a.extend([s("--embed-metadata"), s("--embed-thumbnail")]);
    } else {
        let f = match r.max_height {
            Some(h) => format!("bv*[height<={h}]+ba/b[height<={h}]"),
            None => s("bv*+ba/b"),
        };
        a.extend([s("-f"), f, s("--merge-output-format"), s("mp4")]);
    }
    if let Some(sub) = r.subs.as_ref().filter(|x| !x.langs.is_empty()) {
        a.extend([s("--write-subs"), s("--write-auto-subs"), s("--sub-langs"), sub.langs.join(",")]);
        if sub.format == "srt" {
            a.extend([s("--convert-subs"), s("srt")]);
        }
        if sub.only {
            a.push(s("--skip-download"));
        } else if sub.embed && r.mode == "video" {
            a.push(s("--embed-subs"));
        }
    }
    if let Some(b) = r.cookies_browser.as_ref().filter(|b| !b.is_empty()) {
        a.extend([s("--cookies-from-browser"), b.clone()]);
    }
    a.push(s("--"));
    a.push(r.url.clone());
    a
}

#[tauri::command]
pub fn download(app: AppHandle, req: DownloadRequest) -> Result<(), String> {
    crate::jobs::ensure_writable(&req.out_dir)?;
    let id = req.id.clone();
    // Si no se descarga un archivo (solo subtítulos), "abrir carpeta" lleva a la carpeta de salida.
    let out = output(Some(req.out_dir.clone()));
    let out2 = out.clone();
    let jid = id.clone();
    spawn_job(&app, id, "yt-dlp", download_args(&req), out, move |app, line, _| {
        if let Some(rest) = line.strip_prefix("OMPROG:") {
            let p: Vec<&str> = rest.split('|').collect();
            let mut e = ev(&jid, "progress");
            e.percent = p.first().and_then(|x| x.trim().trim_end_matches('%').trim().parse().ok());
            e.speed = p.get(1).map(|x| x.trim().to_string());
            e.eta = p.get(2).map(|x| x.trim().to_string());
            emit(app, e);
        } else if let Some(t) = line.strip_prefix("OMTITLE:") {
            let mut e = ev(&jid, "title");
            e.message = Some(t.to_string());
            emit(app, e);
        } else if let Some(f) = line.strip_prefix("OMFILE:") {
            *out2.lock().unwrap() = Some(f.to_string());
        } else if !line.is_empty() {
            let mut e = ev(&jid, "log");
            e.message = Some(line.to_string());
            emit(app, e);
        }
    })
}

// ---------------------------------------------------------------- vista previa

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HeightInfo {
    pub height: u64,
    pub fps: Option<f64>,
    /// Tamaño aproximado en bytes (video de esa calidad + mejor audio)
    pub size: Option<f64>,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SubLang {
    pub code: String,
    pub name: String,
    /// true = generados automáticamente por el sitio
    pub auto: bool,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Probe {
    /// "video" | "playlist"
    pub kind: String,
    pub title: String,
    pub uploader: Option<String>,
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    /// Cantidad de videos (solo playlists)
    pub count: Option<u64>,
    /// Primeros títulos (solo playlists)
    pub entries: Vec<String>,
    /// Calidades de video disponibles, de mayor a menor
    pub heights: Vec<HeightInfo>,
    /// Tamaño aproximado del mejor audio, en bytes
    pub audio_size: Option<f64>,
    /// Subtítulos disponibles (primero los cargados por el autor)
    pub subs: Vec<SubLang>,
}

pub fn probe_args(url: &str, playlist: bool, cookies_browser: &Option<String>) -> Vec<String> {
    let mut a: Vec<String> = vec![s("-J"), s("--no-warnings"), s("--encoding"), s("utf-8")];
    if playlist {
        // --flat-playlist: no entra a cada video, así es rápido aunque tenga 500.
        a.extend([s("--yes-playlist"), s("--flat-playlist")]);
    } else {
        a.push(s("--no-playlist"));
    }
    if let Some(b) = cookies_browser.as_ref().filter(|b| !b.is_empty()) {
        a.extend([s("--cookies-from-browser"), b.clone()]);
    }
    a.push(s("--"));
    a.push(url.to_string());
    a
}

fn sub_langs(v: &Value, key: &str, auto: bool) -> Vec<SubLang> {
    let Some(obj) = v[key].as_object() else { return vec![] };
    let mut out: Vec<SubLang> = obj
        .iter()
        .filter(|(k, _)| k.as_str() != "live_chat")
        .map(|(k, list)| SubLang {
            code: k.clone(),
            name: list
                .as_array()
                .and_then(|l| l.first())
                .and_then(|e| e["name"].as_str())
                .unwrap_or(k)
                .to_string(),
            auto,
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Resume el JSON gigante de yt-dlp en lo que la pantalla necesita.
pub fn summarize(v: &Value) -> Probe {
    let str_of = |x: &Value| x.as_str().map(str::to_string);
    let is_playlist = v["_type"].as_str() == Some("playlist");
    let entries = v["entries"].as_array().cloned().unwrap_or_default();

    // Miniatura: la del video, o la última (más grande) de la lista, o la del primer video.
    let thumbnail = str_of(&v["thumbnail"])
        .or_else(|| v["thumbnails"].as_array().and_then(|t| t.last()).and_then(|t| str_of(&t["url"])))
        .or_else(|| {
            entries.first().and_then(|e| {
                e["thumbnails"].as_array().and_then(|t| t.last()).and_then(|t| str_of(&t["url"]))
            })
        });

    let size_of = |f: &Value| f["filesize"].as_f64().or_else(|| f["filesize_approx"].as_f64());
    let formats = v["formats"].as_array().cloned().unwrap_or_default();
    let has_video = |f: &Value| f["vcodec"].as_str().is_some_and(|c| c != "none");
    let has_audio = |f: &Value| f["acodec"].as_str().is_some_and(|c| c != "none");

    let audio_size = formats
        .iter()
        .filter(|f| has_audio(f) && !has_video(f))
        .filter_map(size_of)
        .fold(None, |m: Option<f64>, x| Some(m.map_or(x, |m| m.max(x))));

    let max_opt = |a: Option<f64>, b: Option<f64>| match (a, b) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    let mut heights: Vec<HeightInfo> = Vec::new();
    for f in formats.iter().filter(|f| has_video(f)) {
        let Some(h) = f["height"].as_u64() else { continue };
        let fps = f["fps"].as_f64();
        // Si el formato ya trae audio no se suma el audio aparte.
        let size = size_of(f).map(|x| if has_audio(f) { x } else { x + audio_size.unwrap_or(0.0) });
        match heights.iter_mut().find(|x| x.height == h) {
            Some(x) => {
                x.fps = max_opt(x.fps, fps);
                x.size = max_opt(x.size, size);
            }
            None => heights.push(HeightInfo { height: h, fps, size }),
        }
    }
    heights.sort_by_key(|h| std::cmp::Reverse(h.height));

    let mut subs = sub_langs(v, "subtitles", false);
    let manual: Vec<String> = subs.iter().map(|x| x.code.clone()).collect();
    subs.extend(sub_langs(v, "automatic_captions", true).into_iter().filter(|x| !manual.contains(&x.code)));

    Probe {
        kind: if is_playlist { "playlist" } else { "video" }.into(),
        title: str_of(&v["title"]).unwrap_or_else(|| "(sin título)".into()),
        uploader: str_of(&v["uploader"]).or_else(|| str_of(&v["channel"])),
        thumbnail,
        duration: v["duration"].as_f64(),
        count: if is_playlist { v["playlist_count"].as_u64().or(Some(entries.len() as u64)) } else { None },
        entries: entries.iter().take(5).filter_map(|e| str_of(&e["title"])).collect(),
        heights,
        audio_size,
        subs,
    }
}

/// Primera línea "ERROR: ..." de yt-dlp, más legible que todo el stderr.
pub fn ytdlp_error(stderr: &str) -> String {
    stderr
        .lines()
        .find(|l| l.starts_with("ERROR:"))
        .map(|l| l.trim_start_matches("ERROR:").trim().to_string())
        .unwrap_or_else(|| stderr.trim().lines().last().unwrap_or("Error desconocido").to_string())
}

#[tauri::command]
pub async fn probe(
    app: AppHandle,
    url: String,
    playlist: bool,
    cookies_browser: Option<String>,
) -> Result<Probe, String> {
    let out = app
        .shell()
        .sidecar("yt-dlp")
        .map_err(|e| e.to_string())?
        .args(probe_args(&url, playlist, &cookies_browser))
        .env("PYTHONIOENCODING", "utf-8")
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if out.status.code() != Some(0) {
        return Err(ytdlp_error(&String::from_utf8_lossy(&out.stderr)));
    }
    let v: Value = serde_json::from_slice(&out.stdout).map_err(|e| format!("Respuesta inválida: {e}"))?;
    Ok(summarize(&v))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req() -> DownloadRequest {
        DownloadRequest {
            id: "1".into(),
            url: "https://example.com/v".into(),
            mode: "audio".into(),
            max_height: None,
            audio_format: Some("mp3".into()),
            out_dir: ".".into(),
            subs: None,
            playlist: false,
            cookies_browser: None,
        }
    }

    /// Si el enlace no es una lista, `playlist_title` viene vacío: la ruta no puede empezar con "/"
    /// (en Windows eso es la raíz del disco y saltea la carpeta elegida).
    #[test]
    fn plantilla_de_lista_no_va_a_la_raiz() {
        let a = download_args(&DownloadRequest { playlist: true, ..req() });
        let i = a.iter().position(|x| x == "-o").unwrap();
        assert!(a[i + 1].starts_with("%(playlist_title|.)s/"), "{}", a[i + 1]);
    }

    #[test]
    fn audio_args() {
        let a = download_args(&req());
        assert!(a.contains(&"-x".to_string()));
        assert!(!a.contains(&"--write-subs".to_string()));
        assert_eq!(a.last().unwrap(), "https://example.com/v");
    }

    #[test]
    fn subs_only_args() {
        let mut r = req();
        r.mode = "video".into();
        r.subs = Some(SubsOptions { langs: vec!["es".into(), "en".into()], only: true, format: "srt".into(), embed: true });
        let a = download_args(&r);
        let i = a.iter().position(|x| x == "--sub-langs").unwrap();
        assert_eq!(a[i + 1], "es,en");
        assert!(a.contains(&"--skip-download".to_string()));
        assert!(a.contains(&"--convert-subs".to_string()));
        // "solo subtítulos" no incrusta
        assert!(!a.contains(&"--embed-subs".to_string()));
    }

    #[test]
    fn summarize_video() {
        let v = serde_json::json!({
            "title": "Demo", "uploader": "Canal", "duration": 125.0, "thumbnail": "https://x/t.jpg",
            "formats": [
                {"vcodec": "none", "acodec": "opus", "filesize": 2_000_000.0},
                {"vcodec": "avc1", "acodec": "none", "height": 1080, "fps": 30.0, "filesize": 50_000_000.0},
                {"vcodec": "vp9",  "acodec": "none", "height": 1080, "fps": 60.0, "filesize_approx": 70_000_000.0},
                {"vcodec": "avc1", "acodec": "mp4a", "height": 360, "fps": 30.0, "filesize": 9_000_000.0},
                {"vcodec": "none", "acodec": "none"}
            ],
            "subtitles": {"es": [{"ext": "vtt", "name": "Español"}], "live_chat": []},
            "automatic_captions": {"es": [{"name": "Español"}], "en": [{"name": "English"}]}
        });
        let p = summarize(&v);
        assert_eq!(p.kind, "video");
        assert_eq!(p.audio_size, Some(2_000_000.0));
        assert_eq!(p.heights.len(), 2);
        assert_eq!(p.heights[0], HeightInfo { height: 1080, fps: Some(60.0), size: Some(72_000_000.0) });
        assert_eq!(p.heights[1].size, Some(9_000_000.0));
        // "es" manual una sola vez, "en" automático, sin live_chat
        assert_eq!(
            p.subs,
            vec![
                SubLang { code: "es".into(), name: "Español".into(), auto: false },
                SubLang { code: "en".into(), name: "English".into(), auto: true },
            ]
        );
    }

    #[test]
    fn summarize_playlist() {
        let v = serde_json::json!({
            "_type": "playlist", "title": "Lista", "playlist_count": 42,
            "entries": [{"title": "Uno"}, {"title": "Dos"}]
        });
        let p = summarize(&v);
        assert_eq!(p.kind, "playlist");
        assert_eq!(p.count, Some(42));
        assert_eq!(p.entries, vec!["Uno", "Dos"]);
    }

    #[test]
    fn error_line() {
        assert_eq!(ytdlp_error("WARNING: x\nERROR: [youtube] abc: Video unavailable\n"), "[youtube] abc: Video unavailable");
    }
}
