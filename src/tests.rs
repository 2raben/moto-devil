//! Headless simulation tests: the real game systems and Rapier run at the fixed
//! timestep with inputs written directly to [`Controls`] and [`MenuInput`]. No
//! window or GPU is needed.
//!
//! Physics results are checked as ranges and outcomes, not exact positions.

use std::f32::consts::TAU;
use std::time::Duration;

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;

use crate::bike::{Bike, BikeState, BikeTuning, Controls};
use crate::flow::{CurrentLevel, Levels, MenuAction, MenuInput, Screen};
use crate::level::{Level, Profile, Spikes, TrapDef, TrapKind};
use crate::physics::Pose;
use crate::progress::{ActivePlayer, Grave, Profiles};
use crate::run::Run;
use crate::simulation::SimulationPlugin;
use crate::traps::Trap;

mod countries;
mod effects;
#[cfg(target_os = "linux")]
mod rendered;

/// Bevy's default fixed timestep; each update advances exactly one step.
const STEP: f32 = 1.0 / 64.0;

/// A long flat floor with walls at both ends and the finish far away.
fn flat() -> Level {
    Level {
        id: "flat",
        name: "Flat",
        country: crate::level::Country::Training,
        ground: Profile::start(-100.0, 400.0)
            .to(-100.0, 0.0)
            .to(4000.0, 0.0)
            .to(4000.0, 400.0)
            .build(),
        solids: vec![],
        loops: vec![],
        spikes: vec![],
        effects: vec![],
        traps: vec![],
        checkpoints: vec![],
        start: Vec2::new(100.0, 0.0),
        finish: Rect::new(3800.0, 0.0, 3850.0, 200.0),
        bounds: Rect::new(-100.0, -300.0, 4000.0, 800.0),
        fall_y: -100.0,
    }
}

/// A high platform that ends in a cliff, for long flights.
fn cliff() -> Level {
    Level {
        id: "cliff",
        ground: Profile::start(-100.0, 900.0)
            .to(-100.0, 500.0)
            .to(400.0, 500.0)
            .to(400.0, 0.0)
            .to(4000.0, 0.0)
            .to(4000.0, 400.0)
            .build(),
        start: Vec2::new(100.0, 500.0),
        bounds: Rect::new(-100.0, -300.0, 4000.0, 1200.0),
        ..flat()
    }
}

struct Sim {
    app: App,
}

impl Sim {
    fn new(levels: Vec<Level>) -> Self {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            StatesPlugin,
            SimulationPlugin,
        ))
        .insert_resource(Levels(levels))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            STEP,
        )));
        app.finish();
        app.cleanup();
        let mut sim = Self { app };
        // The first update runs startup; later ones advance one fixed step each.
        sim.app.update();
        sim
    }

    /// Starts riding `level` as the player "Test".
    fn riding(level: Level) -> Self {
        let mut sim = Self::new(vec![level]);
        sim.type_name("Test");
        sim.menu(MenuAction::Confirm);
        sim.steps(1);
        assert_eq!(sim.screen(), Screen::Playing);
        sim
    }

    fn steps(&mut self, count: usize) {
        for _ in 0..count {
            self.app.update();
        }
    }

    fn seconds(&mut self, seconds: f32) {
        self.steps((seconds / STEP).round() as usize);
    }

    /// Steps until `done` holds or `seconds` pass; returns whether it held.
    fn until(&mut self, seconds: f32, mut done: impl FnMut(&mut Self) -> bool) -> bool {
        for _ in 0..(seconds / STEP).round() as usize {
            if done(self) {
                return true;
            }
            self.steps(1);
        }
        done(self)
    }

    fn controls(&mut self) -> Mut<'_, Controls> {
        self.app.world_mut().resource_mut::<Controls>()
    }

    fn menu(&mut self, action: MenuAction) {
        self.app
            .world_mut()
            .resource_mut::<MenuInput>()
            .0
            .push(action);
        self.steps(1);
    }

    fn type_name(&mut self, name: &str) {
        for c in name.chars() {
            self.menu(MenuAction::Type(c));
        }
    }

    fn screen(&self) -> Screen {
        *self.app.world().resource::<State<Screen>>().get()
    }

    fn bike<T: Component + Clone>(&mut self) -> T {
        let world = self.app.world_mut();
        world
            .query_filtered::<&T, With<Bike>>()
            .single(world)
            .expect("one bike")
            .clone()
    }

    fn body(&mut self) -> Pose {
        self.bike::<Pose>()
    }

    fn state(&mut self) -> BikeState {
        self.bike::<BikeState>()
    }

    fn speed(&mut self) -> f32 {
        let world = self.app.world_mut();
        world
            .query_filtered::<&bevy_rapier2d::prelude::Velocity, With<Bike>>()
            .single(world)
            .expect("one bike")
            .linear
            .length()
    }

    fn run(&self) -> Run {
        self.app.world().resource::<Run>().clone()
    }

    fn profiles(&self) -> Profiles {
        self.app.world().resource::<Profiles>().clone()
    }

    fn level_id(&self) -> &'static str {
        self.app.world().resource::<Level>().id
    }

    fn tuning(&self) -> BikeTuning {
        *self.app.world().resource::<BikeTuning>()
    }

    fn traps_fired(&mut self) -> usize {
        let world = self.app.world_mut();
        world
            .query::<&Trap>()
            .iter(world)
            .filter(|trap| trap.fired.is_some())
            .count()
    }

    fn gas(&mut self, throttle: f32) {
        self.controls().throttle = throttle;
    }

    fn settle(&mut self) {
        self.seconds(0.6);
    }
}

#[test]
fn the_bike_rests_level_on_flat_ground() {
    let mut sim = Sim::riding(flat());
    sim.settle();
    let state = sim.state();
    assert!(state.left_contact && state.right_contact, "{state:?}");
    let body = sim.body();
    let rest = sim.tuning().wheel_radius - sim.tuning().wheel_offset.y;
    assert!((body.translation.y - rest).abs() < 2.0, "body at {body:?}");
    assert!(body.angle.abs() < 0.02);
    assert!((body.translation.x - 100.0).abs() < 2.0);
    assert_eq!(sim.run().deaths, 0);
}

#[test]
fn gas_drives_forward_and_the_brake_stops() {
    let mut sim = Sim::riding(flat());
    sim.settle();
    sim.gas(1.0);
    sim.seconds(2.0);
    let speed = sim.speed();
    let max = sim.tuning().max_speed;
    assert!(speed > 0.8 * max && speed < 1.05 * max, "speed {speed}");
    assert!(sim.body().translation.x > 700.0);
    assert_eq!(sim.run().deaths, 0, "full gas alone does not flip the bike");

    sim.gas(0.0);
    sim.controls().brake = 1.0;
    sim.seconds(1.5);
    assert!(sim.speed() < 15.0, "speed {}", sim.speed());
    assert_eq!(sim.run().deaths, 0);
}

#[test]
fn a_jump_lifts_the_bike_only_from_the_ground() {
    let mut sim = Sim::riding(flat());
    sim.settle();
    let rest = sim.body().translation.y;
    sim.controls().jump_pressed = true;
    let mut apex = rest;
    let landed = sim.until(2.0, |sim| {
        apex = apex.max(sim.body().translation.y);
        apex > rest + 10.0 && sim.state().grounded()
    });
    assert!(landed);
    let height = apex - rest;
    assert!((70.0..160.0).contains(&height), "jump height {height}");

    // A press in mid-air does nothing.
    sim.settle();
    sim.controls().jump_pressed = true;
    sim.seconds(0.3);
    let rising = sim.body().translation.y;
    sim.controls().jump_pressed = true;
    sim.seconds(0.1);
    assert!(sim.body().translation.y < rising + 30.0);
}

#[test]
fn holding_a_tilt_in_the_air_makes_a_flip() {
    let mut sim = Sim::riding(cliff());
    sim.settle();
    sim.gas(1.0);
    assert!(
        sim.until(3.0, |sim| !sim.state().grounded()),
        "left the cliff"
    );
    sim.gas(0.0);
    sim.controls().tilt = -1.0;
    assert!(
        sim.until(2.0, |sim| sim.state().air_turn >= TAU),
        "turned a full circle"
    );
    sim.controls().tilt = 0.0;
    assert!(sim.until(3.0, |sim| sim.state().grounded()));
    sim.settle();
    let run = sim.run();
    assert_eq!(run.deaths, 0, "the assist set the bike upright for landing");
    assert_eq!(run.flips, 1);
}

#[test]
fn the_bike_rights_itself_in_the_air() {
    let mut sim = Sim::riding(cliff());
    sim.settle();
    sim.gas(1.0);
    assert!(sim.until(3.0, |sim| !sim.state().grounded()));
    sim.gas(0.0);
    sim.controls().tilt = 1.0;
    sim.seconds(0.15);
    let tilted = sim.body().angle;
    assert!(tilted < -0.3, "tilted to the right: {tilted}");
    sim.controls().tilt = 0.0;
    assert!(sim.until(3.0, |sim| sim.state().grounded()));
    let landing = sim.body().angle;
    assert!(landing.abs() < 0.35, "landed at {landing}");
    sim.settle();
    assert_eq!(sim.run().deaths, 0);
    assert_eq!(sim.run().flips, 0);
}

#[test]
fn holding_tilt_on_the_ground_never_flips_the_bike_over() {
    let mut sim = Sim::riding(flat());
    sim.settle();
    sim.gas(1.0);
    sim.controls().tilt = -1.0;
    let limit = sim.tuning().max_ground_lean + 0.25;
    let mut steepest: f32 = 0.0;
    for _ in 0..(3.0 / STEP) as usize {
        sim.steps(1);
        steepest = steepest.max(sim.body().angle.abs());
    }
    assert!(steepest < limit, "leaned {steepest}");
    assert!(steepest > 0.3, "a wheelie happened");
    assert_eq!(sim.run().deaths, 0);
}

#[test]
fn turning_around_makes_gas_drive_left() {
    let mut sim = Sim::riding(flat());
    sim.settle();
    sim.gas(1.0);
    sim.seconds(1.0);
    sim.gas(0.0);
    sim.controls().turn_pressed = true;
    sim.seconds(0.5);
    assert_eq!(sim.state().facing, -1.0);
    let x = sim.body().translation.x;
    sim.gas(1.0);
    sim.seconds(1.5);
    assert!(sim.body().translation.x < x - 200.0, "drove left");
    assert_eq!(sim.run().deaths, 0);
}

#[test]
fn hitting_the_head_crashes_and_respawns_at_the_start() {
    let mut level = flat();
    // A block low enough for the wheels and body but not the head.
    level.solids = vec![Rect::new(500.0, 55.0, 700.0, 400.0)];
    let mut sim = Sim::riding(level);
    sim.settle();
    sim.gas(1.0);
    assert!(sim.until(3.0, |sim| sim.run().crash.is_some()), "crashed");
    let crash_x = sim.body().translation.x;
    assert!((400.0..520.0).contains(&crash_x), "crashed at {crash_x}");
    assert_eq!(sim.run().deaths, 1);
    sim.gas(0.0);
    assert!(sim.until(1.0, |sim| sim.run().crash.is_none()), "respawned");
    sim.settle();
    assert!((sim.body().translation.x - 100.0).abs() < 5.0);
    assert_eq!(sim.state().facing, 1.0);
    let Grave::Crash(x) = sim.profiles().graves("flat").next().unwrap().1 else {
        panic!("a crash grave");
    };
    assert!((x - crash_x).abs() < 20.0);
}

#[test]
fn falling_out_of_the_level_is_a_crash() {
    let mut level = flat();
    level.ground = Profile::start(-100.0, 400.0)
        .to(-100.0, 0.0)
        .to(400.0, 0.0)
        .to(400.0, -300.0)
        .to(4000.0, -300.0)
        .to(4000.0, 400.0)
        .build();
    let mut sim = Sim::riding(level);
    sim.settle();
    sim.gas(1.0);
    assert!(sim.until(3.0, |sim| sim.run().crash.is_some()));
    assert_eq!(sim.run().deaths, 1);
}

#[test]
fn pop_up_spikes_fire_in_their_zone_and_re_arm_after_a_crash() {
    let mut level = flat();
    level.traps = vec![TrapDef {
        kind: TrapKind::PopUpSpikes {
            spikes: Spikes {
                left: 600.0,
                base: 0.0,
                count: 4,
            },
        },
        trigger: Rect::new(450.0, 0.0, 700.0, 160.0),
    }];
    let mut sim = Sim::riding(level);
    sim.settle();
    sim.gas(0.6);
    assert!(
        sim.until(4.0, |sim| sim.run().crash.is_some()),
        "the spikes killed"
    );
    assert_eq!(sim.traps_fired(), 1);
    assert!(sim.body().translation.x > 450.0);
    sim.gas(0.0);
    sim.seconds(1.0);
    assert_eq!(sim.traps_fired(), 0, "re-armed");
}

#[test]
fn reaching_the_finish_stops_the_clock_and_records_progress() {
    let mut level = flat();
    level.finish = Rect::new(800.0, 0.0, 840.0, 200.0);
    let mut sim = Sim::riding(level);
    sim.gas(1.0);
    assert!(sim.until(5.0, |sim| sim.run().finished));
    let elapsed = sim.run().elapsed;
    sim.seconds(1.5);
    assert_eq!(sim.run().elapsed, elapsed);
    assert!(sim.speed() < 30.0, "the bike brakes by itself");
    assert_eq!(
        sim.profiles().graves("flat").next().unwrap().1,
        Grave::Finished
    );
}

#[test]
fn checkpoints_move_the_respawn_point() {
    let mut level = flat();
    level.checkpoints = vec![crate::level::Checkpoint {
        zone: Rect::new(400.0, 0.0, 440.0, 200.0),
        respawn: Vec2::new(500.0, 0.0),
    }];
    level.solids = vec![Rect::new(900.0, 55.0, 1000.0, 400.0)];
    let mut sim = Sim::riding(level);
    sim.gas(1.0);
    assert!(sim.until(4.0, |sim| sim.run().crash.is_some()));
    assert_eq!(sim.run().respawn, Vec2::new(500.0, 0.0));
    sim.gas(0.0);
    assert!(sim.until(1.0, |sim| sim.run().crash.is_none()));
    sim.settle();
    assert!((sim.body().translation.x - 500.0).abs() < 5.0);
}

#[test]
fn restart_resets_the_bike_and_the_run() {
    let mut sim = Sim::riding(flat());
    sim.gas(1.0);
    sim.seconds(1.0);
    sim.gas(0.0);
    sim.controls().restart = true;
    sim.steps(2);
    let run = sim.run();
    assert!(run.elapsed < 3.0 * STEP);
    assert_eq!((run.deaths, run.flips, run.finished), (0, 0, false));
    sim.settle();
    assert!((sim.body().translation.x - 100.0).abs() < 5.0);
}

#[test]
fn players_choose_a_name_and_continue_where_they_stopped() {
    let short = |id: &'static str| {
        let mut level = flat();
        level.id = id;
        level.finish = Rect::new(300.0, 0.0, 340.0, 200.0);
        level
    };
    let mut sim = Sim::new(vec![short("one"), short("two")]);
    assert_eq!(sim.screen(), Screen::SelectPlayer);
    sim.type_name("Mia");
    sim.menu(MenuAction::Confirm);
    sim.steps(1);
    assert_eq!(sim.screen(), Screen::Playing);
    assert_eq!(sim.level_id(), "one");

    sim.gas(1.0);
    assert!(sim.until(3.0, |sim| sim.run().finished));
    sim.menu(MenuAction::Confirm);
    sim.steps(2);
    assert_eq!(sim.level_id(), "two");
    assert!(!sim.run().finished);

    // Escape goes back; choosing Mia again continues at level two.
    sim.menu(MenuAction::Back);
    sim.steps(1);
    assert_eq!(sim.screen(), Screen::SelectPlayer);
    sim.type_name("mia");
    sim.menu(MenuAction::Confirm);
    sim.steps(1);
    assert_eq!(sim.level_id(), "two");
    assert_eq!(sim.profiles().players.len(), 1, "the same player");
    assert_eq!(sim.app.world().resource::<ActivePlayer>().0, Some(0));

    // After the last level the selection comes back.
    sim.gas(1.0);
    assert!(sim.until(3.0, |sim| sim.run().finished));
    sim.menu(MenuAction::Confirm);
    sim.steps(1);
    assert_eq!(sim.screen(), Screen::SelectPlayer);
    assert_eq!(sim.app.world().resource::<CurrentLevel>().0, 1);
}

#[test]
fn tab_opens_the_key_settings_and_escape_comes_back() {
    let mut sim = Sim::new(vec![flat()]);
    sim.type_name("Mia");
    sim.menu(MenuAction::Settings);
    sim.steps(1);
    assert_eq!(sim.screen(), Screen::Settings);
    sim.menu(MenuAction::Confirm);
    sim.steps(1);
    assert_eq!(
        sim.screen(),
        Screen::Settings,
        "Enter edits, it does not leave"
    );
    sim.menu(MenuAction::Back);
    sim.steps(1);
    assert_eq!(sim.screen(), Screen::SelectPlayer);
    assert!(
        sim.app.world().resource::<Messages<AppExit>>().is_empty(),
        "Escape on the settings does not quit"
    );
}

#[test]
fn escape_on_the_selection_quits() {
    let mut sim = Sim::new(vec![flat()]);
    sim.app
        .world_mut()
        .resource_mut::<MenuInput>()
        .0
        .push(MenuAction::Back);
    sim.app.update();
    let exits = sim.app.world().resource::<Messages<AppExit>>();
    assert!(!exits.is_empty());
}

/// What the scripted rider sees each step.
struct Look {
    x: f32,
    grounded: bool,
    facing: f32,
    attempt: u32,
    finish: Rect,
    pillar: Option<f32>,
    /// Some loop has switched to its falling half: the bike went over the top.
    looping: bool,
}

impl Sim {
    fn look(&mut self) -> Look {
        let body = self.body();
        let state = self.state();
        let world = self.app.world_mut();
        let finish = world
            .query::<&crate::traps::Finish>()
            .single(world)
            .expect("one finish")
            .zone;
        let pillar = world
            .query::<&Trap>()
            .iter(world)
            .find(|trap| matches!(trap.def.kind, TrapKind::FallingPillar { .. }))
            .and_then(|trap| trap.fired);
        let looping = world
            .query::<&crate::traps::LoopState>()
            .iter(world)
            .any(|state| state.falling);
        Look {
            x: body.translation.x,
            grounded: state.grounded(),
            facing: state.facing,
            attempt: self.run().attempt,
            finish,
            pillar,
            looping,
        }
    }

    /// Rides with `policy` until the finish or `seconds` pass; prints crashes.
    fn ride(&mut self, seconds: f32, mut policy: impl FnMut(&Look, &mut Controls)) -> bool {
        let mut deaths = 0;
        for _ in 0..(seconds / STEP) as usize {
            if self.run().finished {
                return true;
            }
            let look = self.look();
            let mut controls = *self.controls();
            policy(&look, &mut controls);
            *self.controls() = controls;
            self.steps(1);
            if self.run().deaths > deaths {
                deaths = self.run().deaths;
                println!(
                    "crash {deaths} at x={:.0} y={:.0}",
                    self.body().translation.x,
                    self.body().translation.y
                );
            }
        }
        self.run().finished
    }
}

#[test]
fn level_one_can_be_finished() {
    let mut sim = Sim::riding(crate::levels::level_one());
    let mut jumped = None;
    let finished = sim.ride(60.0, |look, controls| {
        controls.throttle = 1.0;
        // The first try learns about the spikes; later tries jump them.
        if look.attempt > 0 && jumped != Some(look.attempt) && look.x > 1760.0 && look.grounded {
            controls.jump_pressed = true;
            jumped = Some(look.attempt);
        }
    });
    assert!(finished, "finished level one");
    assert_eq!(sim.run().deaths, 1, "only the trap's first surprise");
}

#[test]
fn level_two_can_be_finished() {
    let mut sim = Sim::riding(crate::levels::level_two());
    let mut spikes_jumped = None;
    let mut ramp_jumped = None;
    let mut turned = false;
    let mut looped = false;
    let finished = sim.ride(120.0, |look, controls| {
        looped |= look.looping;
        *controls = Controls {
            throttle: 1.0,
            ..default()
        };
        if look.attempt > 0
            && spikes_jumped != Some(look.attempt)
            && (1615.0..1700.0).contains(&look.x)
            && look.grounded
        {
            controls.jump_pressed = true;
            spikes_jumped = Some(look.attempt);
        }
        // Wait for the pillar to fall and rise again.
        let pillar_busy = look.pillar.is_some_and(|t| t < 2.7);
        if (1900.0..2030.0).contains(&look.x) && pillar_busy {
            controls.throttle = 0.0;
            controls.brake = 1.0;
        }
        if ramp_jumped != Some(look.attempt) && look.x > 2350.0 && look.x < 2385.0 {
            controls.jump_pressed = true;
            ramp_jumped = Some(look.attempt);
        }
        // The finish jumped behind: turn around and ride back.
        if look.finish.max.x < look.x && look.facing > 0.0 && look.grounded && !turned {
            controls.throttle = 0.0;
            controls.turn_pressed = true;
            turned = true;
        }
    });
    assert!(finished, "finished level two");
    assert!(looped, "rode through the loop");
}
