//! Consultas rápidas a FFmpeg: datos de un archivo y forma de onda.

use crate::jobs::{ffmpeg_path, hidden_command, parse_hms};
use serde::Serialize;

#[derive(Serialize, Default, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub duration: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub has_video: bool,
    pub has_audio: bool,
}

/// Lee lo que FFmpeg imprime al abrir un archivo ("Duration:", "Stream #0:0: Video: ... 1920x1080").
pub fn parse_media_info(stderr: &str) -> MediaInfo {
    let mut m = MediaInfo::default();
    for line in stderr.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("Duration: ") {
            if m.duration.is_none() {
                m.duration = rest.split(',').next().and_then(parse_hms);
            }
        }
        if !l.starts_with("Stream #") {
            continue;
        }
        // Las carátulas de los MP3 aparecen como "video (attached pic)": no cuentan.
        if l.contains("Video:") && !l.contains("attached pic") && !m.has_video {
            m.has_video = true;
            for tok in l.split([' ', ',']) {
                if let Some((w, h)) = tok.split_once('x') {
                    if let (Ok(w), Ok(h)) = (w.parse::<u32>(), h.parse::<u32>()) {
                        if (16..20000).contains(&w) && (16..20000).contains(&h) {
                            m.width = Some(w);
                            m.height = Some(h);
                            break;
                        }
                    }
                }
            }
        }
        if l.contains("Audio:") {
            m.has_audio = true;
        }
    }
    m
}

pub fn media_info_blocking(path: &str) -> Result<MediaInfo, String> {
    let out = hidden_command(&ffmpeg_path())
        .args(["-hide_banner", "-i", path])
        .output()
        .map_err(|e| format!("No se pudo ejecutar FFmpeg: {e}"))?;
    let info = parse_media_info(&String::from_utf8_lossy(&out.stderr));
    if !info.has_video && !info.has_audio {
        return Err(format!("No se reconoce el archivo como video, audio o imagen: {path}"));
    }
    Ok(info)
}

#[tauri::command]
pub async fn media_info(path: String) -> Result<MediaInfo, String> {
    tauri::async_runtime::spawn_blocking(move || media_info_blocking(&path))
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    /// Bytes
    pub size: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration: Option<f64>,
}

/// Tamaño en disco y dimensiones (para mostrar "antes → después").
#[tauri::command]
pub async fn file_info(path: String) -> Result<FileInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let size = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
        let m = media_info_blocking(&path).unwrap_or_default();
        Ok(FileInfo { size, width: m.width, height: m.height, duration: m.duration })
    })
    .await
    .map_err(|e| e.to_string())?
}

// ---------------------------------------------------------------- forma de onda

const WAVE_RATE: usize = 4000;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Waveform {
    /// Picos entre 0 y 1
    pub peaks: Vec<f32>,
    pub duration: f64,
}

/// Agrupa muestras PCM de 16 bits en `points` picos (el máximo absoluto de cada grupo).
pub fn peaks(pcm: &[u8], points: usize) -> Vec<f32> {
    let n = pcm.len() / 2;
    if n == 0 || points == 0 {
        return vec![];
    }
    let per = n.div_ceil(points).max(1);
    (0..n)
        .step_by(per)
        .map(|start| {
            let end = (start + per).min(n);
            let m = (start..end)
                .map(|i| (i16::from_le_bytes([pcm[2 * i], pcm[2 * i + 1]]) as i32).abs())
                .max()
                .unwrap_or(0);
            m as f32 / 32768.0
        })
        .collect()
}

#[tauri::command]
pub async fn waveform(path: String, points: usize) -> Result<Waveform, String> {
    tauri::async_runtime::spawn_blocking(move || {
        // Audio mono a 4 kHz, crudo: liviano y suficiente para dibujar.
        let rate = WAVE_RATE.to_string();
        let out = hidden_command(&ffmpeg_path())
            .args(["-v", "error", "-i", &path, "-vn", "-ac", "1", "-ar", &rate, "-f", "s16le", "-"])
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
        }
        Ok(Waveform { peaks: peaks(&out.stdout, points), duration: (out.stdout.len() / 2) as f64 / WAVE_RATE as f64 })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_video() {
        let s = "Input #0, mov,mp4, from 'a.mp4':\n  Duration: 00:10:34.53, start: 0.000000, bitrate: 1000 kb/s\n  Stream #0:0[0x1](und): Video: h264 (High) (avc1 / 0x31637661), yuv420p(tv, bt709, progressive), 1920x1080 [SAR 1:1 DAR 16:9], 30 fps\n  Stream #0:1[0x2](und): Audio: aac (LC) (mp4a / 0x6134706D), 48000 Hz, stereo, fltp, 128 kb/s\nAt least one output file must be specified";
        let m = parse_media_info(s);
        assert_eq!(m, MediaInfo { duration: Some(634.53), width: Some(1920), height: Some(1080), has_video: true, has_audio: true });
    }

    #[test]
    fn info_mp3_with_cover() {
        let s = "  Duration: 00:03:00.00, start: 0.025057, bitrate: 320 kb/s\n  Stream #0:0: Audio: mp3, 44100 Hz, stereo, fltp, 320 kb/s\n  Stream #0:1: Video: mjpeg (Baseline), yuvj420p, 600x600, 90k tbr (attached pic)";
        let m = parse_media_info(s);
        assert!(m.has_audio);
        assert!(!m.has_video);
        assert_eq!(m.duration, Some(180.0));
    }

    #[test]
    fn peaks_basic() {
        let samples: Vec<i16> = vec![0, 100, -16384, 50, 32767, 0];
        let pcm: Vec<u8> = samples.iter().flat_map(|x| x.to_le_bytes()).collect();
        let p = peaks(&pcm, 3);
        assert_eq!(p.len(), 3);
        assert!((p[1] - 0.5).abs() < 0.001);
        assert!(p[2] > 0.99);
    }
}
