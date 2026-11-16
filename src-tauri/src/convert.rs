//! Conversión de video/audio/imagen con FFmpeg.

use crate::jobs::{ffmpeg_base, ffmpeg_progress, output, parse_hms, spawn_job, stem_of, unique_path};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConvertRequest {
    pub id: String,
    pub input: String,
    /// mp4 | webm | mkv | mp3 | m4a | wav | flac | ogg | opus | gif | jpg
    pub preset: String,
    pub out_dir: String,
    /// cw | ccw | 180 | hflip | vflip
    pub rotate: Option<String>,
    /// Recorte: "00:00:10" o segundos
    pub start: Option<String>,
    pub end: Option<String>,
    /// Para jpg: un fotograma cada N segundos
    pub frame_every: Option<f64>,
    /// Altura máxima del video (no agranda)
    pub height: Option<u32>,
    /// high | medium | low
    pub quality: Option<String>,
    /// kbps para formatos de audio con pérdida
    pub audio_bitrate: Option<u32>,
    pub gif_width: Option<u32>,
    pub gif_fps: Option<u32>,
}

fn s(v: &str) -> String {
    v.to_string()
}

pub fn rotate_filter(r: &Option<String>) -> Option<&'static str> {
    match r.as_deref() {
        Some("cw") => Some("transpose=1"),
        Some("ccw") => Some("transpose=2"),
        Some("180") => Some("transpose=1,transpose=1"),
        Some("hflip") => Some("hflip"),
        Some("vflip") => Some("vflip"),
        _ => None,
    }
}

/// CRF según la calidad pedida (menor = mejor).
pub fn crf(quality: &Option<String>, vp9: bool) -> &'static str {
    match (quality.as_deref(), vp9) {
        (Some("high"), false) => "18",
        (Some("low"), false) => "28",
        (_, false) => "23",
        (Some("high"), true) => "24",
        (Some("low"), true) => "38",
        (_, true) => "32",
    }
}

/// Escala a una altura máxima sin agrandar; -2 mantiene el ancho par.
pub fn scale_filter(height: Option<u32>) -> Option<String> {
    height.map(|h| format!("scale=-2:'min({h},ih)'"))
}

fn audio_codec_args(preset: &str, bitrate: Option<u32>) -> Option<Vec<String>> {
    let br = |default: u32| format!("{}k", bitrate.unwrap_or(default));
    Some(match preset {
        "mp3" => match bitrate {
            Some(_) => vec![s("-c:a"), s("libmp3lame"), s("-b:a"), br(192)],
            None => vec![s("-c:a"), s("libmp3lame"), s("-q:a"), s("2")],
        },
        "m4a" => vec![s("-c:a"), s("aac"), s("-b:a"), br(192)],
        "ogg" => vec![s("-c:a"), s("libvorbis"), s("-b:a"), br(192)],
        "opus" => vec![s("-c:a"), s("libopus"), s("-b:a"), br(128)],
        "wav" => vec![s("-c:a"), s("pcm_s16le")],
        "flac" => vec![s("-c:a"), s("flac")],
        _ => return None,
    })
}

pub fn convert_args(r: &ConvertRequest) -> Result<(Vec<String>, String), String> {
    let stem = stem_of(&r.input);
    let dir = Path::new(&r.out_dir);

    let mut a = ffmpeg_base();
    if let Some(st) = r.start.as_ref().filter(|x| !x.is_empty()) {
        a.extend([s("-ss"), st.clone()]);
    }
    if let Some(en) = r.end.as_ref().filter(|x| !x.is_empty()) {
        a.extend([s("-to"), en.clone()]);
    }
    a.extend([s("-i"), r.input.clone()]);

    // Cadena de filtros de video: rotación -> escala -> (filtro propio del formato)
    let vf = |extra: Option<String>, with_scale: bool| -> Vec<String> {
        let mut f: Vec<String> = Vec::new();
        if let Some(x) = rotate_filter(&r.rotate) {
            f.push(x.into());
        }
        if with_scale {
            if let Some(x) = scale_filter(r.height) {
                f.push(x);
            }
        }
        if let Some(x) = extra {
            f.push(x);
        }
        if f.is_empty() {
            vec![]
        } else {
            vec![s("-vf"), f.join(",")]
        }
    };

    let out: PathBuf = match r.preset.as_str() {
        "mp4" | "mkv" => {
            a.extend(vf(None, true));
            a.extend([s("-c:v"), s("libx264"), s("-preset"), s("medium"), s("-crf"), s(crf(&r.quality, false))]);
            a.extend([s("-pix_fmt"), s("yuv420p"), s("-c:a"), s("aac"), s("-b:a"), s("192k")]);
            if r.preset == "mp4" {
                a.extend([s("-movflags"), s("+faststart")]);
            }
            unique_path(dir, &stem, &r.preset)
        }
        "webm" => {
            a.extend(vf(None, true));
            a.extend([s("-c:v"), s("libvpx-vp9"), s("-crf"), s(crf(&r.quality, true)), s("-b:v"), s("0")]);
            a.extend([s("-c:a"), s("libopus"), s("-b:a"), s("128k")]);
            unique_path(dir, &stem, "webm")
        }
        "gif" => {
            let w = r.gif_width.unwrap_or(480);
            let fps = r.gif_fps.unwrap_or(12);
            let g = format!("fps={fps},scale={w}:-1:flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse");
            a.extend(vf(Some(g), false));
            a.extend([s("-loop"), s("0")]);
            unique_path(dir, &stem, "gif")
        }
        "jpg" => {
            let every = r.frame_every.unwrap_or(1.0).max(0.04);
            let sub = dir.join(format!("{stem}_frames"));
            std::fs::create_dir_all(&sub).map_err(|e| e.to_string())?;
            a.extend(vf(Some(format!("fps=1/{every}")), true));
            a.extend([s("-q:v"), s("2")]);
            sub.join(format!("{stem}_%04d.jpg"))
        }
        p => {
            let codec = audio_codec_args(p, r.audio_bitrate).ok_or_else(|| format!("Formato desconocido: {p}"))?;
            a.push(s("-vn"));
            a.extend(codec);
            unique_path(dir, &stem, p)
        }
    };
    let out_str = out.to_string_lossy().into_owned();
    a.push(out_str.clone());
    Ok((a, out_str))
}

#[tauri::command]
pub fn convert(app: AppHandle, req: ConvertRequest) -> Result<(), String> {
    crate::jobs::ensure_writable(&req.out_dir)?;
    let (args, out_path) = convert_args(&req)?;
    // Duración efectiva (considerando recorte) para el porcentaje.
    let start = req.start.as_deref().and_then(parse_hms).unwrap_or(0.0);
    let total = req.end.as_deref().and_then(parse_hms).map(|e| (e - start).max(0.1));
    let progress = ffmpeg_progress(req.id.clone(), start, total);
    spawn_job(&app, req.id, "ffmpeg", args, output(Some(out_path)), progress)
}

// ---------------------------------------------------------------- fotos

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PhotoRequest {
    pub id: String,
    pub input: String,
    pub out_dir: String,
    /// same | jpg | png | webp
    pub format: String,
    /// Ancho final en píxeles (tiene prioridad sobre percent)
    pub width: Option<u32>,
    /// Porcentaje del tamaño original
    pub percent: Option<u32>,
    /// 1 a 100 (jpg / webp)
    pub quality: u32,
}

pub fn photo_args(r: &PhotoRequest) -> (Vec<String>, String) {
    let stem = stem_of(&r.input);
    let src_ext = Path::new(&r.input)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_else(|| s("jpg"));
    let ext = match r.format.as_str() {
        "same" => if src_ext == "jpeg" { s("jpg") } else { src_ext },
        f => f.to_string(),
    };
    let mut a = vec![s("-hide_banner"), s("-y"), s("-i"), r.input.clone()];
    let scale = match (r.width, r.percent) {
        (Some(w), _) => Some(format!("scale={w}:-1:flags=lanczos")),
        (None, Some(p)) => Some(format!("scale=iw*{p}/100:-1:flags=lanczos")),
        _ => None,
    };
    if let Some(f) = scale {
        a.extend([s("-vf"), f]);
    }
    a.extend([s("-frames:v"), s("1")]);
    let q = r.quality.clamp(1, 100);
    match ext.as_str() {
        // -q:v de JPEG va de 2 (mejor) a 31 (peor)
        "jpg" => a.extend([s("-q:v"), (2 + (100 - q) * 29 / 100).to_string()]),
        "webp" => a.extend([s("-quality"), q.to_string()]),
        _ => {}
    }
    let suffix = match (r.width, r.percent) {
        (Some(w), _) => format!("_{w}px"),
        (None, Some(p)) => format!("_{p}pct"),
        _ => s("_conv"),
    };
    let out = unique_path(Path::new(&r.out_dir), &format!("{stem}{suffix}"), &ext).to_string_lossy().into_owned();
    a.push(out.clone());
    (a, out)
}

#[tauri::command]
pub fn photo(app: AppHandle, req: PhotoRequest) -> Result<(), String> {
    crate::jobs::ensure_writable(&req.out_dir)?;
    let (args, out) = photo_args(&req);
    spawn_job(&app, req.id, "ffmpeg", args, output(Some(out)), |_, _, _| {})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mp4_with_scale_and_rotate() {
        let r = ConvertRequest {
            input: "C:/v/clip.mov".into(),
            preset: "mp4".into(),
            out_dir: std::env::temp_dir().to_string_lossy().into(),
            rotate: Some("cw".into()),
            height: Some(720),
            quality: Some("low".into()),
            ..Default::default()
        };
        let (a, out) = convert_args(&r).unwrap();
        let i = a.iter().position(|x| x == "-vf").unwrap();
        assert_eq!(a[i + 1], "transpose=1,scale=-2:'min(720,ih)'");
        let c = a.iter().position(|x| x == "-crf").unwrap();
        assert_eq!(a[c + 1], "28");
        assert!(out.ends_with(".mp4"));
    }

    #[test]
    fn mp3_bitrate() {
        let r = ConvertRequest {
            input: "a.wav".into(),
            preset: "mp3".into(),
            out_dir: ".".into(),
            audio_bitrate: Some(320),
            ..Default::default()
        };
        let (a, _) = convert_args(&r).unwrap();
        assert!(a.windows(2).any(|w| w[0] == "-b:a" && w[1] == "320k"));
        assert!(a.contains(&"-vn".to_string()));
    }

    #[test]
    fn unknown_preset() {
        let r = ConvertRequest { preset: "xyz".into(), out_dir: ".".into(), ..Default::default() };
        assert!(convert_args(&r).is_err());
    }

    #[test]
    fn photo_jpg() {
        let r = PhotoRequest {
            input: "foto.JPEG".into(),
            out_dir: ".".into(),
            format: "same".into(),
            width: Some(800),
            quality: 100,
            ..Default::default()
        };
        let (a, out) = photo_args(&r);
        assert!(a.windows(2).any(|w| w[0] == "-q:v" && w[1] == "2"));
        assert!(out.ends_with("foto_800px.jpg"));
    }
}
