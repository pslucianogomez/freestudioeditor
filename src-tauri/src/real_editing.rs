//! Pruebas contra el FFmpeg real de las herramientas de edición (efectos de video,
//! editor de fotos, audio y transiciones). Se saltean si FFmpeg no está descargado.
//! Correr con: cargo test real_ -- --nocapture

use crate::audio::{audio_args, join_audio_args, AudioEditRequest, JoinAudioRequest};
use crate::editor::{check_transition, join_args, segments, target_size, total_duration, Clip};
use crate::effects::{video_fx_args, VideoFxRequest};
use crate::media::{parse_media_info, MediaInfo};
use crate::photo_edit::{default_font, photo_edit_args, LogoMark, PhotoEditRequest, TextMark};
use std::path::{Path, PathBuf};
use std::process::Command;

fn ffmpeg() -> Option<PathBuf> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("binaries").join("ffmpeg-x86_64-pc-windows-msvc.exe");
    p.exists().then_some(p)
}

fn run(ff: &Path, args: &[String]) -> Result<(), String> {
    let o = Command::new(ff).args(args).output().expect("ffmpeg");
    if o.status.success() {
        Ok(())
    } else {
        let e = String::from_utf8_lossy(&o.stderr);
        Err(e.lines().rev().take(10).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n"))
    }
}

fn gen(ff: &Path, args: &[&str]) {
    let a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    run(ff, &a).unwrap_or_else(|e| panic!("no se pudo generar el medio de prueba: {e}"));
}

fn info(ff: &Path, f: &str) -> MediaInfo {
    let o = Command::new(ff).args(["-hide_banner", "-i", f]).output().unwrap();
    parse_media_info(&String::from_utf8_lossy(&o.stderr))
}

fn near(a: f64, b: f64, tol: f64, what: &str) {
    assert!((a - b).abs() <= tol, "{what}: esperaba ~{b}, dio {a}");
}

/// Carpeta de prueba con espacios y comilla en el nombre, a propósito.
fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fes test o'k {name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn p(d: &Path, f: &str) -> String {
    d.join(f).to_string_lossy().into_owned()
}

/// a.mp4 640x360 4 s con audio · b.mp4 320x240 3 s sin audio · m.wav 10 s · d.png 800x600
fn media(ff: &Path, d: &Path) -> (String, String, String, String) {
    let (a, b, m, img) = (p(d, "a.mp4"), p(d, "b.mp4"), p(d, "m.wav"), p(d, "d.png"));
    gen(ff, &["-hide_banner", "-y", "-f", "lavfi", "-i", "testsrc=size=640x360:rate=30:d=4", "-f", "lavfi", "-i", "sine=f=440:d=4", "-shortest", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", &a]);
    gen(ff, &["-hide_banner", "-y", "-f", "lavfi", "-i", "testsrc2=size=320x240:rate=25:d=3", "-c:v", "libx264", "-pix_fmt", "yuv420p", &b]);
    gen(ff, &["-hide_banner", "-y", "-f", "lavfi", "-i", "sine=f=330:d=10", &m]);
    gen(ff, &["-hide_banner", "-y", "-f", "lavfi", "-i", "testsrc=size=800x600", "-frames:v", "1", &img]);
    (a, b, m, img)
}

fn fx_base(input: &str) -> VideoFxRequest {
    VideoFxRequest { input: input.into(), speed: 1.0, audio: "keep".into(), contrast: 1.0, saturation: 1.0, quality: Some("low".into()), ..Default::default() }
}

#[test]
fn real_video_effects() {
    let Some(ff) = ffmpeg() else { return };
    let d = dir("fx");
    let (a, b, music, _) = media(&ff, &d);
    let srt = p(&d, "subs: prueba.srt");
    std::fs::write(&srt, "1\n00:00:00,500 --> 00:00:03,000\nHola: 100% de prueba, ¿sí?\n").unwrap();

    struct Case {
        name: &'static str,
        input: String,
        edit: Box<dyn Fn(&mut VideoFxRequest)>,
        subs: bool,
        dur: f64,
        size: (u32, u32),
        audio: bool,
    }
    let m2 = music.clone();
    let m3 = music.clone();
    let cases = [
        Case { name: "x2 con audio", input: a.clone(), edit: Box::new(|r| r.speed = 2.0), subs: false, dur: 2.0, size: (640, 360), audio: true },
        Case { name: "lento sin audio", input: a.clone(), edit: Box::new(|r| { r.speed = 0.5; r.audio = "mute".into(); }), subs: false, dur: 8.0, size: (640, 360), audio: false },
        Case { name: "vertical desenfocado", input: a.clone(), edit: Box::new(|r| { r.aspect = Some("9:16".into()); r.aspect_mode = Some("blur".into()); }), subs: false, dur: 4.0, size: (360, 640), audio: true },
        Case { name: "cuadrado con bandas", input: a.clone(), edit: Box::new(|r| { r.aspect = Some("1:1".into()); r.aspect_mode = Some("pad".into()); }), subs: false, dur: 4.0, size: (640, 640), audio: true },
        Case { name: "vertical recortado", input: a.clone(), edit: Box::new(|r| { r.aspect = Some("9:16".into()); r.aspect_mode = Some("crop".into()); }), subs: false, dur: 4.0, size: (202, 360), audio: true },
        Case { name: "subtítulos + color + estabilizar", input: a.clone(), edit: Box::new(|r| { r.brightness = 0.1; r.contrast = 1.2; r.saturation = 0.0; r.stabilize = true; }), subs: true, dur: 4.0, size: (640, 360), audio: true },
        Case { name: "reemplazar audio", input: a.clone(), edit: Box::new(move |r| { r.audio = "replace".into(); r.audio_file = Some(m2.clone()); }), subs: false, dur: 4.0, size: (640, 360), audio: true },
        Case { name: "mezclar música en video sin audio", input: b.clone(), edit: Box::new(move |r| { r.audio = "mix".into(); r.audio_file = Some(m3.clone()); r.music_volume = Some(0.5); }), subs: false, dur: 3.0, size: (320, 240), audio: true },
    ];

    for (i, c) in cases.iter().enumerate() {
        let mut r = fx_base(&c.input);
        (c.edit)(&mut r);
        let out = p(&d, &format!("fx{i}.mp4"));
        let inf = info(&ff, &c.input);
        let args = video_fx_args(&r, &inf, c.subs.then_some(srt.as_str()), &out).unwrap();
        run(&ff, &args).unwrap_or_else(|e| panic!("{}: FFmpeg falló\n{e}", c.name));
        let m = info(&ff, &out);
        near(m.duration.unwrap(), c.dur, 0.25, c.name);
        assert_eq!((m.width.unwrap(), m.height.unwrap()), c.size, "{}", c.name);
        assert_eq!(m.has_audio, c.audio, "{}: audio", c.name);
    }
}

#[test]
fn real_photo_editor() {
    let Some(ff) = ffmpeg() else { return };
    let d = dir("photo");
    let (_, _, _, img) = media(&ff, &d);
    let text_file = p(&d, "texto.txt");
    std::fs::write(&text_file, "© Hola: 100% d'ejemplo").unwrap();
    let font = default_font().map(|f| f.to_string_lossy().into_owned());
    assert!(font.is_some(), "no hay fuentes en Windows\\Fonts");

    let mut r = PhotoEditRequest {
        input: img.clone(),
        format: "jpg".into(),
        quality: 85,
        crop: Some([100, 50, 400, 300]),
        rotate: Some("cw".into()),
        flip_h: true,
        brightness: 0.05,
        contrast: 1.1,
        saturation: 1.3,
        sharpen: 1.0,
        text: Some(TextMark { text: "x".into(), size_pct: 6.0, position: "br".into(), opacity: 0.8, color: "#ffcc00".into() }),
        logo: Some(LogoMark { path: img.clone(), width_pct: 30.0, position: "tl".into(), opacity: 0.6 }),
        ..Default::default()
    };
    for (ext, max_w, size) in [("jpg", None, (300, 400)), ("png", None, (300, 400)), ("webp", None, (300, 400)), ("jpg", Some(200), (200, 266))] {
        let out = p(&d, &format!("foto_{}_{}.{ext}", max_w.unwrap_or(0), ext));
        let args = photo_edit_args(&r, (800, 600), Some(&text_file), font.as_deref(), &out, max_w).unwrap();
        run(&ff, &args).unwrap_or_else(|e| panic!("foto {ext}: {e}"));
        let m = info(&ff, &out);
        assert_eq!((m.width.unwrap(), m.height.unwrap()), size, "foto {ext} {max_w:?}");
    }
    // Sin ninguna edición también tiene que funcionar.
    r = PhotoEditRequest { input: img, format: "png".into(), quality: 90, contrast: 1.0, saturation: 1.0, ..Default::default() };
    let out = p(&d, "sin_cambios.png");
    run(&ff, &photo_edit_args(&r, (800, 600), None, None, &out, None).unwrap()).unwrap();
    assert_eq!(info(&ff, &out).width, Some(800));
}

#[test]
fn real_audio_effects_and_join() {
    let Some(ff) = ffmpeg() else { return };
    let d = dir("audio");
    let (a, _, music, _) = media(&ff, &d);

    // Ruido + velocidad + tono
    let r = AudioEditRequest {
        input: music.clone(),
        out_dir: d.to_string_lossy().into(),
        format: "mp3".into(),
        bitrate: Some(128),
        duration: 10.0,
        mode: "all".into(),
        denoise: "strong".into(),
        speed: 1.5,
        pitch: -3.0,
        fade_out: 1.0,
        ..Default::default()
    };
    let (args, out, dur) = audio_args(&r).unwrap();
    near(dur, 10.0 / 1.5, 0.01, "duración calculada");
    run(&ff, &args).unwrap_or_else(|e| panic!("audio fx: {e}"));
    near(info(&ff, &out).duration.unwrap(), 10.0 / 1.5, 0.2, "audio fx");

    // Unir con fundido cruzado y sin fundido (mezclando un video y un wav)
    // 4 s + 10 s, menos lo que se superponen en el fundido
    for (cf, expect) in [(1.0, 13.0), (0.0, 14.0)] {
        let jr = JoinAudioRequest { inputs: vec![a.clone(), music.clone()], format: "m4a".into(), crossfade: cf, ..Default::default() };
        let out = p(&d, &format!("unidos_{cf}.m4a"));
        let (args, total) = join_audio_args(&jr, &[4.0, 10.0], &out).unwrap();
        near(total, 14.0 - cf, 0.01, "total calculado");
        run(&ff, &args).unwrap_or_else(|e| panic!("unir audios cf={cf}: {e}"));
        let got = info(&ff, &out);
        assert!(!got.has_video);
        near(got.duration.unwrap(), expect, 0.25, &format!("unir cf={cf}"));
    }
}

#[test]
fn real_video_transitions() {
    let Some(ff) = ffmpeg() else { return };
    let d = dir("trans");
    let (a, b, _, _) = media(&ff, &d);
    let clips = vec![
        Clip { path: a.clone(), start: 0.0, end: None },
        Clip { path: b.clone(), start: 0.0, end: None }, // otra resolución y sin audio
        Clip { path: a.clone(), start: 1.0, end: None },
    ];
    let infos: Vec<MediaInfo> = clips.iter().map(|c| info(&ff, &c.path)).collect();
    let segs = segments(&clips, &infos).unwrap();
    for kind in ["fade", "dissolve", "slideleft"] {
        let t = Some((kind, 0.8));
        check_transition(&segs, t).unwrap();
        let (w, h) = target_size(&infos[0], None);
        let out = p(&d, &format!("trans_{kind}.mp4"));
        run(&ff, &join_args(&segs, w, h, "28", &out, t)).unwrap_or_else(|e| panic!("transición {kind}: {e}"));
        let m = info(&ff, &out);
        // 4 + 3 + 3 - 2 x 0.8 = 8.4
        near(m.duration.unwrap(), total_duration(&segs, t), 0.25, kind);
        assert!(m.has_audio);
    }
}
