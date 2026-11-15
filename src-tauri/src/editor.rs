//! Editor de video: une tramos de uno o varios archivos en un solo MP4.

use crate::convert::crf;
use crate::jobs::{ffmpeg_base, ffmpeg_progress, output, spawn_job, unique_path};
use crate::media::{media_info_blocking, MediaInfo};
use serde::Deserialize;
use std::path::Path;
use tauri::AppHandle;

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub path: String,
    /// Segundos desde el inicio del archivo
    pub start: f64,
    /// None = hasta el final
    pub end: Option<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinRequest {
    pub id: String,
    pub clips: Vec<Clip>,
    pub out_dir: String,
    /// Altura de salida; None = la del primer tramo
    pub height: Option<u32>,
    /// high | medium | low
    pub quality: Option<String>,
}

fn even(x: u32) -> u32 {
    (x - x % 2).max(2)
}

/// Tramo ya validado: dónde empieza, cuánto dura y si tiene audio.
pub struct Segment {
    pub path: String,
    pub start: f64,
    pub len: f64,
    pub has_audio: bool,
}

pub fn segments(clips: &[Clip], infos: &[MediaInfo]) -> Result<Vec<Segment>, String> {
    clips
        .iter()
        .zip(infos)
        .enumerate()
        .map(|(i, (c, m))| {
            if !m.has_video {
                return Err(format!("El tramo {} no tiene video.", i + 1));
            }
            let end = c.end.or(m.duration).ok_or_else(|| format!("No se pudo leer la duración del tramo {}.", i + 1))?;
            let len = end - c.start;
            if len <= 0.05 {
                return Err(format!("El tramo {} termina antes de empezar.", i + 1));
            }
            Ok(Segment { path: c.path.clone(), start: c.start.max(0.0), len, has_audio: m.has_audio })
        })
        .collect()
}

/// Tamaño de salida: el del primer tramo, o escalado a la altura pedida.
pub fn target_size(first: &MediaInfo, height: Option<u32>) -> (u32, u32) {
    let (w, h) = (first.width.unwrap_or(1280), first.height.unwrap_or(720));
    match height {
        Some(th) => (even((w as f64 * th as f64 / h as f64).round() as u32), even(th)),
        None => (even(w), even(h)),
    }
}

/// Arma el comando: cada tramo se normaliza (tamaño, fps, audio estéreo) y se concatenan.
/// Los tramos sin audio reciben silencio para que la concatenación no falle.
pub fn join_args(segs: &[Segment], w: u32, h: u32, crf_value: &str, out: &str) -> Vec<String> {
    let s = |v: &str| v.to_string();
    let mut a = ffmpeg_base();
    for g in segs {
        a.extend([s("-ss"), format!("{:.3}", g.start), s("-t"), format!("{:.3}", g.len), s("-i"), g.path.clone()]);
    }
    let mut f: Vec<String> = Vec::new();
    let mut labels = String::new();
    for (i, g) in segs.iter().enumerate() {
        f.push(format!(
            "[{i}:v]scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=30,format=yuv420p[v{i}]"
        ));
        if g.has_audio {
            f.push(format!("[{i}:a]aresample=48000,aformat=sample_fmts=fltp:channel_layouts=stereo[a{i}]"));
        } else {
            f.push(format!(
                "anullsrc=r=48000:cl=stereo,atrim=duration={:.3},aformat=sample_fmts=fltp:channel_layouts=stereo[a{i}]",
                g.len
            ));
        }
        labels.push_str(&format!("[v{i}][a{i}]"));
    }
    f.push(format!("{labels}concat=n={}:v=1:a=1[v][a]", segs.len()));
    a.extend([s("-filter_complex"), f.join(";"), s("-map"), s("[v]"), s("-map"), s("[a]")]);
    a.extend([s("-c:v"), s("libx264"), s("-preset"), s("medium"), s("-crf"), s(crf_value)]);
    a.extend([s("-c:a"), s("aac"), s("-b:a"), s("192k"), s("-movflags"), s("+faststart")]);
    a.push(out.to_string());
    a
}

#[tauri::command]
pub async fn join_clips(app: AppHandle, req: JoinRequest) -> Result<(), String> {
    if req.clips.is_empty() {
        return Err("Agregá al menos un tramo.".into());
    }
    let clips = req.clips.clone();
    let infos: Vec<MediaInfo> = tauri::async_runtime::spawn_blocking(move || {
        clips.iter().map(|c| media_info_blocking(&c.path)).collect::<Result<Vec<_>, _>>()
    })
    .await
    .map_err(|e| e.to_string())??;

    let segs = segments(&req.clips, &infos)?;
    let (w, h) = target_size(&infos[0], req.height);
    let out = unique_path(Path::new(&req.out_dir), "video_editado", "mp4").to_string_lossy().into_owned();
    let args = join_args(&segs, w, h, crf(&req.quality, false), &out);
    let total: f64 = segs.iter().map(|g| g.len).sum();
    let progress = ffmpeg_progress(req.id.clone(), 0.0, Some(total));
    spawn_job(&app, req.id, "ffmpeg", args, output(Some(out)), progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(w: u32, h: u32, d: f64, audio: bool) -> MediaInfo {
        MediaInfo { duration: Some(d), width: Some(w), height: Some(h), has_video: true, has_audio: audio }
    }

    #[test]
    fn size() {
        assert_eq!(target_size(&info(1920, 1080, 1.0, true), None), (1920, 1080));
        assert_eq!(target_size(&info(1920, 1080, 1.0, true), Some(720)), (1280, 720));
        assert_eq!(target_size(&info(1081, 607, 1.0, true), None), (1080, 606));
    }

    #[test]
    fn join_two_with_silence() {
        let clips = vec![
            Clip { path: "a.mp4".into(), start: 5.0, end: Some(10.0) },
            Clip { path: "b.mp4".into(), start: 0.0, end: None },
        ];
        let segs = segments(&clips, &[info(1920, 1080, 60.0, true), info(1280, 720, 8.0, false)]).unwrap();
        assert_eq!(segs[1].len, 8.0);
        let a = join_args(&segs, 1920, 1080, "23", "out.mp4");
        let fc = &a[a.iter().position(|x| x == "-filter_complex").unwrap() + 1];
        assert!(fc.contains("[1:v]scale=1920:1080"));
        assert!(fc.contains("anullsrc")); // b.mp4 no tiene audio
        assert!(fc.ends_with("[v0][a0][v1][a1]concat=n=2:v=1:a=1[v][a]"));
    }

    #[test]
    fn bad_range() {
        let clips = vec![Clip { path: "a.mp4".into(), start: 10.0, end: Some(5.0) }];
        assert!(segments(&clips, &[info(640, 480, 60.0, true)]).is_err());
    }
}
