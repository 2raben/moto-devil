//! Moto Devil: a stunt-bike game whose levels lie to you.

mod bike;
mod camera;
mod course;
mod effects;
mod flow;
mod hud;
mod input;
mod level;
mod levels;
mod menu;
mod physics;
mod progress;
mod run;
mod save;
mod simulation;
mod traps;
mod visuals;

#[cfg(test)]
mod tests;

use bevy::prelude::*;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Moto Devil".into(),
                resolution: (960, 540).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            simulation::SimulationPlugin,
            save::SavePlugin,
            input::InputPlugin,
            visuals::VisualsPlugin,
            camera::CameraPlugin,
            hud::HudPlugin,
            menu::MenuPlugin,
        ))
        .run()
}
