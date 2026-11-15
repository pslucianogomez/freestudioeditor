//! Infraestructura compartida: lanzar procesos (yt-dlp / FFmpeg), seguir su
//! salida, emitir eventos de progreso y cancelar o detener trabajos.

use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

/// Procesos en curso, por id de trabajo.
#[derive(Default)]
pub struct Jobs(pub Mutex<HashMap<String, CommandChild>>);

#[derive(Clone, Serialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JobEvent {
    pub id: String,
    /// progress | log | title | done | error | cancelled
    pub kind: String,
    pub percent: Option<f64>,
    pub speed: Option<String>,
    pub eta: Option<String>,
    /// Segundos transcurridos (grabación de pantalla)
    pub elapsed: Option<f64>,
    pub message: Option<String>,
    pub output: Option<String>,
}

pub fn emit(app: &AppHandle, ev: JobEvent) {
    let _ = app.emit("job", ev);
}

pub fn ev(id: &str, kind: &str) -> JobEvent {
    JobEvent { id: id.to_string(), kind: kind.to_string(), ..Default::default() }
}

/// Ruta del archivo de salida que se informa al terminar.
pub type Output = Arc<Mutex<Option<String>>>;

pub fn output(initial: Option<String>) -> Output {
    Arc::new(Mutex::new(initial))
}

/// Carpeta donde Tauri deja los sidecars (junto al ejecutable).
pub fn bin_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_default()
}

pub fn ffmpeg_path() -> PathBuf {
    bin_dir().join(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" })
}

/// Proceso sin ventana de consola (para consultas rápidas a FFmpeg).
pub fn hidden_command(path: &Path) -> std::process::Command {
    #[allow(unused_mut)]
    let mut c = std::process::Command::new(path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    c
}

/// Nombre de salida que no pisa archivos existentes.
pub fn unique_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let mut p = dir.join(format!("{stem}.{ext}"));
    let mut n = 1;
    while p.exists() {
        p = dir.join(format!("{stem} ({n}).{ext}"));
        n += 1;
    }
    p
}

pub fn stem_of(path: &str) -> String {
    Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or("salida").to_string()
}

/// "00:01:23.45" o "83.45" -> segundos
pub fn parse_hms(t: &str) -> Option<f64> {
    let p: Vec<f64> = t.trim().split(':').map(|x| x.parse().ok()).collect::<Option<_>>()?;
    match p.as_slice() {
        [h, m, s] => Some(h * 3600.0 + m * 60.0 + s),
        [m, s] => Some(m * 60.0 + s),
        [s] => Some(*s),
        _ => None,
    }
}

/// Lanza un sidecar y procesa su salida en segundo plano.
/// `on_line(app, línea, es_stderr)` interpreta cada línea.
pub fn spawn_job<F>(
    app: &AppHandle,
    id: String,
    bin: &str,
    args: Vec<String>,
    output: Output,
    mut on_line: F,
) -> Result<(), String>
where
    F: FnMut(&AppHandle, &str, bool) + Send + 'static,
{
    let cmd = app
        .shell()
        .sidecar(bin)
        .map_err(|e| e.to_string())?
        .args(args)
        .env("PYTHONIOENCODING", "utf-8");
    let (mut rx, child) = cmd.spawn().map_err(|e| e.to_string())?;
    app.state::<Jobs>().0.lock().unwrap().insert(id.clone(), child);

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut tail: Vec<String> = Vec::new();
        let mut code: Option<i32> = None;
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(b) => {
                    let line = String::from_utf8_lossy(&b).trim_end().to_string();
                    on_line(&app, &line, false);
                }
                CommandEvent::Stderr(b) => {
                    let line = String::from_utf8_lossy(&b).trim_end().to_string();
                    on_line(&app, &line, true);
                    if !line.is_empty() {
                        tail.push(line);
                        if tail.len() > 8 {
                            tail.remove(0);
                        }
                    }
                }
                CommandEvent::Error(e) => tail.push(e),
                CommandEvent::Terminated(p) => code = p.code,
                _ => {}
            }
        }
        // Si ya no está en la tabla, lo canceló el usuario.
        let was_running = app.state::<Jobs>().0.lock().unwrap().remove(&id).is_some();
        let mut done = ev(&id, "cancelled");
        if was_running {
            if code == Some(0) {
                done.kind = "done".into();
                done.percent = Some(100.0);
                done.output = output.lock().unwrap().clone();
            } else {
                done.kind = "error".into();
                done.message = Some(tail.join("\n"));
            }
        }
        emit(&app, done);
    });
    Ok(())
}

/// Interpreta el progreso de FFmpeg (`-progress pipe:1`).
/// `total`: duración a procesar; si es None se lee de "Duration:" menos `start`.
pub fn ffmpeg_progress(id: String, start: f64, total: Option<f64>) -> impl FnMut(&AppHandle, &str, bool) + Send + 'static {
    let mut duration = total;
    let mut speed: Option<String> = None;
    move |app, line, is_err| {
        if is_err {
            if duration.is_none() {
                if let Some(rest) = line.trim().strip_prefix("Duration: ") {
                    duration = rest.split(',').next().and_then(parse_hms).map(|d| (d - start).max(0.1));
                }
            }
            return;
        }
        if let Some(v) = line.strip_prefix("speed=") {
            speed = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("out_time_us=") {
            if let (Ok(us), Some(d)) = (v.trim().parse::<f64>(), duration) {
                let mut e = ev(&id, "progress");
                e.percent = Some((us / 1_000_000.0 / d * 100.0).clamp(0.0, 99.9));
                e.speed = speed.clone();
                emit(app, e);
            }
        }
    }
}

/// Argumentos comunes al inicio de todo comando FFmpeg con progreso.
pub fn ffmpeg_base() -> Vec<String> {
    ["-hide_banner", "-nostats", "-progress", "pipe:1", "-y"].map(String::from).to_vec()
}

#[tauri::command]
pub fn cancel_job(app: AppHandle, id: String) -> Result<(), String> {
    let child = app.state::<Jobs>().0.lock().unwrap().remove(&id);
    match child {
        Some(c) => c.kill().map_err(|e| e.to_string()),
        None => Ok(()),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Versions {
    yt_dlp: String,
    ffmpeg: String,
}

pub async fn run_capture(app: &AppHandle, bin: &str, args: &[&str]) -> String {
    match app.shell().sidecar(bin) {
        Ok(cmd) => match cmd.args(args).output().await {
            Ok(o) => {
                let mut s = String::from_utf8_lossy(&o.stdout).to_string();
                s.push_str(&String::from_utf8_lossy(&o.stderr));
                s.trim().to_string()
            }
            Err(e) => format!("error: {e}"),
        },
        Err(e) => format!("no encontrado: {e}"),
    }
}

#[tauri::command]
pub async fn versions(app: AppHandle) -> Versions {
    let y = run_capture(&app, "yt-dlp", &["--version"]).await;
    let f = run_capture(&app, "ffmpeg", &["-version"]).await;
    Versions {
        yt_dlp: y.lines().next().unwrap_or("").to_string(),
        ffmpeg: f.lines().next().unwrap_or("").to_string(),
    }
}

/// yt-dlp se actualiza solo (los sitios cambian seguido).
#[tauri::command]
pub async fn update_ytdlp(app: AppHandle) -> String {
    run_capture(&app, "yt-dlp", &["-U"]).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hms() {
        assert_eq!(parse_hms("00:01:23.5"), Some(83.5));
        assert_eq!(parse_hms("1:00"), Some(60.0));
        assert_eq!(parse_hms("12.5"), Some(12.5));
        assert_eq!(parse_hms("N/A"), None);
    }
}
