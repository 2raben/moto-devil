//! Scripted rides through every country level. The pilot knows each level's
//! tricks, like a player on a later attempt: it waits out timed traps, jumps
//! what it must, and turns around when the finish runs away. Each level must be
//! finished without a single crash.

use super::*;
use crate::effects::RideEffects;
use crate::traps::{AVALANCHE_WARNING, Finish};

/// What the pilot needs to know about a level beyond the generic rules.
#[derive(Default)]
struct Plan {
    /// Press jump while grounded with the body inside one of these x ranges.
    jumps: &'static [(f32, f32)],
    /// Ride with this throttle inside these x ranges.
    slow: &'static [(f32, f32, f32)],
    /// Hold jump to use jet fuel inside these x ranges.
    jets: &'static [(f32, f32)],
    /// Stand still inside these x ranges until a timed trap is done.
    shelters: &'static [(f32, f32)],
}

/// How long a fired trap keeps the pilot waiting, if it waits for it at all.
fn wait_for(kind: TrapKind) -> Option<f32> {
    match kind {
        TrapKind::Raven { .. } => Some(2.1),
        TrapKind::FallingPillar { .. } | TrapKind::RisingPillar { .. } => Some(3.0),
        TrapKind::Tnt { .. } => Some(1.0),
        TrapKind::Avalanche { from_x, to_x, .. } if to_x < from_x => Some(f32::MAX),
        _ => None,
    }
}

impl Sim {
    /// Rides the current level with the pilot; returns whether it finished.
    /// `sprung` collects the triggers of the traps that fired on the way.
    fn fly(&mut self, plan: &Plan, seconds: f32, sprung: &mut Vec<Rect>) -> bool {
        let mut jumped: Vec<(usize, u32)> = Vec::new();
        for step in 0..(seconds / STEP) as usize {
            if self.run().finished {
                return true;
            }
            let p = self.body().translation;
            let state = self.state();
            let grounded = state.grounded();
            let ride = self.bike::<RideEffects>();
            let velocity = self.bike::<bevy_rapier2d::prelude::Velocity>().linear;
            let attempt = self.run().attempt;
            let world = self.app.world_mut();
            let finish = world
                .query::<&Finish>()
                .single(world)
                .expect("one finish")
                .zone;
            let mut wait = false;
            let mut jump_ball = false;
            let mut hold_back = false;
            let sheltered = plan.shelters.iter().any(|&(a, b)| (a..b).contains(&p.x));
            let mut fleeing = false;
            for (trap, pose) in world.query::<(&Trap, &Pose)>().iter(world) {
                if trap.fired.is_some() && !sprung.contains(&trap.def.trigger) {
                    sprung.push(trap.def.trigger);
                }
                let busy =
                    |duration: f32| trap.fired.is_some_and(|t| t < duration) && !trap.departed();
                if let Some(duration) = wait_for(trap.def.kind) {
                    let near = p.x >= trap.def.trigger.min.x - 10.0
                        && p.x < trap.def.trigger.max.x + 240.0;
                    wait |= busy(duration) && (near || sheltered);
                }
                if let TrapKind::Avalanche { .. } = trap.def.kind
                    && busy(f32::MAX)
                {
                    // Stand under a gallery roof, or outrun it.
                    wait |= sheltered;
                    fleeing |= !sheltered
                        && wait_for(trap.def.kind).is_none()
                        && trap.fired.is_some_and(|t| t > AVALANCHE_WARNING);
                }
                if let TrapKind::Ball {
                    start,
                    radius,
                    speed,
                    bounce_height,
                    ..
                } = trap.def.kind
                {
                    let Some(age) = trap.fired.filter(|_| !trap.departed()) else {
                        continue;
                    };
                    let gap = start.x + pose.translation.x - p.x;
                    if bounce_height == 0.0 {
                        jump_ball |= (0.0..300.0).contains(&gap);
                    } else if (-radius..900.0).contains(&gap) {
                        // Pass a hopping ball while it is high up: brake if it
                        // would be low when it reaches the bike.
                        let closing = (velocity.x.max(0.0) + speed).max(1.0);
                        let meet = age + (gap - 60.0).max(0.0) / closing;
                        let up = (2.0 * bounce_height / crate::physics::GRAVITY).sqrt();
                        let phase = meet.rem_euclid(2.0 * up);
                        let height = crate::physics::GRAVITY * up * phase
                            - 0.5 * crate::physics::GRAVITY * phase * phase;
                        hold_back |= gap > 60.0 && height < 0.6 * bounce_height;
                    }
                }
            }
            if fleeing {
                wait = false;
            }
            let in_range =
                |ranges: &[(f32, f32)]| ranges.iter().position(|&(a, b)| (a..b).contains(&p.x));
            let mut controls = Controls {
                throttle: 1.0,
                ..default()
            };
            if let Some(&(_, _, throttle)) =
                plan.slow.iter().find(|&&(a, b, _)| (a..b).contains(&p.x))
            {
                controls.throttle = throttle;
            }
            if wait || hold_back {
                controls.throttle = 0.0;
                controls.brake = 1.0;
            }
            if let Some(range) = in_range(plan.jumps)
                && grounded
                && !jumped.contains(&(range, attempt))
            {
                controls.jump_pressed = true;
                jumped.push((range, attempt));
            }
            if grounded && (jump_ball || ride.sink > 0.5) {
                controls.jump_pressed = true;
            }
            if ride.stuck.is_some() {
                controls.jump_pressed = true;
            }
            controls.jump_held =
                ride.jet_left > 0.0 && in_range(plan.jets).is_some() && velocity.y < 150.0;
            // The finish ran away: turn towards it.
            let behind = finish.max.x < p.x - 20.0;
            let ahead = finish.min.x > p.x + 20.0;
            if grounded
                && state.turning.is_none()
                && ((behind && state.facing > 0.0) || (ahead && state.facing < 0.0))
            {
                controls.throttle = 0.0;
                controls.turn_pressed = true;
            }
            *self.controls() = controls;
            let deaths = self.run().deaths;
            self.steps(1);
            if std::env::var_os("PILOT_TRACE").is_some() && step % 8 == 0 {
                println!(
                    "t={:.2} x={:.0} y={:.0} v=({:.0},{:.0}) ground={grounded} throttle={} brake={} jump={} sink={:.2}",
                    step as f32 * STEP,
                    p.x,
                    p.y,
                    velocity.x,
                    velocity.y,
                    controls.throttle,
                    controls.brake,
                    controls.jump_pressed,
                    ride.sink
                );
            }
            if self.run().deaths > deaths {
                println!(
                    "crash at x={:.0} y={:.0} (before: {p:?}, sink {:.2})",
                    self.body().translation.x,
                    self.body().translation.y,
                    ride.sink
                );
                return false;
            }
        }
        self.run().finished
    }
}

fn ride_level(id: &str, plan: Plan) {
    let level = crate::levels::all()
        .into_iter()
        .find(|level| level.id == id)
        .unwrap_or_else(|| panic!("no level {id}"));
    // Hatches and bridges that give way are jumped over, so they may stay
    // armed; every other lie is met on the way.
    let triggers: Vec<Rect> = level
        .traps
        .iter()
        .filter(|trap| !matches!(trap.kind, TrapKind::DropSlab { .. }))
        .map(|trap| trap.trigger)
        .collect();
    let mut sim = Sim::riding(level);
    let mut sprung = Vec::new();
    let finished = sim.fly(&plan, 180.0, &mut sprung);
    let run = sim.run();
    assert!(finished, "{id}: did not finish, at {:?}", sim.body());
    assert_eq!(run.deaths, 0, "{id}: crashed");
    let missed: Vec<_> = triggers.iter().filter(|t| !sprung.contains(t)).collect();
    assert!(missed.is_empty(), "{id}: traps never fired: {missed:?}");
}

macro_rules! level_rides {
    ($($name:ident: $id:literal => $plan:expr;)*) => {
        $(
            #[test]
            fn $name() {
                ride_level($id, $plan);
            }
        )*
    };
}

level_rides! {
    egypt_dunes: "aegypten-duenen" => Plan {
        jumps: &[(1880.0, 1915.0), (2530.0, 2565.0)],
        ..default()
    };
    egypt_valley: "aegypten-tal-der-koenige" => Plan {
        jumps: &[(3430.0, 3465.0), (5060.0, 5095.0)],
        ..default()
    };
    egypt_pyramid: "aegypten-pyramide" => Plan {
        jumps: &[(330.0, 365.0), (1250.0, 1290.0), (4010.0, 4040.0)],
        ..default()
    };
    australia_outback: "australien-outback" => Plan {
        jumps: &[(2335.0, 2370.0), (5355.0, 5390.0)],
        ..default()
    };
    australia_gorges: "australien-schluchten" => Plan {
        jumps: &[(535.0, 570.0), (2830.0, 2865.0), (3430.0, 3700.0), (6525.0, 6560.0)],
        ..default()
    };
    australia_red_rock: "australien-roter-felsen" => Plan {
        jumps: &[(2200.0, 2230.0)],
        jets: &[(3200.0, 4400.0)],
        ..default()
    };
    usa_desert: "usa-wueste" => Plan {
        jumps: &[(2235.0, 2270.0), (4420.0, 4455.0)],
        ..default()
    };
    usa_canyon: "usa-canyon" => Plan {
        jumps: &[(760.0, 795.0), (3760.0, 3795.0)],
        ..default()
    };
    usa_gold_mine: "usa-goldmine" => Plan {
        jumps: &[(3135.0, 3200.0), (5735.0, 5770.0)],
        ..default()
    };
    paris_rooftops: "paris-daecher" => Plan {
        jumps: &[
            (735.0, 770.0),
            (1440.0, 1475.0),
            (3335.0, 3370.0),
            (4070.0, 4105.0),
            (6055.0, 6090.0),
            (6780.0, 6815.0),
        ],
        ..default()
    };
    paris_montmartre: "paris-montmartre" => Plan {
        jumps: &[(1235.0, 1270.0), (1870.0, 1905.0), (5085.0, 5120.0)],
        ..default()
    };
    paris_eiffel_tower: "paris-eiffelturm" => Plan {
        jumps: &[(1800.0, 1835.0), (2635.0, 2670.0), (5585.0, 5620.0), (5830.0, 5870.0)],
        jets: &[(3650.0, 4750.0), (5560.0, 5900.0)],
        ..default()
    };
    switzerland_alp_path: "schweiz-alpweg" => Plan {
        jumps: &[(4700.0, 4735.0)],
        shelters: &[(1250.0, 1880.0)],
        ..default()
    };
    switzerland_pass: "schweiz-pass" => Plan {
        jumps: &[(3220.0, 3255.0)],
        shelters: &[(5650.0, 6100.0)],
        ..default()
    };
    switzerland_summit: "schweiz-gipfel" => Plan {
        jumps: &[(5700.0, 5735.0), (5960.0, 6000.0)],
        shelters: &[(1450.0, 1820.0), (4450.0, 4950.0)],
        ..default()
    };
}
