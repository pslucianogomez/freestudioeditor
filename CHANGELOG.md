# Changelog

## [0.2.1] - 2026-11-16

### Corregido
- **Descargas**: con "Playlist / canal completo" marcado y un enlace que YouTube no reconoce como lista (por ejemplo un Mix), el archivo iba a la raíz del disco (`C:\`) y fallaba con "Permission denied". Ahora queda en la carpeta elegida.
- Todas las herramientas comprueban antes de empezar que se pueda guardar en la carpeta de destino y, si no, lo avisan con un mensaje claro.

## [0.2.0] - 2026-11-15

### Agregado
- **Efectos de video** (sección nueva): velocidad de ×0.25 a ×4 manteniendo el tono, quitar el audio, reemplazarlo o mezclarlo con música de fondo, formato vertical 9:16 / cuadrado / 4:5 / 16:9 (con fondo desenfocado, bandas negras o recortando), subtítulos quemados en la imagen (.srt/.ass/.vtt), estabilizar, brillo / contraste / saturación.
- **Editor de fotos** (sección nueva): recortar con proporciones fijas, rotar y espejar, brillo / contraste / saturación / nitidez, estilos rápidos, marca de agua con texto o logo (posición, tamaño y opacidad). Vista previa exacta generada por FFmpeg. Abre HEIC/AVIF (se guardan como JPG).
- **Editor de audio**: reducción de ruido de fondo, velocidad sin cambiar el tono, cambio de tono en semitonos, y modo **Unir varios audios** con fundido cruzado opcional.
- **Editor de video**: transiciones entre tramos (fundido, fundido a negro, disolver, barrido, deslizar, círculo).
- Pruebas contra FFmpeg real para todas las herramientas nuevas (rutas con espacios y comillas, textos con `:` y `%`, videos sin audio).

### Cambiado
- "Fotos" pasa a llamarse "Fotos por lote".

## [0.1.0] - 2026-11-15

Primera versión pública.

### Descargar
- Video o solo audio (MP3, M4A, OPUS, FLAC, WAV) desde YouTube y más de mil sitios, con yt-dlp.
- Playlists y canales completos, calidades hasta 4K.
- Vista previa del enlace: miniatura, título, duración y las calidades reales con tamaño estimado.
- Subtítulos: idiomas del autor o automáticos, SRT o VTT, incrustados en el video o solo los subtítulos.
- Botón Pegar. Cookies opcionales del navegador para videos que piden sesión.

### Convertir
- MP4, WebM, MKV, MP3, M4A, OGG, OPUS, WAV, FLAC, GIF animado y fotogramas JPG.
- Resolución, calidad, bitrate, opciones de GIF y atajos por dispositivo (WhatsApp, celular, TV, web).
- Rotar, espejar y recortar por tiempo.

### Editar y grabar
- Editor de video: unir varios videos, recortar, dividir, reordenar, con reproductor.
- Editor de audio: forma de onda, quedarse con un tramo o borrarlo, fundidos, volumen y normalización.
- Grabador de pantalla en MP4 con micrófono opcional.
- Fotos: cambiar tamaño y formato (JPG, PNG, WebP) por lote, con vista de antes y después.

### General
- Arrastrar y soltar archivos en cualquier parte de la ventana.
- Cola de trabajos con progreso, cancelación, tamaño final y "abrir carpeta".
- Actualizar yt-dlp desde la app.
