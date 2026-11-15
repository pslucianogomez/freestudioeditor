# OpenMedia

**Descargá, convertí y editá video y audio. Libre, gratis, sin marcas de agua ni límites.**
App de escritorio para Windows (Mac y Linux en el roadmap), construida sobre [yt-dlp](https://github.com/yt-dlp/yt-dlp) y [FFmpeg](https://ffmpeg.org).

> *OpenMedia es un nombre provisorio.*

## Qué hace hoy

- **Descargar** video o solo audio desde YouTube y [más de mil sitios](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md): videos sueltos, playlists y canales, hasta 4K. Vista previa con miniatura, duración y las calidades reales de cada video con su tamaño.
- **Subtítulos**: elegir idiomas (del autor o automáticos), SRT o VTT, incrustarlos en el video o bajar solo los subtítulos.
- **Convertir** a MP4, WebM, MKV, MP3, M4A, OGG, OPUS, WAV, FLAC, GIF animado y fotogramas JPG; resolución, calidad, bitrate y atajos por dispositivo (WhatsApp, celular, TV, web). Rotar, espejar y recortar.
- **Editor de video**: unir varios videos, recortar cada tramo, dividir para sacar partes del medio, reordenar, con reproductor integrado.
- **Editor de audio**: forma de onda, seleccionar y quedarse con un tramo o borrarlo, fundidos, volumen y normalización.
- **Grabar pantalla** en MP4, con micrófono opcional.
- **Fotos**: cambiar tamaño y formato (JPG, PNG, WebP) por lote.
- Arrastrar y soltar archivos en cualquier parte de la ventana; botón "Pegar" para enlaces.
- Cola de trabajos con progreso, cancelación y "abrir carpeta". Actualizar yt-dlp desde la app.

Lo que viene está en [docs/ROADMAP.md](docs/ROADMAP.md).

## ¿Necesito iniciar sesión?

No. Los videos públicos se descargan sin cuenta. Solo hace falta para videos privados, con restricción de edad o de miembros, o cuando el sitio pide "confirmar que no sos un bot". En ese caso, en **Ajustes → Cookies del navegador** la app usa la sesión que ya tenés abierta en tu navegador. **OpenMedia nunca pide ni guarda contraseñas.**

## Uso responsable

Esta herramienta es para descargar contenido propio, de dominio público, con licencia libre (por ejemplo Creative Commons) o con permiso de su autor. Respetá los derechos de autor y los términos de cada sitio. Los autores del proyecto no promueven ni se responsabilizan por usos que infrinjan derechos de terceros.

## Desarrollo

Requisitos: [Node.js 22+](https://nodejs.org), [Rust](https://rustup.rs) y los [prerequisitos de Tauri para Windows](https://v2.tauri.app/start/prerequisites/) (Build Tools de Visual Studio con C++ y WebView2).

```bash
npm install
npm run fetch-binaries   # baja yt-dlp.exe y ffmpeg.exe a src-tauri/binaries
npx tauri icon app-icon.svg   # genera los íconos (una sola vez)
npm run tauri dev        # app en modo desarrollo
npm run tauri build      # instalador en src-tauri/target/release/bundle/nsis
```

Estructura:

```
src/             interfaz (React + TypeScript)
src-tauri/       núcleo (Rust): lanza yt-dlp/FFmpeg y reporta progreso
scripts/         utilidades de build
docs/            arquitectura y roadmap
```

Las versiones se publican solas: al subir un tag `v*`, GitHub Actions compila el instalador y crea un borrador en *Releases*.

## Contribuir

¡Bienvenido! Leé [CONTRIBUTING.md](CONTRIBUTING.md). Buscá issues con la etiqueta `good first issue`.

## Licencia

[GPL-3.0-or-later](LICENSE). Incluye binarios de yt-dlp ([Unlicense](https://github.com/yt-dlp/yt-dlp/blob/master/LICENSE)) y FFmpeg (build GPL de [BtbN](https://github.com/BtbN/FFmpeg-Builds)); ver [docs/THIRD_PARTY.md](docs/THIRD_PARTY.md).

---

### English

OpenMedia is a free, open-source desktop app to download, convert and lightly edit video and audio, built on yt-dlp and FFmpeg. No watermarks, no limits, no account needed for public videos. See the Spanish sections above for build steps (`npm install`, `npm run fetch-binaries`, `npm run tauri dev`). Contributions in English or Spanish are welcome.
