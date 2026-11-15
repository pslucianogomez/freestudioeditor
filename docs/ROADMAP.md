# Roadmap

Estado: ✅ hecho · ⬜ pendiente. Las prioridades se discuten en issues.

## v0.1 — base
- ✅ Descarga de video/audio (yt-dlp), playlists, canales, hasta 4K
- ✅ Conversión a 11 formatos, rotar / espejar / recortar
- ✅ Cola con progreso, cancelar, abrir carpeta
- ✅ Cookies opcionales del navegador · actualizar yt-dlp desde la app
- ✅ Instalador Windows automático (GitHub Actions)

## v0.2 — pulido y edición
- ✅ Vista previa del enlace (título, miniatura, duración, calidades reales con tamaño)
- ✅ Arrastrar y soltar archivos · botón Pegar
- ✅ Subtítulos: idiomas, automáticos, SRT/VTT, incrustar, solo subtítulos
- ✅ Conversión: resolución, calidad, bitrate, atajos por dispositivo, opciones de GIF
- ✅ Editor de video: unir, recortar, dividir, reordenar, reproductor
- ✅ Editor de audio: forma de onda, recortar / borrar tramo, fundidos, volumen, normalizar
- ✅ Grabador de pantalla con micrófono
- ✅ Fotos: tamaño y formato por lote

## v0.3 — herramientas de edición
- ✅ Efectos de video: velocidad, audio (quitar / reemplazar / mezclar), formato vertical o cuadrado, subtítulos quemados, estabilizar, color
- ✅ Editor de fotos: recortar, rotar, color, nitidez, marca de agua de texto y logo, vista previa exacta
- ✅ Audio: reducción de ruido, velocidad, tono, unir varios audios con fundido
- ✅ Transiciones entre tramos en el editor de video

## Próximo
- ⬜ Quitar el fondo de fotos (necesita un modelo de IA local)
- ⬜ Texto sobre video (títulos) y recortar el encuadre a mano
- ⬜ Aplicar las ediciones de foto a un lote entero
- ⬜ Voz a texto (subtítulos automáticos con Whisper) y texto a voz
- ⬜ Interfaz en inglés (i18n)
- ⬜ Firma de código (SignPath Foundation) para evitar avisos de SmartScreen
- ⬜ Auto-update de la app (tauri-plugin-updater)
- ⬜ Descargas en paralelo con límite configurable
- ⬜ Grabar una ventana o una región, y el audio de la PC sin "Mezcla estéreo" (WASAPI loopback)
- ⬜ Transcripción y resumen local (Whisper)
- ⬜ macOS y Linux

## Fuera de alcance
- Evadir DRM (Netflix, Spotify, DVDs con CSS, etc.)
- Cliente torrent
