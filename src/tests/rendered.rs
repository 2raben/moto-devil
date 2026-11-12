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
