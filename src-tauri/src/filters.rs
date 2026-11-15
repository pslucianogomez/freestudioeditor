//! Helpers para armar filtros de FFmpeg.

/// Escapa una ruta para usarla como valor dentro de un filtro (`subtitles=`, `textfile=`...).
///
/// Dentro de un filtergraph hay dos niveles de escape: el del valor de la opción
/// (`:` separa opciones) y el del grafo (`'` delimita). En Windows además conviene
/// usar `/` en lugar de `\`. Resultado: `'C\:/Users/x/mi archivo.srt'`.
pub fn filter_path(p: &str) -> String {
    let s = p.replace('\\', "/").replace(':', "\\:");
    // Una comilla simple se cierra, se escapa y se vuelve a abrir.
    format!("'{}'", s.replace('\'', "'\\\\\\''"))
}

/// Cadena de `atempo` para un factor de velocidad. Cada `atempo` acepta 0.5–2 sin
/// perder calidad, así que valores fuera de rango se dividen en varios.
pub fn atempo_chain(speed: f64) -> Vec<String> {
    let mut s = speed.clamp(0.0625, 16.0);
    let mut out = Vec::new();
    while s < 0.5 {
        out.push("atempo=0.5".to_string());
        s /= 0.5;
    }
    while s > 2.0 {
        out.push("atempo=2.0".to_string());
        s /= 2.0;
    }
    if (s - 1.0).abs() > 1e-6 {
        out.push(format!("atempo={s:.6}"));
    }
    out
}

/// Posición de una marca (texto o logo) en coordenadas de `drawtext`/`overlay`.
/// `w`/`h`: tamaño de la imagen; `ow`/`oh`: tamaño de la marca; `m`: margen.
pub fn mark_position(pos: &str, w: &str, h: &str, ow: &str, oh: &str, m: &str) -> (String, String) {
    let left = m.to_string();
    let right = format!("{w}-{ow}-{m}");
    let top = m.to_string();
    let bottom = format!("{h}-{oh}-{m}");
    let cx = format!("({w}-{ow})/2");
    let cy = format!("({h}-{oh})/2");
    match pos {
        "tl" => (left, top),
        "tr" => (right, top),
        "bl" => (left, bottom),
        "center" => (cx, cy),
        _ => (right, bottom), // "br", por defecto
    }
}

/// Redondea hacia abajo a par (x264 y yuv420p necesitan dimensiones pares).
pub fn even(x: u32) -> u32 {
    (x - x % 2).max(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths() {
        assert_eq!(filter_path(r"C:\Users\a b\x.srt"), r"'C\:/Users/a b/x.srt'");
        assert_eq!(filter_path("C:/o'k.srt"), r"'C\:/o'\\\''k.srt'");
    }

    #[test]
    fn tempo() {
        assert!(atempo_chain(1.0).is_empty());
        assert_eq!(atempo_chain(2.0), vec!["atempo=2.000000"]);
        assert_eq!(atempo_chain(4.0), vec!["atempo=2.0", "atempo=2.000000"]);
        assert_eq!(atempo_chain(0.25), vec!["atempo=0.5", "atempo=0.500000"]);
    }

    #[test]
    fn positions() {
        assert_eq!(mark_position("br", "W", "H", "w", "h", "20"), ("W-w-20".into(), "H-h-20".into()));
        assert_eq!(mark_position("center", "W", "H", "w", "h", "0"), ("(W-w)/2".into(), "(H-h)/2".into()));
    }
}
