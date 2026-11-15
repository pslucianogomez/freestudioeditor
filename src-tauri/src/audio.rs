//! Editor de audio: recortar, borrar un tramo, fundidos, volumen y normalizar.

use crate::jobs::{ffmpeg_base, ffmpeg_progress, output, spawn_job, stem_of, unique_path};
use serde::Deserialize;
use std::path::Path;
use tauri::AppHandle;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AudioEditRequest {
    pub id: String,
    pub input: String,
    pub out_dir: String,
    /// mp3 | m4a | ogg | opus | wav | flac
    pub format: String,
    pub bitrate: Option<u32>,
    /// Duración total del archivo en segundos
    pub duration: f64,
    /// all | keep | delete
    pub mode: String,
    pub sel_start: f64,
    pub sel_end: f64,
    pub fade_in: f64,
    pub fade_out: f64,
    pub volume_db: f64,
    pub normalize: bool,
}

/// Devuelve el filtro (`-filter_complex`) y la duración resultante.
pub fn audio_filter(r: &AudioEditRequest) -> Result<(String, f64), String> {
    let total = r.duration;
    let (s, e) = (r.sel_start.max(0.0), r.sel_end.min(total));
    let need_sel = r.mode == "keep" || r.mode == "delete";
    if need_sel && e - s < 0.05 {
        return Err("Seleccioná un tramo en la forma de onda.".into());
    }
    let (mut g, dur) = match r.mode.as_str() {
        "keep" => (format!("[0:a]atrim=start={s:.3}:end={e:.3},asetpts=PTS-STARTPTS"), e - s),
        "delete" if s < 0.01 => (format!("[0:a]atrim=start={e:.3},asetpts=PTS-STARTPTS"), total - e),
        "delete" if e > total - 0.01 => (format!("[0:a]atrim=end={s:.3},asetpts=PTS-STARTPTS"), s),
        "delete" => (
            format!(
                "[0:a]atrim=end={s:.3},asetpts=PTS-STARTPTS[p0];[0:a]atrim=start={e:.3},asetpts=PTS-STARTPTS[p1];[p0][p1]concat=n=2:v=0:a=1"
            ),
            total - (e - s),
        ),
        _ => ("[0:a]anull".to_string(), total),
    };
    if r.volume_db.abs() > 0.01 {
        g += &format!(",volume={:.1}dB", r.volume_db);
    }
    if r.normalize {
        // loudnorm sube la frecuencia de muestreo: se vuelve a 48 kHz.
        g += ",loudnorm=I=-16:TP=-1.5:LRA=11,aresample=48000";
    }
    if r.fade_in > 0.0 {
        g += &format!(",afade=t=in:st=0:d={:.2}", r.fade_in);
    }
    if r.fade_out > 0.0 {
        g += &format!(",afade=t=out:st={:.3}:d={:.2}", (dur - r.fade_out).max(0.0), r.fade_out);
    }
    g += "[out]";
    Ok((g, dur))
}

fn codec(format: &str, bitrate: Option<u32>) -> Vec<String> {
    let br = |d: u32| format!("{}k", bitrate.unwrap_or(d));
    let v: Vec<String> = match format {
        "m4a" => vec!["-c:a".into(), "aac".into(), "-b:a".into(), br(192)],
        "ogg" => vec!["-c:a".into(), "libvorbis".into(), "-b:a".into(), br(192)],
        "opus" => vec!["-c:a".into(), "libopus".into(), "-b:a".into(), br(128)],
        "wav" => vec!["-c:a".into(), "pcm_s16le".into()],
        "flac" => vec!["-c:a".into(), "flac".into()],
        _ => vec!["-c:a".into(), "libmp3lame".into(), "-b:a".into(), br(192)],
    };
    v
}

pub fn audio_args(r: &AudioEditRequest) -> Result<(Vec<String>, String, f64), String> {
    let (filter, dur) = audio_filter(r)?;
    let ext = if ["mp3", "m4a", "ogg", "opus", "wav", "flac"].contains(&r.format.as_str()) { r.format.as_str() } else { "mp3" };
    let out = unique_path(Path::new(&r.out_dir), &format!("{}_editado", stem_of(&r.input)), ext)
        .to_string_lossy()
        .into_owned();
    let mut a = ffmpeg_base();
    a.extend(["-i".into(), r.input.clone(), "-filter_complex".into(), filter, "-map".into(), "[out]".into(), "-vn".into()]);
    a.extend(codec(ext, r.bitrate));
    a.push(out.clone());
    Ok((a, out, dur))
}

#[tauri::command]
pub fn audio_edit(app: AppHandle, req: AudioEditRequest) -> Result<(), String> {
    let (args, out, dur) = audio_args(&req)?;
    let progress = ffmpeg_progress(req.id.clone(), 0.0, Some(dur.max(0.1)));
    spawn_job(&app, req.id, "ffmpeg", args, output(Some(out)), progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(mode: &str, s: f64, e: f64) -> AudioEditRequest {
        AudioEditRequest { input: "a.mp3".into(), out_dir: ".".into(), format: "mp3".into(), duration: 100.0, mode: mode.into(), sel_start: s, sel_end: e, ..Default::default() }
    }

    #[test]
    fn keep() {
        let (f, d) = audio_filter(&req("keep", 10.0, 30.0)).unwrap();
        assert_eq!(d, 20.0);
        assert_eq!(f, "[0:a]atrim=start=10.000:end=30.000,asetpts=PTS-STARTPTS[out]");
    }

    #[test]
    fn delete_middle_with_fades() {
        let mut r = req("delete", 10.0, 30.0);
        r.fade_out = 2.0;
        r.volume_db = 3.0;
        let (f, d) = audio_filter(&r).unwrap();
        assert_eq!(d, 80.0);
        assert!(f.contains("concat=n=2:v=0:a=1,volume=3.0dB"));
        assert!(f.ends_with("afade=t=out:st=78.000:d=2.00[out]"));
    }

    #[test]
    fn delete_from_start() {
        let (f, d) = audio_filter(&req("delete", 0.0, 30.0)).unwrap();
        assert_eq!(d, 70.0);
        assert!(!f.contains("concat"));
    }

    #[test]
    fn needs_selection() {
        assert!(audio_filter(&req("keep", 5.0, 5.0)).is_err());
        assert!(audio_filter(&req("all", 0.0, 0.0)).is_ok());
    }
}
