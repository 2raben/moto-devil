//! Opt-in GPU captures. Run each test in its own process: winit creates one
//! event loop per process. Scripted controls drive the bike; no saved profiles
//! are read or written.

use super::*;
use crate::effects::RideEffects;
use bevy::render::view::window::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};
use bevy::winit::{WinitPlugin, WinitSettings};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Resource)]
struct CaptureScene {
    jet: bool,
    path: std::path::PathBuf,
    requested: bool,
    frames: usize,
    completed: Arc<AtomicBool>,
}

fn capture_scene(jet: bool) {
    let filename = if jet {
        "moto-devil-jet.png"
    } else {
        "moto-devil-trampoline-cactus.png"
    };
    let path = std::env::temp_dir().join(filename);
    let mut level = crate::levels::flight_school();
    level.start.x = if jet { 11700.0 } else { 10600.0 };
    let completed = Arc::new(AtomicBool::new(false));
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Moto Devil – automatische Grafikprüfung".into(),
                    resolution: (1280, 720).into(),
                    visible: false,
                    focused: false,
                    ..default()
                }),
                ..default()
            })
            .set(WinitPlugin {
                run_on_any_thread: true,
            }),
    )
    .add_plugins((
        SimulationPlugin,
        crate::visuals::VisualsPlugin,
        crate::camera::CameraPlugin,
        crate::hud::HudPlugin,
    ))
    .insert_resource(Levels(vec![level]))
    .insert_state(Screen::Playing)
    .insert_resource(WinitSettings::continuous())
    .insert_resource(TimeUpdateStrategy::FixedTimesteps(1))
    .insert_resource(CaptureScene {
        jet,
        path,
        requested: false,
        frames: 0,
        completed: completed.clone(),
    })
    .add_systems(Update, capture_when_ready);
    assert_eq!(app.run(), AppExit::Success);
    assert!(
        completed.load(Ordering::SeqCst),
        "GPU capture did not complete"
    );
}

fn capture_when_ready(
    mut commands: Commands,
    mut scene: ResMut<CaptureScene>,
    run: Res<Run>,
    mut controls: ResMut<Controls>,
    riders: Query<&RideEffects>,
    mut exit: MessageWriter<AppExit>,
) {
    scene.frames += 1;
    if scene.frames > 1200 || run.crash.is_some() {
        exit.write(AppExit::error());
        return;
    }
    let Some(ride) = riders.iter().next() else {
        return;
    };
    controls.throttle = if scene.jet { 0.0 } else { 1.0 };
    controls.jump_held = scene.jet;
    let ready = if scene.jet {
        ride.jet_active && ride.jet_left < 1.5
    } else {
        ride.stuck.is_some()
    };
    if ready && !scene.requested {
        scene.requested = true;
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(scene.path.clone()))
            .observe(
                |_: On<ScreenshotCaptured>,
                 scene: Res<CaptureScene>,
                 mut exit: MessageWriter<AppExit>| {
                    scene.completed.store(true, Ordering::SeqCst);
                    exit.write(AppExit::Success);
                },
            );
    }
}

#[test]
#[ignore = "requires a Linux display and GPU; writes a screenshot into the temp directory"]
fn capture_trampoline_cactus() {
    capture_scene(false);
}

#[test]
#[ignore = "requires a Linux display and GPU; run separately from the other capture"]
fn capture_jet() {
    capture_scene(true);
}

#[derive(Resource)]
struct CountryShot {
    frames: usize,
    at: usize,
    throttle: f32,
    path: std::path::PathBuf,
    requested: bool,
    completed: Arc<AtomicBool>,
}

/// Captures a country level: `MOTO_SHOT=<level id>,<start x>,<frames>,<throttle>`.
#[test]
#[ignore = "requires a Linux display and GPU; writes a screenshot into the temp directory"]
fn capture_country() {
    let spec = std::env::var("MOTO_SHOT").unwrap_or_else(|_| "aegypten-duenen,100,60,0".into());
    let parts: Vec<&str> = spec.split(',').collect();
    let mut level = crate::levels::all()
        .into_iter()
        .find(|level| level.id == parts[0])
        .expect("a level id");
    let x: f32 = parts[1].parse().unwrap();
    level.start = Vec2::new(x, level.ground_height(x).unwrap());
    let path = std::env::temp_dir().join(format!("moto-devil-{}-{}.png", parts[0], parts[1]));
    let completed = Arc::new(AtomicBool::new(false));
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: (1280, 720).into(),
                    visible: false,
                    focused: false,
                    ..default()
                }),
                ..default()
            })
            .set(WinitPlugin {
                run_on_any_thread: true,
            }),
    )
    .add_plugins((
        SimulationPlugin,
        crate::visuals::VisualsPlugin,
        crate::backdrop::BackdropPlugin,
        crate::camera::CameraPlugin,
        crate::hud::HudPlugin,
    ))
    .insert_resource(Levels(vec![level]))
    .insert_state(Screen::Playing)
    .insert_resource(WinitSettings::continuous())
    .insert_resource(TimeUpdateStrategy::FixedTimesteps(1))
    .insert_resource(CountryShot {
        frames: 0,
        at: parts[2].parse().unwrap(),
        throttle: parts[3].parse().unwrap(),
        path,
        requested: false,
        completed: completed.clone(),
    })
    .add_systems(Update, shoot_country);
    assert_eq!(app.run(), AppExit::Success);
    assert!(completed.load(Ordering::SeqCst));
}

fn shoot_country(
    mut commands: Commands,
    mut shot: ResMut<CountryShot>,
    mut controls: ResMut<Controls>,
) {
    shot.frames += 1;
    controls.throttle = shot.throttle;
    if shot.frames >= shot.at && !shot.requested {
        shot.requested = true;
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(shot.path.clone()))
            .observe(
                |_: On<ScreenshotCaptured>,
                 shot: Res<CountryShot>,
                 mut exit: MessageWriter<AppExit>| {
                    shot.completed.store(true, Ordering::SeqCst);
                    exit.write(AppExit::Success);
                },
            );
    }
}

/// Captures the "Tasten" screen while it waits for a new gas key, with the
/// jump left without a key.
#[test]
#[ignore = "requires a Linux display and GPU; writes a screenshot into the temp directory"]
fn capture_key_settings() {
    use crate::bindings::{Action, BindingEditor, KeyBindings};

    let mut keys = KeyBindings::default();
    keys.clear(Action::Jump, 0);
    let completed = Arc::new(AtomicBool::new(false));
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: (960, 540).into(),
                    visible: false,
                    focused: false,
                    ..default()
                }),
                ..default()
            })
            .set(WinitPlugin {
                run_on_any_thread: true,
            }),
    )
    .add_plugins((
        SimulationPlugin,
        crate::camera::CameraPlugin,
        crate::hud::HudPlugin,
        crate::settings::SettingsViewPlugin,
    ))
    .insert_resource(Levels(vec![flat()]))
    .insert_resource(keys)
    .insert_resource(BindingEditor {
        row: 0,
        slot: 1,
        waiting: true,
        notice: None,
    })
    .insert_state(Screen::Settings)
    .insert_resource(WinitSettings::continuous())
    .insert_resource(CountryShot {
        frames: 0,
        at: 20,
        throttle: 0.0,
        path: std::env::temp_dir().join("moto-devil-keys.png"),
        requested: false,
        completed: completed.clone(),
    })
    .add_systems(Update, shoot_country);
    assert_eq!(app.run(), AppExit::Success);
    assert!(completed.load(Ordering::SeqCst));
}
