//! Editor de audio: recortar, borrar un tramo, fundidos, volumen y normalizar.

use crate::filters::atempo_chain;
use crate::jobs::{ffmpeg_base, ffmpeg_progress, output, spawn_job, stem_of, unique_path};
use crate::media::media_info_blocking;
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
    /// Reducción de ruido de fondo: "" | light | strong
    #[serde(default)]
    pub denoise: String,
    /// 1.0 = normal (0 se toma como 1)
    #[serde(default)]
    pub speed: f64,
    /// Cambio de tono en semitonos (-12 a 12), sin cambiar la velocidad
    #[serde(default)]
    pub pitch: f64,
}

/// Agrega reducción de ruido, velocidad y tono a la cadena `g`. Devuelve el factor de velocidad.
fn color_chain(g: &mut String, r: &AudioEditRequest) -> f64 {
    match r.denoise.as_str() {
        "light" => *g += ",afftdn=nr=12:nf=-40",
        "strong" => *g += ",afftdn=nr=30:nf=-30",
        _ => {}
    }
    let speed = if r.speed > 0.0 { r.speed } else { 1.0 };
    // Tono: se "acelera" el audio subiendo la frecuencia de muestreo (sube el tono)
    // y después atempo compensa la velocidad.
    let pitch = r.pitch.clamp(-12.0, 12.0);
    let pf = 2f64.powf(pitch / 12.0);
    if pitch.abs() > 0.01 {
        *g += &format!(",aresample=48000,asetrate={:.0},aresample=48000", 48000.0 * pf);
    }
    for t in atempo_chain(speed / pf) {
        *g += ",";
        *g += &t;
    }
    speed
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
    let speed = color_chain(&mut g, r);
    let dur = dur / speed;
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
    crate::jobs::ensure_writable(&req.out_dir)?;
    let (args, out, dur) = audio_args(&req)?;
    let progress = ffmpeg_progress(req.id.clone(), 0.0, Some(dur.max(0.1)));
    spawn_job(&app, req.id, "ffmpeg", args, output(Some(out)), progress)
}

// ---------------------------------------------------------------- unir audios

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct JoinAudioRequest {
    pub id: String,
    pub inputs: Vec<String>,
    pub out_dir: String,
    pub format: String,
    pub bitrate: Option<u32>,
    /// Segundos de fundido cruzado entre pistas (0 = pegadas)
    pub crossfade: f64,
}

/// Une las pistas en orden. `durations`: duración de cada una (para validar el fundido).
/// Devuelve los argumentos y la duración total.
pub fn join_audio_args(r: &JoinAudioRequest, durations: &[f64], out: &str) -> Result<(Vec<String>, f64), String> {
    let n = r.inputs.len();
    if n < 2 {
        return Err("Agregá al menos dos audios para unir.".into());
    }
    let cf = r.crossfade.max(0.0);
    if cf > 0.0 {
        if let Some(i) = durations.iter().position(|d| *d <= cf * 1.05) {
            return Err(format!("La pista {} dura menos que el fundido. Bajá los segundos de fundido.", i + 1));
        }
    }
    let mut a = ffmpeg_base();
    for p in &r.inputs {
        a.extend(["-i".to_string(), p.clone()]);
    }
    // Todas las pistas al mismo formato, si no concat/acrossfade fallan.
    let mut g: Vec<String> = (0..n)
        .map(|i| format!("[{i}:a]aresample=48000,aformat=sample_fmts=fltp:channel_layouts=stereo[a{i}]"))
        .collect();
    if cf > 0.0 {
        let mut cur = "[a0]".to_string();
        for i in 1..n {
            let out_label = if i == n - 1 { "[out]".to_string() } else { format!("[x{i}]") };
            g.push(format!("{cur}[a{i}]acrossfade=d={cf:.2}:c1=tri:c2=tri{out_label}"));
            cur = out_label;
        }
    } else {
        let labels: String = (0..n).map(|i| format!("[a{i}]")).collect();
        g.push(format!("{labels}concat=n={n}:v=0:a=1[out]"));
    }
    let ext = Path::new(out).extension().and_then(|e| e.to_str()).unwrap_or("mp3").to_string();
    a.extend(["-filter_complex".into(), g.join(";"), "-map".into(), "[out]".into(), "-vn".into()]);
    a.extend(codec(&ext, r.bitrate));
    a.push(out.to_string());
    let total = durations.iter().sum::<f64>() - cf * (n as f64 - 1.0);
    Ok((a, total))
}

#[tauri::command]
pub async fn join_audio(app: AppHandle, req: JoinAudioRequest) -> Result<(), String> {
    crate::jobs::ensure_writable(&req.out_dir)?;
    let inputs = req.inputs.clone();
    let durations: Vec<f64> = tauri::async_runtime::spawn_blocking(move || {
        inputs
            .iter()
            .map(|p| {
                let m = media_info_blocking(p)?;
                if !m.has_audio {
                    return Err(format!("{p} no tiene audio."));
                }
                m.duration.ok_or_else(|| format!("No se pudo leer la duración de {p}."))
            })
            .collect::<Result<Vec<_>, _>>()
    })
    .await
    .map_err(|e| e.to_string())??;
    let ext = if ["mp3", "m4a", "ogg", "opus", "wav", "flac"].contains(&req.format.as_str()) { req.format.as_str() } else { "mp3" };
    let out = unique_path(Path::new(&req.out_dir), "audios_unidos", ext).to_string_lossy().into_owned();
    let (args, total) = join_audio_args(&req, &durations, &out)?;
    let progress = ffmpeg_progress(req.id.clone(), 0.0, Some(total.max(0.1)));
    spawn_job(&app, req.id, "ffmpeg", args, output(Some(out)), progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_and_pitch() {
        let mut r = req("all", 0.0, 0.0);
        r.speed = 2.0;
        r.pitch = 12.0;
        r.denoise = "light".into();
        let (f, d) = audio_filter(&r).unwrap();
        assert_eq!(d, 50.0);
        assert!(f.contains("afftdn=nr=12"));
        assert!(f.contains("asetrate=96000"));
        // velocidad 2 / tono x2 = atempo 1 -> no hace falta atempo
        assert!(!f.contains("atempo"));
    }

    #[test]
    fn join_crossfade() {
        let r = JoinAudioRequest { inputs: vec!["a.mp3".into(), "b.mp3".into(), "c.mp3".into()], crossfade: 2.0, ..Default::default() };
        let (a, total) = join_audio_args(&r, &[10.0, 10.0, 10.0], "o.mp3").unwrap();
        assert_eq!(total, 26.0);
        let g = &a[a.iter().position(|x| x == "-filter_complex").unwrap() + 1];
        assert!(g.contains("[a0][a1]acrossfade=d=2.00:c1=tri:c2=tri[x1]"));
        assert!(g.ends_with("[x1][a2]acrossfade=d=2.00:c1=tri:c2=tri[out]"));
        assert!(join_audio_args(&r, &[10.0, 1.0, 10.0], "o.mp3").is_err());
    }

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
