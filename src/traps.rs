//! Traps, hazards, the finish flag, and loops: the parts of a level that change
//! while riding.
//!
//! Each trap fires once when the bike's body enters its trigger zone and re-arms
//! when the run starts a new attempt (a respawn or a restart). Traps fire the
//! same way every time; nothing here is random.

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::bike::{Bike, BikeTuning};
use crate::level::{Loop, SPIKE_HEIGHT, TrapDef, TrapKind};
use crate::physics::{GRAVITY, Pose, PreviousPose};
use crate::run::Run;

/// The slab gives way this long after the bike lands on it.
pub const SLAB_DELAY: f32 = 0.15;
/// Pop-up spikes need this long to come fully out.
pub const SPIKES_RISE: f32 = 0.08;
/// A falling pillar accelerates this fast.
pub const PILLAR_GRAVITY: f32 = 3000.0;
/// A pillar rests on the floor this long before rising.
pub const PILLAR_HOLD: f32 = 1.0;
pub const PILLAR_RISE_SPEED: f32 = 250.0;
/// TNT explodes this long after it is triggered.
pub const TNT_FUSE: f32 = 0.6;
/// The blast is deadly for this long.
pub const TNT_BLAST: f32 = 0.25;

/// Something deadly to touch, or `None` while it is harmless.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct Hazard(pub Option<HazardShape>);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HazardShape {
    Rect(Rect),
    Circle { center: Vec2, radius: f32 },
}

impl HazardShape {
    /// Whether a circle overlaps this shape.
    pub fn touches(&self, center: Vec2, radius: f32) -> bool {
        match *self {
            Self::Rect(rect) => center.clamp(rect.min, rect.max).distance(center) < radius,
            Self::Circle {
                center: other,
                radius: other_radius,
            } => center.distance(other) < radius + other_radius,
        }
    }
}

/// A trap. Its [`Pose`] is the offset of the moving part from where the level
/// places it.
#[derive(Component, Clone, Debug)]
#[require(Pose, PreviousPose, Hazard)]
pub struct Trap {
    pub def: TrapDef,
    /// Seconds since the trap fired, while it is fired.
    pub fired: Option<f32>,
    /// The run attempt this trap was armed for.
    pub attempt: u32,
    /// The slab's collider, for a dropping slab.
    pub collider: Option<Entity>,
    collider_off: bool,
}

impl Trap {
    pub fn new(def: TrapDef, collider: Option<Entity>) -> Self {
        Self {
            def,
            fired: None,
            attempt: 0,
            collider,
            collider_off: false,
        }
    }

    /// For TNT: whether it has exploded (the crate is gone).
    pub fn exploded(&self) -> bool {
        matches!(self.def.kind, TrapKind::Tnt { .. }) && self.fired.is_some_and(|t| t >= TNT_FUSE)
    }
}

/// The finish zone. A jumping-finish trap moves it away from `home`.
#[derive(Component, Clone, Copy, Debug)]
#[require(Pose, PreviousPose)]
pub struct Finish {
    pub zone: Rect,
    pub home: Rect,
}

/// A loop's state. Its rising half (right side) is solid until the bike passes
/// the top; then the falling half (left side) is solid and the rising half lets
/// the bike leave at the bottom. See [`LoopHalf`].
#[derive(Component, Clone, Copy, Debug)]
pub struct LoopState {
    pub def: Loop,
    pub falling: bool,
}

/// One half of a loop's track.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoopArc {
    pub owner: Entity,
    pub half: LoopHalf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopHalf {
    /// From the bottom up the right side to the top.
    Rising,
    /// From the top down the left side to the bottom.
    Falling,
}

impl LoopHalf {
    /// The arc's start and end angle, counter-clockwise from +x.
    pub fn angles(self) -> (f32, f32) {
        use std::f32::consts::FRAC_PI_2;
        match self {
            Self::Rising => (-FRAC_PI_2, FRAC_PI_2),
            Self::Falling => (FRAC_PI_2, 3.0 * FRAC_PI_2),
        }
    }

    /// Points along the inner surface of a loop.
    pub fn points(self, def: &Loop) -> Vec<Vec2> {
        let (from, to) = self.angles();
        const STEPS: usize = 24;
        (0..=STEPS)
            .map(|step| {
                let angle = from + (to - from) * step as f32 / STEPS as f32;
                def.center + def.radius * Vec2::from_angle(angle)
            })
            .collect()
    }
}

/// How far a falling pillar has come down `fall` units after `t` seconds.
pub fn pillar_drop(t: f32, fall: f32) -> f32 {
    let drop_time = (2.0 * fall / PILLAR_GRAVITY).sqrt();
    if t < drop_time {
        0.5 * PILLAR_GRAVITY * t * t
    } else if t < drop_time + PILLAR_HOLD {
        fall
    } else {
        (fall - PILLAR_RISE_SPEED * (t - drop_time - PILLAR_HOLD)).max(0.0)
    }
}

fn set_collider(commands: &mut Commands, collider: Entity, enabled: bool) {
    if enabled {
        commands.entity(collider).remove::<ColliderDisabled>();
    } else {
        commands.entity(collider).insert(ColliderDisabled);
    }
}

fn apply_loop(
    commands: &mut Commands,
    owner: Entity,
    falling: bool,
    arcs: &Query<(Entity, &LoopArc)>,
) {
    for (entity, arc) in arcs.iter().filter(|(_, arc)| arc.owner == owner) {
        let solid = (arc.half == LoopHalf::Falling) == falling;
        set_collider(commands, entity, solid);
    }
}

/// Re-arms traps, the finish, and loops when the run starts a new attempt.
pub fn rearm_course(
    mut commands: Commands,
    run: Res<Run>,
    mut armed_for: Local<Option<u32>>,
    mut traps: Query<(&mut Trap, &mut Pose, &mut Hazard), Without<Finish>>,
    mut finishes: Query<(&mut Finish, &mut Pose)>,
    mut loops: Query<(Entity, &mut LoopState)>,
    arcs: Query<(Entity, &LoopArc)>,
) {
    if *armed_for == Some(run.attempt) {
        return;
    }
    *armed_for = Some(run.attempt);
    for (mut trap, mut pose, mut hazard) in &mut traps {
        if trap.attempt == run.attempt && trap.fired.is_none() {
            continue;
        }
        trap.attempt = run.attempt;
        trap.fired = None;
        *pose = Pose::default();
        *hazard = Hazard(None);
        if let Some(collider) = trap.collider
            && trap.collider_off
        {
            set_collider(&mut commands, collider, true);
            trap.collider_off = false;
        }
    }
    for (mut finish, mut pose) in &mut finishes {
        finish.zone = finish.home;
        *pose = Pose::at(finish.home.center());
    }
    for (owner, mut state) in &mut loops {
        if state.falling {
            state.falling = false;
            apply_loop(&mut commands, owner, false, &arcs);
        }
    }
}

/// Moves fired traps along and updates their hazards.
pub fn update_traps(
    mut commands: Commands,
    time: Res<Time>,
    mut traps: Query<(&mut Trap, &mut Pose, &mut Hazard)>,
) {
    let dt = time.delta_secs();
    for (mut trap, mut pose, mut hazard) in &mut traps {
        if let Some(t) = trap.fired.as_mut() {
            *t += dt;
        }
        let t = trap.fired;
        match trap.def.kind {
            TrapKind::DropSlab { .. } => {
                let falling = t.map(|t| t - SLAB_DELAY).filter(|t| *t > 0.0);
                pose.translation.y = falling.map_or(0.0, |t| -(0.5 * GRAVITY * t * t).min(800.0));
                if falling.is_some() && !trap.collider_off {
                    if let Some(collider) = trap.collider {
                        set_collider(&mut commands, collider, false);
                    }
                    trap.collider_off = true;
                }
            }
            TrapKind::PopUpSpikes { spikes } => {
                let raised = t.map_or(0.0, |t| (t / SPIKES_RISE).min(1.0));
                pose.translation.y = (raised - 1.0) * SPIKE_HEIGHT;
                hazard.0 = (raised > 0.3).then(|| HazardShape::Rect(spikes.hazard(raised)));
            }
            TrapKind::FallingPillar { pillar, floor } => {
                let drop = t.map_or(0.0, |t| pillar_drop(t, pillar.min.y - floor));
                pose.translation.y = -drop;
                let down = Vec2::new(0.0, drop);
                hazard.0 = Some(HazardShape::Rect(Rect::from_corners(
                    pillar.min - down,
                    pillar.max - down,
                )));
            }
            TrapKind::Tnt {
                crate_box,
                blast_radius,
            } => {
                let blasting = t.is_some_and(|t| (TNT_FUSE..TNT_FUSE + TNT_BLAST).contains(&t));
                hazard.0 = blasting.then_some(HazardShape::Circle {
                    center: crate_box.center(),
                    radius: blast_radius,
                });
            }
            TrapKind::JumpingFinish { .. } => {}
        }
    }
}

/// The bike's body, apart from traps and the finish, which also have poses.
type BikeBody = (With<Bike>, Without<Trap>, Without<Finish>);

/// Fires armed traps whose trigger zone contains the bike's body.
pub fn trigger_traps(
    run: Res<Run>,
    bikes: Query<&Pose, BikeBody>,
    mut traps: Query<&mut Trap>,
    mut finishes: Query<(&mut Finish, &mut Pose), Without<Bike>>,
) {
    if !run.in_control() {
        return;
    }
    for body in &bikes {
        for mut trap in &mut traps {
            if trap.fired.is_some() || !trap.def.trigger.contains(body.translation) {
                continue;
            }
            trap.fired = Some(0.0);
            if let TrapKind::JumpingFinish { to } = trap.def.kind {
                for (mut finish, mut pose) in &mut finishes {
                    finish.zone = to;
                    *pose = Pose::at(to.center());
                }
            }
        }
    }
}

/// Switches which half of each loop is solid as the bike goes round.
pub fn switch_loops(
    mut commands: Commands,
    tuning: Res<BikeTuning>,
    bikes: Query<&Pose, With<Bike>>,
    mut loops: Query<(Entity, &mut LoopState)>,
    arcs: Query<(Entity, &LoopArc)>,
) {
    let Some(body) = bikes.iter().next() else {
        return;
    };
    let p = body.translation;
    let margin = 2.0 * tuning.wheel_offset.x;
    for (owner, mut state) in &mut loops {
        let Loop { center, radius } = state.def;
        let past_top =
            p.x < center.x && p.y > center.y + 0.3 * radius && p.distance(center) < radius;
        let left_loop = p.x > center.x + radius + margin
            || p.x < center.x - radius - margin
            || p.y < center.y - radius - margin;
        let falling = if state.falling { !left_loop } else { past_top };
        if falling != state.falling {
            state.falling = falling;
            apply_loop(&mut commands, owner, falling, &arcs);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pillar_drops_waits_and_rises() {
        let fall = 210.0;
        assert_eq!(pillar_drop(0.0, fall), 0.0);
        let landed = (2.0 * fall / PILLAR_GRAVITY).sqrt();
        assert!((pillar_drop(landed + 0.01, fall) - fall).abs() < 1e-3);
        assert_eq!(pillar_drop(landed + PILLAR_HOLD - 0.01, fall), fall);
        assert!(pillar_drop(landed + PILLAR_HOLD + 0.2, fall) < fall);
        assert_eq!(pillar_drop(100.0, fall), 0.0, "back up for good");
    }

    #[test]
    fn hazards_touch_circles() {
        let rect = HazardShape::Rect(Rect::new(0.0, 0.0, 10.0, 10.0));
        assert!(rect.touches(Vec2::new(15.0, 5.0), 6.0));
        assert!(!rect.touches(Vec2::new(17.0, 5.0), 6.0));
        let blast = HazardShape::Circle {
            center: Vec2::ZERO,
            radius: 100.0,
        };
        assert!(blast.touches(Vec2::new(105.0, 0.0), 10.0));
        assert!(!blast.touches(Vec2::new(120.0, 0.0), 10.0));
    }

    #[test]
    fn loop_halves_meet_at_top_and_bottom() {
        let def = Loop {
            center: Vec2::new(0.0, 100.0),
            radius: 100.0,
        };
        let rising = LoopHalf::Rising.points(&def);
        let falling = LoopHalf::Falling.points(&def);
        assert!(rising[0].distance(Vec2::ZERO) < 1e-3);
        assert!(rising.last().unwrap().distance(*falling.first().unwrap()) < 1e-3);
        assert!(falling.last().unwrap().distance(Vec2::ZERO) < 1e-3);
        assert!(
            rising.iter().all(|p| p.x >= -1e-3),
            "rising is the right half"
        );
    }
}
