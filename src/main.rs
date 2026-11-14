//! Moto Devil: a stunt-bike game whose levels lie to you.

mod backdrop;
mod bike;
mod bindings;
mod camera;
mod course;
mod effects;
mod flow;
mod hud;
mod input;
mod level;
mod levels;
mod map;
mod menu;
mod physics;
mod progress;
mod run;
mod save;
mod settings;
mod simulation;
mod traps;
mod visuals;

#[cfg(test)]
mod tests;

use bevy::prelude::*;

fn main() -> AppExit {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(window()),
        ..default()
    }));
    if let Some(store) = save_store() {
        app.insert_resource(store);
    }
    app.add_plugins((
        simulation::SimulationPlugin,
        save::SavePlugin,
        input::InputPlugin,
        visuals::VisualsPlugin,
        backdrop::BackdropPlugin,
        camera::CameraPlugin,
        hud::HudPlugin,
        menu::MenuPlugin,
        map::LevelMapPlugin,
        settings::SettingsViewPlugin,
    ))
    .run()
}

/// A 960 × 540 window on the desktop.
#[cfg(not(target_arch = "wasm32"))]
fn window() -> Window {
    Window {
        title: "Moto Devil".into(),
        resolution: (960, 540).into(),
        ..default()
    }
}

/// In the browser, the canvas of `web/index.html`, sized by the page. Bevy keeps
/// the browser's own handling of keys away from it, so the arrows and the space
/// bar do not scroll and Tab does not leave the game.
#[cfg(target_arch = "wasm32")]
fn window() -> Window {
    Window {
        title: "Moto Devil".into(),
        canvas: Some("#moto-devil".into()),
        fit_canvas_to_parent: true,
        ..default()
    }
}

/// Files in the user's data directory, if there is one (ADR 0005, ADR 0010).
#[cfg(not(target_arch = "wasm32"))]
fn save_store() -> Option<save::SaveStore> {
    save::FileStore::in_data_dir().map(save::SaveStore::new)
}

/// The page's local storage, if the browser allows it (ADR 0010).
#[cfg(target_arch = "wasm32")]
fn save_store() -> Option<save::SaveStore> {
    save::BrowserStore::open().map(save::SaveStore::new)
}
