//! Contact effects and temporary flight. The course owns pickup/contact state;
//! the bike owns its current effects. Neither survives a new attempt.

use bevy::prelude::*;
use bevy_rapier2d::prelude::Velocity;

use crate::bike::{self, Bike, BikeState, BikeTuning, Wheel};
use crate::physics::Pose;
use crate::run::Run;
use crate::traps::HazardShape;

pub const JET_SECONDS: f32 = 2.0;
pub const JET_ACCEL: f32 = 1800.0;
pub const JET_MAX_RISE: f32 = 420.0;
pub const MUD_SPEED_FACTOR: f32 = 0.35;
/// Seconds a bike standing in quicksand takes to sink completely.
pub const QUICKSAND_SINK_SECONDS: f32 = 1.2;
/// A bike at full speed sinks this much slower than a standing one.
pub const QUICKSAND_SPEED_RELIEF: f32 = 0.6;
/// Out of the sand, a bike comes up this many times faster than it sank.
pub const QUICKSAND_RECOVERY: f32 = 2.5;
/// Gas pushes and the top speed reach this share of normal in quicksand.
pub const QUICKSAND_SPEED_FACTOR: f32 = 0.6;
/// Fraction of its speed the bike loses per second while in quicksand.
pub const QUICKSAND_DRAG: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EffectKind {
    Cactus,
    Mud,
    /// Swallows a bike that stops: it sinks while a wheel touches the sand,
    /// slower at speed, and crashes when it has sunk completely.
    Quicksand,
    /// Sets a repeatable launch velocity for the whole bike on a landing.
    Trampoline {
        velocity: Vec2,
    },
    Jet,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectDef {
    pub zone: Rect,
    pub kind: EffectKind,
}

/// Shared by collision and presentation: empty space between cactus arms is
/// not sticky. All rectangles are in world coordinates.
pub fn cactus_parts(zone: Rect) -> [Rect; 4] {
    let center = zone.center();
    let arm_x = (zone.width() * 0.5 - 5.0).max(0.0);
    let arm_height = zone.height() * 0.3 + 5.0;
    [
        Rect::from_center_size(center, Vec2::new(14.0, zone.height())),
        Rect::from_center_size(center - Vec2::Y * 5.0, Vec2::new(2.0 * arm_x + 10.0, 10.0)),
        Rect::from_center_size(
            center + Vec2::new(-arm_x, -5.0 + arm_height * 0.5),
            Vec2::new(10.0, arm_height),
        ),
        Rect::from_center_size(
            center + Vec2::new(arm_x, -5.0 + arm_height * 0.5),
            Vec2::new(10.0, arm_height),
        ),
    ]
}

#[derive(Component, Debug)]
pub struct CourseEffect {
    pub def: EffectDef,
    pub spent: bool,
    touching: bool,
    attempt: u32,
}

impl CourseEffect {
    pub fn new(def: EffectDef) -> Self {
        Self {
            def,
            spent: false,
            touching: false,
            attempt: 0,
        }
    }
}

#[derive(Component, Clone, Debug, Default)]
pub struct RideEffects {
    pub stuck: Option<Entity>,
    pub muddy: bool,
    /// The top of the quicksand while a wheel touches it.
    pub sand_surface: Option<f32>,
    /// How far the bike has sunk, from `0.0` to `1.0` (buried).
    pub sink: f32,
    pub jet_left: f32,
    pub jet_active: bool,
    pub launch: Option<Vec2>,
    /// A released cactus cannot catch the bike until it has left its area.
    pub immune_cacti: Vec<Entity>,
    attempt: u32,
}

impl RideEffects {
    /// The bike has sunk completely into quicksand.
    pub fn buried(&self) -> bool {
        self.sink >= 1.0
    }
}

/// How much a bike sinks in one step of `dt` seconds, or comes up again when it
/// is out of the sand. `speed_share` is its speed as a share of the top speed.
pub fn sink_step(sink: f32, in_sand: bool, speed_share: f32, dt: f32) -> f32 {
    let rate = dt / QUICKSAND_SINK_SECONDS;
    if in_sand {
        let relief = 1.0 - QUICKSAND_SPEED_RELIEF * speed_share.clamp(0.0, 1.0);
        (sink + rate * relief).min(1.0)
    } else {
        (sink - rate * QUICKSAND_RECOVERY).max(0.0)
    }
}

/// Runs after respawn and before driving, including on a newly loaded course.
pub fn reset_effects(
    run: Res<Run>,
    mut effects: Query<&mut CourseEffect>,
    mut riders: Query<&mut RideEffects>,
) {
    for mut effect in &mut effects {
        if effect.attempt != run.attempt {
            effect.attempt = run.attempt;
            effect.spent = false;
            effect.touching = false;
        }
    }
    for mut ride in &mut riders {
        if ride.attempt != run.attempt || !run.in_control() {
            *ride = RideEffects {
                attempt: run.attempt,
                ..default()
            };
        }
    }
}

/// Reads actual wheel contacts with effect regions after physics and crash
/// detection. All overlapping effects are considered; no first-match rule.
pub fn sense_effects(
    time: Res<Time>,
    run: Res<Run>,
    tuning: Res<BikeTuning>,
    mut bikes: Query<(&Bike, &BikeState, &Pose, &Velocity, &mut RideEffects), Without<Wheel>>,
    wheels: Query<(&Pose, &Velocity), With<Wheel>>,
    mut effects: Query<(Entity, &mut CourseEffect)>,
) {
    for (bike, state, body, velocity, mut ride) in &mut bikes {
        if !run.in_control() {
            *ride = RideEffects {
                attempt: run.attempt,
                ..default()
            };
            continue;
        }
        let (Ok((left, lv)), Ok((right, rv))) =
            (wheels.get(bike.left_wheel), wheels.get(bike.right_wheel))
        else {
            continue;
        };
        let wheel_poses = [left.translation, right.translation];
        let hull = bike::hull(body, state.facing, wheel_poses, &tuning);
        ride.muddy = false;
        ride.sand_surface = None;
        for (entity, mut effect) in &mut effects {
            let shape = HazardShape::Rect(effect.def.zone);
            let touching = wheel_poses.iter().any(|&p| {
                if effect.def.kind == EffectKind::Cactus {
                    cactus_parts(effect.def.zone)
                        .iter()
                        .any(|&rect| HazardShape::Rect(rect).touches(p, tuning.wheel_radius))
                } else {
                    shape.touches(p, tuning.wheel_radius)
                }
            });
            match effect.def.kind {
                EffectKind::Cactus => {
                    // Keep release immunity through the gaps between arms and
                    // trunk; only leaving the whole plant can re-arm it.
                    if !wheel_poses
                        .iter()
                        .any(|&p| shape.touches(p, tuning.wheel_radius))
                    {
                        ride.immune_cacti.retain(|&id| id != entity);
                    }
                    if touching && ride.stuck.is_none() && !ride.immune_cacti.contains(&entity) {
                        ride.stuck = Some(entity);
                    }
                }
                EffectKind::Mud => ride.muddy |= touching,
                EffectKind::Quicksand => {
                    if touching {
                        let top = effect.def.zone.max.y;
                        ride.sand_surface = Some(ride.sand_surface.map_or(top, |y| y.max(top)));
                    }
                }
                EffectKind::Trampoline { velocity } => {
                    let landed_from_above = [(left, lv), (right, rv)].iter().any(|(p, v)| {
                        p.translation.y >= effect.def.zone.max.y
                            && v.linear.y <= 1.0
                            && shape.touches(p.translation, tuning.wheel_radius)
                    });
                    if touching && !effect.touching && landed_from_above {
                        ride.launch = Some(velocity);
                    }
                }
                EffectKind::Jet => {
                    if !effect.spent && hull.iter().any(|&(p, r)| shape.touches(p, r)) {
                        effect.spent = true;
                        ride.jet_left = JET_SECONDS;
                    }
                }
            }
            effect.touching = touching;
        }
        let speed_share = velocity.linear.length() / tuning.max_speed;
        ride.sink = sink_step(
            ride.sink,
            ride.sand_surface.is_some(),
            speed_share,
            time.delta_secs(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standing_in_quicksand_sinks_faster_than_riding_through() {
        let step = 1.0 / 64.0;
        let mut standing = 0.0;
        let mut riding = 0.0;
        for _ in 0..32 {
            standing = sink_step(standing, true, 0.0, step);
            riding = sink_step(riding, true, 1.0, step);
        }
        assert!(standing > 2.0 * riding, "{standing} vs {riding}");
        assert!((standing - 0.5 / QUICKSAND_SINK_SECONDS).abs() < 1e-3);
        let mut out = standing;
        for _ in 0..16 {
            out = sink_step(out, false, 0.0, step);
        }
        assert!(out < standing * 0.5, "comes up quickly out of the sand");
        assert_eq!(sink_step(0.99, true, 0.0, 1.0), 1.0);
        assert_eq!(sink_step(0.01, false, 0.0, 1.0), 0.0);
    }
}
