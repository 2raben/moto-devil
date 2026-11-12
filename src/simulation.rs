//! The headless game: game flow, levels, the bike, traps, and run rules, with
//! Rapier on the fixed timestep.
//!
//! This plugin adds no rendering, windowing, or device input, so tests can run it
//! with `MinimalPlugins`. It needs Bevy's `TransformPlugin` (for Rapier) and
//! `StatesPlugin`, which `DefaultPlugins` includes and tests add by hand. Presentation plugins read its components and
//! resources and write only [`Controls`] and [`MenuInput`].

use bevy::prelude::*;

use crate::bike::{self, BikeTuning, Controls};
use crate::flow::{self, CurrentLevel, Levels, MenuInput, Screen, Selection};
use crate::physics::{self, GameSystems, PhysicsPlugin};
use crate::progress::{ActivePlayer, Profiles};
use crate::run::{self, Run};
use crate::traps;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugin)
            .init_state::<Screen>()
            .init_resource::<Levels>()
            .init_resource::<CurrentLevel>()
            .init_resource::<MenuInput>()
            .init_resource::<Selection>()
            .init_resource::<Profiles>()
            .init_resource::<ActivePlayer>()
            .init_resource::<BikeTuning>()
            .init_resource::<Controls>()
            .init_resource::<Run>()
            .add_systems(OnEnter(Screen::Playing), flow::load_level)
            .add_systems(OnExit(Screen::Playing), flow::unload_level)
            .add_systems(
                Update,
                (
                    flow::select_player.run_if(in_state(Screen::SelectPlayer)),
                    flow::playing_menu.run_if(in_state(Screen::Playing)),
                ),
            )
            .add_systems(
                FixedUpdate,
                (
                    physics::remember_poses,
                    run::restart_or_respawn,
                    traps::rearm_course,
                    bike::drive_bike,
                    traps::update_traps,
                )
                    .chain()
                    .in_set(GameSystems::Drive)
                    .run_if(in_state(Screen::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (
                    physics::read_body_poses,
                    bike::sense_ground,
                    traps::trigger_traps,
                    traps::switch_loops,
                    run::reach_checkpoint,
                    run::detect_crash,
                    run::reach_finish,
                    run::tick_run,
                )
                    .chain()
                    .in_set(GameSystems::React)
                    .run_if(in_state(Screen::Playing)),
            );
    }
}
