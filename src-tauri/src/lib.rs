//! Núcleo de OpenMedia. Cada módulo es una herramienta:
//!
//! - `jobs`     lanzar procesos, progreso, cancelar (compartido)
//! - `download` descargas y vista previa (yt-dlp)
//! - `convert`  conversión de video/audio/imagen y fotos (FFmpeg)
//! - `media`    datos de un archivo y forma de onda
//! - `editor`   unir y cortar tramos de video
//! - `audio`    editor de audio
//! - `record`   grabación de pantalla

mod audio;
mod convert;
mod download;
mod editor;
mod jobs;
mod media;
mod record;

#[cfg(test)]
mod real_ffmpeg;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(jobs::Jobs::default())
        .invoke_handler(tauri::generate_handler![
            jobs::cancel_job,
            jobs::versions,
            jobs::update_ytdlp,
            download::download,
            download::probe,
            convert::convert,
            convert::photo,
            media::media_info,
            media::file_info,
            media::waveform,
            editor::join_clips,
            audio::audio_edit,
            record::list_audio_devices,
            record::record_screen,
            record::stop_recording,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar la app");
}
