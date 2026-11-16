//! Editor de fotos: recortar, rotar, espejar, brillo / contraste / saturación,
//! nitidez, marca de agua de texto o de logo. Incluye una vista previa exacta
//! generada con el mismo comando que la exportación.

use crate::filters::{even, filter_path, mark_position};
use crate::jobs::{ffmpeg_path, hidden_command, output, spawn_job, stem_of, unique_path};
use crate::media::media_info_blocking;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

#[derive(Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TextMark {
    pub text: String,
    /// Tamaño de letra como % del alto de la imagen
    pub size_pct: f64,
    /// tl | tr | bl | br | center
    pub position: String,
    /// 0–1
    pub opacity: f64,
    /// "#ffffff"
    pub color: String,
}

#[derive(Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LogoMark {
    pub path: String,
    /// Ancho del logo como % del ancho de la imagen
    pub width_pct: f64,
    pub position: String,
    pub opacity: f64,
}

#[derive(Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PhotoEditRequest {
    pub id: String,
    pub input: String,
    pub out_dir: String,
    /// same | jpg | png | webp
    pub format: String,
    pub quality: u32,
    /// Recorte en píxeles del original: [x, y, ancho, alto]
    pub crop: Option<[u32; 4]>,
    /// cw | ccw | 180
    pub rotate: Option<String>,
    pub flip_h: bool,
    pub flip_v: bool,
    pub brightness: f64,
    pub contrast: f64,
    pub saturation: f64,
    /// 0 = sin cambio, hasta 2
    pub sharpen: f64,
    pub text: Option<TextMark>,
    pub logo: Option<LogoMark>,
}

fn s(v: &str) -> String {
    v.to_string()
}

/// "#ff8800" -> "0xff8800" (formato de color de FFmpeg). Si no se entiende, blanco.
fn ffmpeg_color(c: &str) -> String {
    let h = c.trim().trim_start_matches('#');
    if h.len() == 6 && h.chars().all(|c| c.is_ascii_hexdigit()) {
        format!("0x{h}")
    } else {
        s("white")
    }
}

/// Fuente para el texto: Arial o Segoe UI, que vienen con Windows.
pub fn default_font() -> Option<PathBuf> {
    let windir = std::env::var("SystemRoot").unwrap_or_else(|_| s(r"C:\Windows"));
    ["arial.ttf", "segoeui.ttf", "tahoma.ttf"]
        .iter()
        .map(|f| Path::new(&windir).join("Fonts").join(f))
        .find(|p| p.exists())
}

/// Tamaño final de la imagen después de recortar y rotar (para ubicar el logo).
pub fn final_size(w: u32, h: u32, r: &PhotoEditRequest) -> (u32, u32) {
    let (mut fw, mut fh) = match r.crop {
        Some([_, _, cw, ch]) if cw > 0 && ch > 0 => (cw, ch),
        _ => (w, h),
    };
    if matches!(r.rotate.as_deref(), Some("cw") | Some("ccw")) {
        std::mem::swap(&mut fw, &mut fh);
    }
    (fw, fh)
}

/// Arma el comando. `text_file`: archivo con el texto de la marca (evita escapar el texto).
/// `max_width`: para la vista previa, limita el ancho final.
pub fn photo_edit_args(
    r: &PhotoEditRequest,
    size: (u32, u32),
    text_file: Option<&str>,
    font: Option<&str>,
    out: &str,
    max_width: Option<u32>,
) -> Result<Vec<String>, String> {
    let mut a = vec![s("-hide_banner"), s("-y"), s("-i"), r.input.clone()];
    let logo = r.logo.as_ref().filter(|l| !l.path.is_empty());
    if let Some(l) = logo {
        a.extend([s("-i"), l.path.clone()]);
    }

    let mut f: Vec<String> = Vec::new();
    if let Some([x, y, w, h]) = r.crop {
        if w > 0 && h > 0 {
            f.push(format!("crop={w}:{h}:{x}:{y}"));
        }
    }
    match r.rotate.as_deref() {
        Some("cw") => f.push(s("transpose=1")),
        Some("ccw") => f.push(s("transpose=2")),
        Some("180") => f.push(s("transpose=1,transpose=1")),
        _ => {}
    }
    if r.flip_h {
        f.push(s("hflip"));
    }
    if r.flip_v {
        f.push(s("vflip"));
    }
    if r.brightness.abs() > 0.001 || (r.contrast - 1.0).abs() > 0.001 || (r.saturation - 1.0).abs() > 0.001 {
        f.push(format!("eq=brightness={:.3}:contrast={:.3}:saturation={:.3}", r.brightness, r.contrast, r.saturation));
    }
    if r.sharpen > 0.001 {
        f.push(format!("unsharp=5:5:{:.2}:5:5:0", r.sharpen.min(3.0)));
    }
    if let (Some(t), Some(tf)) = (r.text.as_ref().filter(|t| !t.text.trim().is_empty()), text_file) {
        let font = font.ok_or("No se encontró una fuente en Windows\\Fonts para escribir el texto.")?;
        let size = format!("h*{:.4}", (t.size_pct / 100.0).clamp(0.005, 0.5));
        let (x, y) = mark_position(&t.position, "w", "h", "tw", "th", "h*0.03");
        f.push(format!(
            "drawtext=fontfile={}:textfile={}:expansion=none:fontsize={size}:fontcolor={}@{:.2}:shadowcolor=black@{:.2}:shadowx=2:shadowy=2:x={x}:y={y}",
            filter_path(font),
            filter_path(tf),
            ffmpeg_color(&t.color),
            t.opacity.clamp(0.0, 1.0),
            (t.opacity * 0.6).clamp(0.0, 1.0),
        ));
    }

    let base = if f.is_empty() { s("null") } else { f.join(",") };
    let preview_scale = max_width.map(|w| format!(",scale='min({w},iw)':-2")).unwrap_or_default();
    let graph = match logo {
        Some(l) => {
            let (fw, _) = final_size(size.0, size.1, r);
            let lw = even(((fw as f64) * l.width_pct.clamp(1.0, 100.0) / 100.0).round() as u32);
            let (x, y) = mark_position(&l.position, "W", "H", "w", "h", "H*0.03");
            format!(
                "[0:v]{base}[main];[1:v]scale={lw}:-1,format=rgba,colorchannelmixer=aa={:.2}[logo];[main][logo]overlay={x}:{y}:format=auto{preview_scale}[out]",
                l.opacity.clamp(0.0, 1.0)
            )
        }
        None => format!("[0:v]{base}{preview_scale}[out]"),
    };
    a.extend([s("-filter_complex"), graph, s("-map"), s("[out]"), s("-frames:v"), s("1"), s("-update"), s("1")]);

    let ext = Path::new(out).extension().and_then(|e| e.to_str()).unwrap_or("jpg").to_lowercase();
    let q = r.quality.clamp(1, 100);
    match ext.as_str() {
        "jpg" | "jpeg" => a.extend([s("-q:v"), (2 + (100 - q) * 29 / 100).to_string()]),
        "webp" => a.extend([s("-quality"), q.to_string()]),
        _ => {}
    }
    a.push(s(out));
    Ok(a)
}

fn out_ext(r: &PhotoEditRequest) -> String {
    match r.format.as_str() {
        "jpg" | "png" | "webp" => r.format.clone(),
        _ => {
            let e = Path::new(&r.input).extension().and_then(|e| e.to_str()).unwrap_or("jpg").to_lowercase();
            match e.as_str() {
                "jpeg" => s("jpg"),
                "png" | "webp" | "jpg" | "bmp" | "tif" | "tiff" => e,
                _ => s("jpg"), // HEIC, GIF, etc. se guardan como JPG
            }
        }
    }
}

fn temp_dir() -> Result<PathBuf, String> {
    let d = std::env::temp_dir().join("freeeditorstudio");
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d)
}

fn stamp() -> u128 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
}

/// Lo que el comando necesita además del pedido: tamaño original, archivo con el texto y fuente.
type Prepared = ((u32, u32), Option<String>, Option<String>);

/// Prepara todo lo que el comando necesita.
fn prepare(r: &PhotoEditRequest) -> Result<Prepared, String> {
    let info = media_info_blocking(&r.input)?;
    let size = (info.width.unwrap_or(0), info.height.unwrap_or(0));
    if let Some([x, y, w, h]) = r.crop {
        if w > 0 && (x + w > size.0 || y + h > size.1) {
            return Err("El recorte se sale de la imagen.".into());
        }
    }
    let text = r.text.as_ref().filter(|t| !t.text.trim().is_empty());
    let text_file = match text {
        Some(t) => {
            let p = temp_dir()?.join(format!("text-{}.txt", stamp()));
            std::fs::write(&p, t.text.as_bytes()).map_err(|e| e.to_string())?;
            Some(p.to_string_lossy().into_owned())
        }
        None => None,
    };
    let font = text.and_then(|_| default_font()).map(|p| p.to_string_lossy().into_owned());
    Ok((size, text_file, font))
}

/// Genera una vista previa (máx. 1200 px de ancho) y devuelve su ruta.
#[tauri::command]
pub async fn photo_preview(req: PhotoEditRequest) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (size, text_file, font) = prepare(&req)?;
        // Carpeta por "slot" (el id del pedido: "base" para la imagen original, "preview" para el
        // resultado). Dentro de cada una se borran las vistas previas anteriores.
        let slot: String = req.id.chars().filter(|c| c.is_ascii_alphanumeric()).take(20).collect();
        let dir = temp_dir()?.join(format!("preview-{slot}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let out = dir.join(format!("p-{}.jpg", stamp())).to_string_lossy().into_owned();
        let mut preview = req.clone();
        preview.quality = 90;
        let args = photo_edit_args(&preview, size, text_file.as_deref(), font.as_deref(), &out, Some(1200))?;
        let o = hidden_command(&ffmpeg_path()).args(&args).output().map_err(|e| e.to_string())?;
        if !o.status.success() {
            let err = String::from_utf8_lossy(&o.stderr);
            return Err(err.lines().rev().take(3).collect::<Vec<_>>().join("\n"));
        }
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn photo_edit(app: AppHandle, req: PhotoEditRequest) -> Result<(), String> {
    crate::jobs::ensure_writable(&req.out_dir)?;
    let r = req.clone();
    let (size, text_file, font) = tauri::async_runtime::spawn_blocking(move || prepare(&r))
        .await
        .map_err(|e| e.to_string())??;
    let out = unique_path(Path::new(&req.out_dir), &format!("{}_editada", stem_of(&req.input)), &out_ext(&req))
        .to_string_lossy()
        .into_owned();
    let args = photo_edit_args(&req, size, text_file.as_deref(), font.as_deref(), &out, None)?;
    spawn_job(&app, req.id, "ffmpeg", args, output(Some(out)), |_, _, _| {})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> PhotoEditRequest {
        PhotoEditRequest { input: "a.jpg".into(), format: "same".into(), quality: 90, contrast: 1.0, saturation: 1.0, ..Default::default() }
    }

    #[test]
    fn crop_rotate_size() {
        let mut r = base();
        r.crop = Some([10, 10, 400, 300]);
        r.rotate = Some("cw".into());
        assert_eq!(final_size(800, 600, &r), (300, 400));
        let a = photo_edit_args(&r, (800, 600), None, None, "o.jpg", None).unwrap();
        let g = &a[a.iter().position(|x| x == "-filter_complex").unwrap() + 1];
        assert_eq!(g, "[0:v]crop=400:300:10:10,transpose=1[out]");
    }

    #[test]
    fn no_edits_is_null() {
        let a = photo_edit_args(&base(), (800, 600), None, None, "o.png", None).unwrap();
        assert!(a.contains(&"[0:v]null[out]".to_string()));
    }

    #[test]
    fn logo_overlay() {
        let mut r = base();
        r.logo = Some(LogoMark { path: "logo.png".into(), width_pct: 25.0, position: "br".into(), opacity: 0.5 });
        let a = photo_edit_args(&r, (800, 600), None, None, "o.jpg", None).unwrap();
        let g = &a[a.iter().position(|x| x == "-filter_complex").unwrap() + 1];
        assert!(g.contains("[1:v]scale=200:-1"));
        assert!(g.contains("overlay=W-w-H*0.03:H-h-H*0.03"));
    }

    #[test]
    fn text_needs_font() {
        let mut r = base();
        r.text = Some(TextMark { text: "hola".into(), size_pct: 5.0, position: "br".into(), opacity: 1.0, color: "#ffffff".into() });
        assert!(photo_edit_args(&r, (800, 600), Some("t.txt"), None, "o.jpg", None).is_err());
        assert!(photo_edit_args(&r, (800, 600), Some("t.txt"), Some("C:/f.ttf"), "o.jpg", None).is_ok());
    }

    #[test]
    fn heic_saves_as_jpg() {
        let mut r = base();
        r.input = "IMG_0001.HEIC".into();
        assert_eq!(out_ext(&r), "jpg");
    }
}
