//! Pruebas contra el FFmpeg real (src-tauri/binaries). Se saltean si no está.
//! Generan medios sintéticos (barras de color + tono) en una carpeta temporal.
//! Correr con: cargo test real_ -- --nocapture

use crate::audio::{audio_args, AudioEditRequest};
use crate::convert::{convert_args, photo_args, ConvertRequest, PhotoRequest};
use crate::editor::{join_args, segments, target_size, Clip};
use crate::media::{parse_media_info, MediaInfo};
use crate::record::{parse_dshow_audio, record_args, RecordRequest};
use std::path::{Path, PathBuf};
use std::process::Command;

fn ffmpeg() -> Option<PathBuf> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("binaries").join("ffmpeg-x86_64-pc-windows-msvc.exe");
    p.exists().then_some(p)
}

fn run(ff: &Path, args: &[String]) -> (bool, String) {
    let o = Command::new(ff).args(args).output().expect("ffmpeg");
    (o.status.success(), String::from_utf8_lossy(&o.stderr).to_string())
}

fn gen(ff: &Path, args: &str) {
    let a: Vec<String> = args.split(' ').map(String::from).collect();
    let (ok, err) = run(ff, &a);
    assert!(ok, "no se pudo generar el medio de prueba: {err}");
}

fn info(ff: &Path, f: &Path) -> MediaInfo {
    let o = Command::new(ff).args(["-hide_banner", "-i"]).arg(f).output().unwrap();
    parse_media_info(&String::from_utf8_lossy(&o.stderr))
}

fn assert_output(ff: &Path, out: &str, (ok, err): (bool, String)) {
    assert!(ok, "FFmpeg falló para {out}:\n{}", err.lines().rev().take(8).collect::<Vec<_>>().join("\n"));
    let size = std::fs::metadata(out).map(|m| m.len()).unwrap_or(0);
    assert!(size > 0, "salida vacía: {out}");
    let _ = ff;
}

struct Fixtures {
    dir: PathBuf,
    a: String, // 640x360, 4 s, con audio
    b: String, // 320x240, 3 s, sin audio
    c: String, // wav 10 s
    d: String, // png 800x600
}

fn fixtures(ff: &Path, name: &str) -> Fixtures {
    let dir = std::env::temp_dir().join(format!("openmedia-test-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let p = |f: &str| dir.join(f).to_string_lossy().into_owned();
    let (a, b, c, d) = (p("a.mp4"), p("b.mp4"), p("c.wav"), p("d.png"));
    gen(ff, &format!("-hide_banner -y -f lavfi -i testsrc=size=640x360:rate=30:d=4 -f lavfi -i sine=f=440:d=4 -shortest -c:v libx264 -pix_fmt yuv420p -c:a aac {a}"));
    gen(ff, &format!("-hide_banner -y -f lavfi -i testsrc2=size=320x240:rate=25:d=3 -c:v libx264 -pix_fmt yuv420p {b}"));
    gen(ff, &format!("-hide_banner -y -f lavfi -i sine=f=330:d=10 {c}"));
    gen(ff, &format!("-hide_banner -y -f lavfi -i testsrc=size=800x600 -frames:v 1 {d}"));
    Fixtures { dir, a, b, c, d }
}

#[test]
fn real_convert_all_presets() {
    let Some(ff) = ffmpeg() else { return };
    let fx = fixtures(&ff, "convert");
    let out_dir = fx.dir.to_string_lossy().into_owned();
    for preset in ["mp4", "webm", "mkv", "mp3", "m4a", "ogg", "opus", "wav", "flac", "gif", "jpg"] {
        let r = ConvertRequest {
            input: fx.a.clone(),
            preset: preset.into(),
            out_dir: out_dir.clone(),
            rotate: Some("cw".into()),
            start: Some("1".into()),
            end: Some("3".into()),
            height: Some(240),
            quality: Some("low".into()),
            audio_bitrate: Some(128),
            ..Default::default()
        };
        let (args, out) = convert_args(&r).unwrap();
        let res = run(&ff, &args);
        if preset == "jpg" {
            assert!(res.0, "jpg falló: {}", res.1);
            let n = std::fs::read_dir(Path::new(&out).parent().unwrap()).unwrap().count();
            assert!(n >= 2, "jpg: se esperaban fotogramas, hubo {n}");
        } else {
            assert_output(&ff, &out, res);
        }
        if preset == "mp4" {
            // Rotado 90° y escalado a 240 de alto: 640x360 -> 360x640 -> 136x240 aprox.
            let m = info(&ff, Path::new(&out));
            assert_eq!(m.height, Some(240));
            assert!(m.width.unwrap() < 240, "debería quedar vertical: {m:?}");
            let d = m.duration.unwrap();
            assert!((d - 2.0).abs() < 0.3, "recorte 1→3 s, duró {d}");
        }
    }
}

#[test]
fn real_join_with_and_without_audio() {
    let Some(ff) = ffmpeg() else { return };
    let fx = fixtures(&ff, "join");
    let clips = vec![
        Clip { path: fx.a.clone(), start: 1.0, end: Some(3.0) },
        Clip { path: fx.b.clone(), start: 0.0, end: None },
        Clip { path: fx.a.clone(), start: 3.0, end: None },
    ];
    let infos: Vec<MediaInfo> = clips.iter().map(|c| info(&ff, Path::new(&c.path))).collect();
    assert!(!infos[1].has_audio);
    let segs = segments(&clips, &infos).unwrap();
    let (w, h) = target_size(&infos[0], Some(360));
    let out = fx.dir.join("joined.mp4").to_string_lossy().into_owned();
    assert_output(&ff, &out, run(&ff, &join_args(&segs, w, h, "28", &out)));
    let m = info(&ff, Path::new(&out));
    assert_eq!((m.width, m.height), (Some(640), Some(360)));
    assert!(m.has_audio);
    let d = m.duration.unwrap();
    assert!((d - 6.0).abs() < 0.4, "2 + 3 + 1 = 6 s, duró {d}");
}

#[test]
fn real_audio_edits() {
    let Some(ff) = ffmpeg() else { return };
    let fx = fixtures(&ff, "audio");
    let out_dir = fx.dir.to_string_lossy().into_owned();
    let cases = [("delete", 2.0, 5.0, 7.0), ("keep", 2.0, 5.0, 3.0), ("delete", 0.0, 4.0, 6.0), ("all", 0.0, 0.0, 10.0)];
    for (mode, s, e, expect) in cases {
        for format in ["mp3", "wav"] {
            let r = AudioEditRequest {
                input: fx.c.clone(),
                out_dir: out_dir.clone(),
                format: format.into(),
                bitrate: Some(128),
                duration: 10.0,
                mode: mode.into(),
                sel_start: s,
                sel_end: e,
                fade_in: 0.5,
                fade_out: 1.0,
                volume_db: -3.0,
                normalize: true,
                ..Default::default()
            };
            let (args, out, dur) = audio_args(&r).unwrap();
            assert_eq!(dur, expect);
            assert_output(&ff, &out, run(&ff, &args));
            let d = info(&ff, Path::new(&out)).duration.unwrap();
            assert!((d - expect).abs() < 0.2, "{mode} {format}: esperaba {expect} s, duró {d}");
        }
    }
}

#[test]
fn real_photos() {
    let Some(ff) = ffmpeg() else { return };
    let fx = fixtures(&ff, "photos");
    for (format, width, percent) in [("webp", Some(300), None), ("jpg", None, Some(50)), ("same", Some(100), None)] {
        let r = PhotoRequest { input: fx.d.clone(), out_dir: fx.dir.to_string_lossy().into(), format: format.into(), width, percent, quality: 80, ..Default::default() };
        let (args, out) = photo_args(&r);
        assert_output(&ff, &out, run(&ff, &args));
        let m = info(&ff, Path::new(&out));
        assert_eq!(m.width, Some(width.unwrap_or(400)), "{format}");
    }
}

#[test]
fn real_record_two_seconds() {
    let Some(ff) = ffmpeg() else { return };
    let dir = std::env::temp_dir().join("openmedia-test-record");
    let _ = std::fs::create_dir_all(&dir);
    let out = dir.join("rec.mp4").to_string_lossy().into_owned();
    let r = RecordRequest { id: "t".into(), out_dir: dir.to_string_lossy().into(), name: "rec".into(), fps: 15, audio_device: None, cursor: true };
    let mut args = record_args(&r, &out);
    let last = args.pop().unwrap();
    args.extend(["-t".into(), "2".into(), last]);
    assert_output(&ff, &out, run(&ff, &args));
    let m = info(&ff, Path::new(&out));
    assert!(m.has_video && m.width.unwrap().is_multiple_of(2));
    let _ = std::fs::remove_file(&out); // no dejar capturas de pantalla
    let o = Command::new(&ff).args(["-hide_banner", "-list_devices", "true", "-f", "dshow", "-i", "dummy"]).output().unwrap();
    let mics = parse_dshow_audio(&String::from_utf8_lossy(&o.stderr));
    println!("micrófonos: {mics:?}");
    // Con el último micrófono de la lista (suele ser el integrado), si hay alguno.
    if let Some(mic) = mics.last() {
        let r = RecordRequest { audio_device: Some(mic.clone()), ..r };
        let mut args = record_args(&r, &out);
        let last = args.pop().unwrap();
        args.extend(["-t".into(), "2".into(), last]);
        assert_output(&ff, &out, run(&ff, &args));
        assert!(info(&ff, Path::new(&out)).has_audio, "la grabación con micrófono no tiene audio");
        let _ = std::fs::remove_file(&out);
    }
}
