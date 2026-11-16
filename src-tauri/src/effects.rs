//! Efectos de video: velocidad, audio (quitar / reemplazar / mezclar con música),
//! formato (vertical, cuadrado...), subtítulos quemados, estabilizar y color.

use crate::convert::crf;
use crate::filters::{atempo_chain, even, filter_path};
use crate::jobs::{ffmpeg_base, ffmpeg_progress, output, spawn_job, stem_of, unique_path};
use crate::media::{media_info_blocking, MediaInfo};
use serde::Deserialize;
use std::path::Path;
use tauri::AppHandle;

#[derive(Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct VideoFxRequest {
    pub id: String,
    pub input: String,
    pub out_dir: String,
    /// 1.0 = normal, 2.0 = el doble de rápido, 0.5 = cámara lenta
    pub speed: f64,
    /// keep | mute | replace | mix
    pub audio: String,
    /// Música para replace / mix
    pub audio_file: Option<String>,
    /// Volumen de la música al mezclar (0–1)
    pub music_volume: Option<f64>,
    /// "9:16" | "1:1" | "4:5" | "16:9"
    pub aspect: Option<String>,
    /// crop | pad | blur
    pub aspect_mode: Option<String>,
    /// Archivo .srt / .ass / .vtt a quemar en la imagen
    pub subtitles: Option<String>,
    pub stabilize: bool,
    /// -1 a 1 (0 = sin cambio)
    pub brightness: f64,
    /// 0 a 2 (1 = sin cambio)
    pub contrast: f64,
    /// 0 a 3 (1 = sin cambio, 0 = blanco y negro)
    pub saturation: f64,
    pub quality: Option<String>,
}

fn s(v: &str) -> String {
    v.to_string()
}

fn parse_ratio(r: &str) -> Option<(u32, u32)> {
    let (a, b) = r.split_once(':')?;
    let (a, b) = (a.trim().parse().ok()?, b.trim().parse().ok()?);
    (a > 0 && b > 0).then_some((a, b))
}

/// Lienzo de salida para un formato: el lado largo igual al lado largo del original.
pub fn canvas(iw: u32, ih: u32, rw: u32, rh: u32) -> (u32, u32) {
    let m = iw.max(ih) as f64;
    if rw >= rh {
        (even(m as u32), even((m * rh as f64 / rw as f64).round() as u32))
    } else {
        (even((m * rw as f64 / rh as f64).round() as u32), even(m as u32))
    }
}

/// Arma un filter_complex paso a paso, encadenando etiquetas [v0], [v1]...
pub struct Graph {
    pub parts: Vec<String>,
    cur: String,
    n: usize,
}

impl Graph {
    pub fn new(input: &str) -> Self {
        Graph { parts: Vec::new(), cur: input.to_string(), n: 0 }
    }

    fn next_label(&mut self) -> String {
        let l = format!("[v{}]", self.n);
        self.n += 1;
        l
    }

    /// Agrega una cadena lineal de filtros sobre la salida actual.
    pub fn chain(&mut self, filters: &str) {
        let out = self.next_label();
        self.parts.push(format!("{}{filters}{out}", self.cur));
        self.cur = out;
    }

    /// Fondo desenfocado: el video entra encajado sobre una copia agrandada y borrosa.
    pub fn blur_fill(&mut self, cw: u32, ch: u32) {
        self.parts.push(format!("{}split=2[bg0][fg0]", self.cur));
        self.parts.push(format!("[bg0]scale={cw}:{ch}:force_original_aspect_ratio=increase,crop={cw}:{ch},boxblur=20:2[bg]"));
        self.parts.push(format!("[fg0]scale={cw}:{ch}:force_original_aspect_ratio=decrease[fg]"));
        let out = self.next_label();
        self.parts.push(format!("[bg][fg]overlay=(W-w)/2:(H-h)/2{out}"));
        self.cur = out;
    }

    /// Cierra el grafo con `last` y la etiqueta `label`.
    pub fn finish(mut self, last: &str, label: &str) -> Vec<String> {
        self.parts.push(format!("{}{last}{label}", self.cur));
        self.parts
    }
}

/// Grafo de filtros de video. Devuelve los segmentos (unidos con ";") y la etiqueta final.
pub fn video_graph(r: &VideoFxRequest, info: &MediaInfo, subs_path: Option<&str>) -> (Vec<String>, String) {
    let mut g = Graph::new("[0:v]");
    if r.stabilize {
        g.chain("deshake");
    }
    let eq_needed = r.brightness.abs() > 0.001 || (r.contrast - 1.0).abs() > 0.001 || (r.saturation - 1.0).abs() > 0.001;
    if eq_needed {
        g.chain(&format!("eq=brightness={:.3}:contrast={:.3}:saturation={:.3}", r.brightness, r.contrast, r.saturation));
    }
    // Subtítulos antes de cambiar la velocidad, así coinciden con los tiempos del original.
    if let Some(p) = subs_path {
        g.chain(&format!("subtitles=filename={}", filter_path(p)));
    }
    if let Some((rw, rh)) = r.aspect.as_deref().and_then(parse_ratio) {
        let (iw, ih) = (info.width.unwrap_or(1280), info.height.unwrap_or(720));
        let (cw, ch) = canvas(iw, ih, rw, rh);
        match r.aspect_mode.as_deref().unwrap_or("crop") {
            "pad" => g.chain(&format!(
                "scale={cw}:{ch}:force_original_aspect_ratio=decrease,pad={cw}:{ch}:(ow-iw)/2:(oh-ih)/2:color=black"
            )),
            "blur" => g.blur_fill(cw, ch),
            // crop: recorta al centro sin agrandar.
            _ => g.chain(&format!("crop='min(iw,ih*{rw}/{rh})':'min(ih,iw*{rh}/{rw})',scale=trunc(iw/2)*2:trunc(ih/2)*2")),
        }
    }
    if (r.speed - 1.0).abs() > 0.001 {
        g.chain(&format!("setpts=PTS/{:.6}", r.speed));
    }
    (g.finish("format=yuv420p", "[vout]"), s("[vout]"))
}

/// Grafo de audio. None = el resultado no lleva audio.
pub fn audio_graph(r: &VideoFxRequest, info: &MediaInfo) -> Option<String> {
    let tempo = atempo_chain(r.speed).join(",");
    let with_tempo = |base: &str| if tempo.is_empty() { base.to_string() } else { format!("{base},{tempo}") };
    let vol = r.music_volume.unwrap_or(0.3).clamp(0.0, 2.0);
    match r.audio.as_str() {
        "mute" => None,
        // La música no se acelera: se rellena con silencio y el video manda en la duración.
        "replace" => Some(s("[1:a]aresample=48000,apad[aout]")),
        "mix" if info.has_audio => Some(format!(
            "[0:a]{}[a0];[1:a]aresample=48000,volume={vol:.2}[a1];[a0][a1]amix=inputs=2:duration=first:dropout_transition=0:normalize=0[aout]",
            with_tempo("aresample=48000")
        )),
        "mix" => Some(format!("[1:a]aresample=48000,volume={vol:.2},apad[aout]")),
        _ if info.has_audio => Some(format!("[0:a]{}[aout]", with_tempo("aresample=48000"))),
        _ => None,
    }
}

pub fn video_fx_args(r: &VideoFxRequest, info: &MediaInfo, subs_path: Option<&str>, out: &str) -> Result<Vec<String>, String> {
    let needs_music = r.audio == "replace" || r.audio == "mix";
    let music = r.audio_file.as_ref().filter(|f| !f.is_empty());
    if needs_music && music.is_none() {
        return Err("Elegí el archivo de música.".into());
    }
    if !info.has_video {
        return Err("El archivo no tiene video.".into());
    }
    let mut a = ffmpeg_base();
    a.extend([s("-i"), r.input.clone()]);
    if needs_music {
        a.extend([s("-i"), music.unwrap().clone()]);
    }
    let (mut graph, vlabel) = video_graph(r, info, subs_path);
    let alabel = audio_graph(r, info);
    if let Some(ag) = &alabel {
        graph.push(ag.clone());
    }
    a.extend([s("-filter_complex"), graph.join(";"), s("-map"), vlabel]);
    if alabel.is_some() {
        a.extend([s("-map"), s("[aout]"), s("-c:a"), s("aac"), s("-b:a"), s("192k")]);
        if r.audio == "replace" || (r.audio == "mix" && !info.has_audio) {
            a.push(s("-shortest"));
        }
    } else {
        a.push(s("-an"));
    }
    a.extend([s("-c:v"), s("libx264"), s("-preset"), s("medium"), s("-crf"), s(crf(&r.quality, false))]);
    a.extend([s("-movflags"), s("+faststart"), s(out)]);
    Ok(a)
}

/// Copia los subtítulos a un nombre simple en la carpeta temporal: evita que comillas
/// u otros caracteres raros de la ruta rompan el filtro.
fn stage_subtitles(path: &str) -> Result<String, String> {
    let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("srt").to_lowercase();
    let dir = std::env::temp_dir().join("freeeditorstudio");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let dst = dir.join(format!("subs-{stamp}.{ext}"));
    std::fs::copy(path, &dst).map_err(|e| format!("No se pudo leer el archivo de subtítulos: {e}"))?;
    Ok(dst.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn video_fx(app: AppHandle, req: VideoFxRequest) -> Result<(), String> {
    crate::jobs::ensure_writable(&req.out_dir)?;
    let r = req.clone();
    let (info, subs) = tauri::async_runtime::spawn_blocking(move || -> Result<_, String> {
        let info = media_info_blocking(&r.input)?;
        let subs = match r.subtitles.as_ref().filter(|x| !x.is_empty()) {
            Some(p) => Some(stage_subtitles(p)?),
            None => None,
        };
        Ok((info, subs))
    })
    .await
    .map_err(|e| e.to_string())??;

    let out = unique_path(Path::new(&req.out_dir), &format!("{}_efectos", stem_of(&req.input)), "mp4")
        .to_string_lossy()
        .into_owned();
    let args = video_fx_args(&req, &info, subs.as_deref(), &out)?;
    let total = info.duration.map(|d| d / req.speed.max(0.01));
    let progress = ffmpeg_progress(req.id.clone(), 0.0, total);
    spawn_job(&app, req.id, "ffmpeg", args, output(Some(out)), progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(w: u32, h: u32, audio: bool) -> MediaInfo {
        MediaInfo { duration: Some(10.0), width: Some(w), height: Some(h), has_video: true, has_audio: audio }
    }

    fn base() -> VideoFxRequest {
        VideoFxRequest { input: "in.mp4".into(), speed: 1.0, audio: "keep".into(), contrast: 1.0, saturation: 1.0, ..Default::default() }
    }

    #[test]
    fn canvas_sizes() {
        assert_eq!(canvas(1920, 1080, 9, 16), (1080, 1920));
        assert_eq!(canvas(1920, 1080, 1, 1), (1920, 1920));
        assert_eq!(canvas(1080, 1920, 16, 9), (1920, 1080));
    }

    #[test]
    fn plain_copy_has_only_format() {
        let (g, l) = video_graph(&base(), &info(640, 360, true), None);
        assert_eq!(g, vec!["[0:v]format=yuv420p[vout]"]);
        assert_eq!(l, "[vout]");
    }

    #[test]
    fn blur_vertical_and_speed() {
        let mut r = base();
        r.aspect = Some("9:16".into());
        r.aspect_mode = Some("blur".into());
        r.speed = 2.0;
        let (g, _) = video_graph(&r, &info(1920, 1080, true), None);
        let joined = g.join(";");
        assert!(joined.contains("crop=1080:1920,boxblur"));
        assert!(joined.contains("setpts=PTS/2.000000"));
        assert_eq!(audio_graph(&r, &info(1920, 1080, true)).unwrap(), "[0:a]aresample=48000,atempo=2.000000[aout]");
    }

    #[test]
    fn audio_modes() {
        let mut r = base();
        r.audio = "mute".into();
        assert!(audio_graph(&r, &info(1, 1, true)).is_none());
        r.audio = "keep".into();
        assert!(audio_graph(&r, &info(1, 1, false)).is_none());
        r.audio = "mix".into();
        assert!(audio_graph(&r, &info(1, 1, true)).unwrap().contains("amix=inputs=2"));
        r.audio = "replace".into();
        assert!(video_fx_args(&r, &info(1, 1, true), None, "o.mp4").is_err()); // falta la música
    }
}
