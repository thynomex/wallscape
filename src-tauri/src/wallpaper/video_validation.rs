//! Explicit integration checks for a replacement runtime. No desktop tests run
//! in the regular suite. Generate synthetic media with make-libmpv-fixtures.ps1.
use super::*;

fn fixtures() -> PathBuf {
    PathBuf::from(std::env::var_os("WALLSCAPE_MPV_FIXTURES").expect("set WALLSCAPE_MPV_FIXTURES"))
}

fn fixture_player(path: &Path) -> VideoPlayer {
    let mut player = VideoPlayer::new(path).unwrap();
    player.mpv = Some(
        Mpv::with_initializer(|init| {
            init.set_option("config", "no")?;
            init.set_option("vo", "null")?;
            init.set_option("audio", "no")?;
            init.set_option("pause", true)?;
            Ok(())
        })
        .unwrap(),
    );
    player
}

#[test]
#[ignore = "requires generated synthetic media and an explicitly selected libmpv"]
fn compressed_container_codec_matrix_and_invalid_media() {
    let root = fixtures();
    let cases: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();
    for case in cases.as_array().unwrap() {
        let file = case["file"].as_str().unwrap();
        let codec = case["codec"].as_str().unwrap();
        for cap in [None, Some(30)] {
            let mut player = fixture_player(&root.join(file));
            player.set_fps_cap(cap).unwrap();
            player
                .play()
                .unwrap_or_else(|error| panic!("{file} cap={cap:?}: {error:#}"));
            let mpv = player.mpv.as_ref().unwrap();
            assert_eq!(mpv.get_property::<String>("video-format").unwrap(), codec);
            assert_eq!(mpv.get_property::<i64>("width").unwrap(), 320);
            assert_eq!(mpv.get_property::<i64>("height").unwrap(), 180);
            player.set_paused(false).unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                assert!(
                    Instant::now() < deadline,
                    "{file} failed to advance, cap={cap:?}"
                );
                let mpv = player.mpv.as_ref().unwrap();
                if mpv.get_property::<f64>("time-pos").unwrap_or(0.0) > 0.15 {
                    break;
                }
                if let Some(event) = mpv.wait_event(0.01) {
                    if let Event::EndFile(reason) = event.unwrap() {
                        panic!("{file} ended early: {reason:?}");
                    }
                }
            }
            player.stop().unwrap();
            println!("PASS {file}, decoder={codec}, cap={cap:?}");
        }
    }
    let mut invalid = fixture_player(&root.join("unsupported.mp4"));
    let error = invalid.play().unwrap_err().to_string();
    // libmpv reports an unreadable file either as a playback error event or as
    // end-of-file before video output; both runtimes in use do the former.
    assert!(
        error.contains("mpv")
            && (error.contains("before video output") || error.contains("reported playback error")),
        "{error}"
    );
    println!("PASS invalid-media error: {error}");
}

#[test]
#[ignore = "creates real wallpaper windows; run only in a controlled interactive Windows session"]
fn desktop_hardware_decode_d3d11_and_monitor_replacement() {
    use crate::wallpaper::{
        FitMode, MonitorManager, PlaybackOptions, WallpaperEngine, WallpaperTarget,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
    };

    assert_eq!(std::env::var("WALLSCAPE_DESKTOP_SMOKE").as_deref(), Ok("1"));
    let original = crate::wallpaper::get_desktop_wallpaper().unwrap();
    let sample = fixtures().join("desktop-h264.mp4");
    let monitors = MonitorManager::new().unwrap().get_monitors().to_vec();
    assert!(!monitors.is_empty());
    println!("Desktop monitors: {monitors:?}");
    let mut engine = WallpaperEngine::new().unwrap();
    engine.initialize().unwrap();

    // Engine's Drop destroys all player windows on panic as well as success.
    for cap in [None, Some(30)] {
        engine
            .set_video_wallpaper_with_options(
                sample.to_str().unwrap(),
                WallpaperTarget::All,
                PlaybackOptions {
                    fps_limit: cap,
                    ..Default::default()
                },
            )
            .unwrap();
        let until = Instant::now() + Duration::from_millis(1200);
        while Instant::now() < until {
            unsafe {
                let mut msg = MSG::default();
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            engine.service_video_loops();
            std::thread::sleep(Duration::from_millis(5));
        }
        engine.validate_runtime_outputs(cap);
        for fit in [FitMode::Fill, FitMode::Stretch, FitMode::Fit] {
            engine.set_fit_mode(fit, None).unwrap();
        }
        engine.set_paused(true).unwrap();
        engine.set_playback_speed(2.0, None).unwrap();
        engine.set_paused(false).unwrap();
    }
    engine
        .set_video_wallpaper_with_options(
            sample.to_str().unwrap(),
            WallpaperTarget::Monitor(monitors[0].id.clone()),
            PlaybackOptions::default(),
        )
        .unwrap();
    let before = engine.playback_values();
    assert!(engine
        .set_video_wallpaper_with_options(
            fixtures().join("unsupported.mp4").to_str().unwrap(),
            WallpaperTarget::All,
            PlaybackOptions::default(),
        )
        .is_err());
    assert!(engine.has_active_players());
    assert_eq!(engine.playback_values(), before);
    engine.stop().unwrap();
    assert!(!engine.has_active_players());
    drop(engine);
    assert_eq!(crate::wallpaper::get_desktop_wallpaper().unwrap(), original);
    println!("PASS desktop cleanup and unchanged original wallpaper");
}
