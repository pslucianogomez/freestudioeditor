# Arquitectura

```
┌──────────── Interfaz (React, src/) ────────────┐
│ Descargar · Convertir · Editor de video ·      │
│ Editor de audio · Grabar · Fotos · Ajustes     │
└──────────── invoke() ▲ eventos "job" ──────────┘
               │       │
┌──────────── Núcleo (Rust, src-tauri/src/) ─────┐
│ jobs.rs     lanzar procesos, progreso, cancelar│
│ download.rs descargas y vista previa (yt-dlp)  │
│ convert.rs  conversión y fotos                 │
│ media.rs    datos de archivo, forma de onda    │
│ editor.rs   unir/cortar video                  │
│ audio.rs    editor de audio                    │
│ record.rs   grabación de pantalla              │
└──────┬───────────────────────────┬─────────────┘
       ▼                           ▼
   yt-dlp.exe                  ffmpeg.exe
 (descarga, mezcla con ffmpeg)  (conversión)
```

- **Sidecars**: `yt-dlp` y `ffmpeg` se incluyen en el instalador (`bundle.externalBin`) y quedan junto al `.exe`. Se bajan con `npm run fetch-binaries`.
- **Progreso yt-dlp**: `--progress-template` con prefijo `OMPROG:`; título con `--print video:OMTITLE:`; archivo final con `--print after_move:OMFILE:`.
- **Progreso FFmpeg**: `-progress pipe:1` (`out_time_us`) dividido por la duración leída de `Duration:` en stderr.
- **Cancelación**: tabla `Jobs` (id → proceso); `cancel_job` lo saca de la tabla y lo mata, así el final se informa como `cancelled`.
- **Ajustes**: `localStorage` del webview (carpeta de salida, navegador para cookies).
- **Sin servidor**: todo corre en la PC del usuario, con su conexión.
- **Vista previa en la app**: los videos/audios/fotos locales se muestran con el protocolo `asset:` de Tauri (`convertFileSrc`).
- **Arrastrar y soltar**: evento nativo de Tauri (`onDragDropEvent`) en `App.tsx`, que decide a qué herramienta mandar los archivos.
- **Grabación**: FFmpeg `gdigrab` (pantalla) + `dshow` (micrófono); se detiene mandando `q` por stdin para cerrar el MP4 prolijamente. MP4 fragmentado por si se corta.
- **Pruebas**: `cargo test` corre las unitarias; `real_ffmpeg.rs` genera medios sintéticos y prueba cada herramienta contra el FFmpeg real (se saltean si no está descargado).
