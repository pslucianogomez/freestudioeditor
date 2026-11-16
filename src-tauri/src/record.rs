//! Grabación de pantalla con FFmpeg (gdigrab) y micrófono opcional (dshow).

use crate::jobs::{emit, ev, ffmpeg_path, hidden_command, output, spawn_job, unique_path, Jobs};
use serde::Deserialize;
use std::path::Path;
use tauri::{AppHandle, Manager};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordRequest {
    pub id: String,
    pub out_dir: String,
    /// Nombre del archivo sin extensión
    pub name: String,
    pub fps: u32,
    /// Nombre del dispositivo de audio (ver list_audio_devices)
    pub audio_device: Option<String>,
    pub cursor: bool,
}

pub fn record_args(r: &RecordRequest, out: &str) -> Vec<String> {
    let s = |v: &str| v.to_string();
    let mut a = vec![s("-hide_banner"), s("-nostats"), s("-progress"), s("pipe:1"), s("-y")];
    a.extend([s("-f"), s("gdigrab"), s("-framerate"), r.fps.clamp(5, 60).to_string()]);
    a.extend([s("-draw_mouse"), s(if r.cursor { "1" } else { "0" }), s("-i"), s("desktop")]);
    let audio = r.audio_device.as_ref().filter(|d| !d.is_empty());
    if let Some(dev) = audio {
        // rtbufsize: búfer del micrófono para que no se pierda audio si la PC se atrasa.
        a.extend([s("-rtbufsize"), s("100M"), s("-f"), s("dshow"), s("-i"), format!("audio={dev}")]);
    }
    // x264 necesita ancho y alto pares.
    a.extend([s("-vf"), s("scale=trunc(iw/2)*2:trunc(ih/2)*2")]);
    a.extend([s("-c:v"), s("libx264"), s("-preset"), s("ultrafast"), s("-crf"), s("23"), s("-pix_fmt"), s("yuv420p")]);
    if audio.is_some() {
        a.extend([s("-c:a"), s("aac"), s("-b:a"), s("160k")]);
    }
    // MP4 fragmentado: si algo se corta, el archivo igual se puede reproducir.
    a.extend([s("-movflags"), s("+frag_keyframe+empty_moov")]);
    a.push(out.to_string());
    a
}

/// Lee la lista de dispositivos de DirectShow y devuelve los de audio.
pub fn parse_dshow_audio(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|l| l.contains("(audio)") && !l.contains("Alternative name"))
        .filter_map(|l| {
            let start = l.find('"')? + 1;
            let end = start + l[start..].find('"')?;
            Some(l[start..end].to_string())
        })
        .collect()
}

#[tauri::command]
pub async fn list_audio_devices() -> Vec<String> {
    tauri::async_runtime::spawn_blocking(|| {
        hidden_command(&ffmpeg_path())
            .args(["-hide_banner", "-list_devices", "true", "-f", "dshow", "-i", "dummy"])
            .output()
            .map(|o| parse_dshow_audio(&String::from_utf8_lossy(&o.stderr)))
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
pub fn record_screen(app: AppHandle, req: RecordRequest) -> Result<(), String> {
    crate::jobs::ensure_writable(&req.out_dir)?;
    let out = unique_path(Path::new(&req.out_dir), &req.name, "mp4").to_string_lossy().into_owned();
    let args = record_args(&req, &out);
    let jid = req.id.clone();
    spawn_job(&app, req.id, "ffmpeg", args, output(Some(out)), move |app, line, is_err| {
        if is_err {
            return;
        }
        if let Some(v) = line.strip_prefix("out_time_us=") {
            if let Ok(us) = v.trim().parse::<f64>() {
                let mut e = ev(&jid, "progress");
                e.elapsed = Some(us / 1_000_000.0);
                emit(app, e);
            }
        }
    })
}

/// Detiene la grabación prolijamente: FFmpeg termina al recibir "q".
#[tauri::command]
pub fn stop_recording(app: AppHandle, id: String) -> Result<(), String> {
    let jobs = app.state::<Jobs>();
    let mut map = jobs.0.lock().unwrap();
    match map.get_mut(&id) {
        Some(child) => child.write(b"q\n").map_err(|e| e.to_string()),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devices() {
        let s = r#"[dshow @ 0000] "Integrated Camera" (video)
[dshow @ 0000]   Alternative name "@device_pnp_\\?\usb"
[dshow @ 0000] "Microphone (Realtek(R) Audio)" (audio)
[dshow @ 0000]   Alternative name "@device_cm_{33D9A762}\wave_{ABC}"
[dshow @ 0000] "Stereo Mix (Realtek(R) Audio)" (audio)"#;
        assert_eq!(parse_dshow_audio(s), vec!["Microphone (Realtek(R) Audio)", "Stereo Mix (Realtek(R) Audio)"]);
    }

    #[test]
    fn args_with_mic() {
        let r = RecordRequest { id: "1".into(), out_dir: ".".into(), name: "g".into(), fps: 30, audio_device: Some("Mic (X)".into()), cursor: true };
        let a = record_args(&r, "g.mp4");
        assert!(a.contains(&"audio=Mic (X)".to_string()));
        assert!(a.contains(&"aac".to_string()));
        assert_eq!(a.last().unwrap(), "g.mp4");
    }
}
