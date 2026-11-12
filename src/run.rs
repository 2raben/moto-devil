//! The rules of one run through a level: crashes and respawns, checkpoints, the
//! finish, the clock, and the progress they write into [`Profiles`].

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::bike::{self, Bike, BikeState, BikeTuning, BodyParts, Controls, Wheel};
use crate::effects::RideEffects;
use crate::level::Level;
use crate::physics::{BIKE, Pose, TrackedBody, WORLD};
use crate::progress::{ActivePlayer, Profiles};
use crate::traps::{Finish, Hazard};

/// How long the flip and checkpoint cues stay on screen.
pub const CUE_SECONDS: f32 = 1.2;

/// Progress through the current run.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct Run {
    /// Seconds since the run started; stops at the finish.
    pub elapsed: f32,
    pub deaths: u32,
    pub flips: u32,
    pub finished: bool,
    /// Seconds left until the respawn, after a crash.
    pub crash: Option<f32>,
    /// Where the bike comes back after a crash.
    pub respawn: Vec2,
    /// Counts respawns and restarts; traps re-arm when it changes.
    pub attempt: u32,
    /// The last flips landed and the seconds since, for the HUD.
    pub flip_cue: Option<(u32, f32)>,
    /// Seconds since the last new checkpoint, for the HUD.
    pub checkpoint_cue: Option<f32>,
}

impl Run {
    pub fn new(start: Vec2) -> Self {
        Self {
            respawn: start,
            ..default()
        }
    }

    /// The player steers: not crashed and not finished.
    pub fn in_control(&self) -> bool {
        !self.finished && self.crash.is_none()
    }
}

/// Consumes a restart, or respawns the bike once a crash's delay is over.
pub fn restart_or_respawn(
    mut commands: Commands,
    level: Res<Level>,
    tuning: Res<BikeTuning>,
    mut run: ResMut<Run>,
    mut controls: ResMut<Controls>,
    mut bikes: Query<(Entity, &Bike, &mut BikeState)>,
    mut bodies: Query<BodyParts, With<TrackedBody>>,
) {
    let restart = std::mem::take(&mut controls.restart);
    let ground = if restart {
        let attempt = run.attempt + 1;
        *run = Run {
            attempt,
            ..Run::new(level.start)
        };
        level.start
    } else if run.crash.is_some_and(|left| left <= 0.0) {
        run.crash = None;
        run.attempt += 1;
        run.respawn
    } else {
        return;
    };
    for (entity, bike, mut state) in &mut bikes {
        bike::place_bike(
            &mut commands,
            entity,
            bike,
            &mut state,
            ground,
            &tuning,
            &mut bodies,
        );
    }
}

/// Moves the respawn point when the bike reaches a checkpoint.
pub fn reach_checkpoint(level: Res<Level>, mut run: ResMut<Run>, bikes: Query<&Pose, With<Bike>>) {
    if !run.in_control() {
        return;
    }
    for body in &bikes {
        let reached = level
            .checkpoints
            .iter()
            .find(|checkpoint| checkpoint.zone.contains(body.translation));
        if let Some(checkpoint) = reached
            && run.respawn != checkpoint.respawn
        {
            run.respawn = checkpoint.respawn;
            run.checkpoint_cue = Some(0.0);
        }
    }
}

/// Detects a crash: the head touches level geometry, any part touches a hazard,
/// the bike sinks completely into quicksand, or it falls out. Moves the player's gravestone forward if this is the
/// farthest crash in this level.
#[allow(clippy::too_many_arguments)]
pub fn detect_crash(
    level: Res<Level>,
    tuning: Res<BikeTuning>,
    rapier: ReadRapierContext,
    active: Res<ActivePlayer>,
    mut profiles: ResMut<Profiles>,
    mut run: ResMut<Run>,
    bikes: Query<(&Bike, &BikeState, &Pose, &RideEffects)>,
    wheels: Query<&Pose, With<Wheel>>,
    hazards: Query<&Hazard>,
) {
    if !run.in_control() {
        return;
    }
    let Ok(context) = rapier.single() else {
        return;
    };
    for (bike, state, body, ride) in &bikes {
        let (Ok(left), Ok(right)) = (wheels.get(bike.left_wheel), wheels.get(bike.right_wheel))
        else {
            continue;
        };
        let head = bike::head_position(body, state.facing, &tuning);
        let mut head_hit = false;
        let filter = QueryFilter::new()
            .exclude_sensors()
            .groups(CollisionGroups::new(BIKE, WORLD));
        context.intersect_shape(
            head,
            0.0,
            &Collider::ball(tuning.head_radius),
            filter,
            |_, _| {
                head_hit = true;
                false
            },
        );
        let hull = bike::hull(
            body,
            state.facing,
            [left.translation, right.translation],
            &tuning,
        );
        let hazard_hit = hazards.iter().filter_map(|hazard| hazard.0).any(|shape| {
            hull.iter()
                .any(|&(center, radius)| shape.touches(center, radius))
        });
        let fell_out = body.translation.y < level.fall_y;
        if !(head_hit || hazard_hit || fell_out || ride.buried()) {
            continue;
        }
        run.crash = Some(tuning.crash_delay);
        run.deaths += 1;
        if let Some(player) = active.0 {
            profiles.record_crash(player, level.id, body.translation.x);
        }
        return;
    }
}

/// Finishes the run when the bike touches the finish zone.
pub fn reach_finish(
    level: Res<Level>,
    active: Res<ActivePlayer>,
    mut profiles: ResMut<Profiles>,
    mut run: ResMut<Run>,
    bikes: Query<(&Bike, &Pose)>,
    wheels: Query<&Pose, With<Wheel>>,
    finishes: Query<&Finish>,
) {
    if !run.in_control() {
        return;
    }
    let Some(finish) = finishes.iter().next() else {
        return;
    };
    for (bike, body) in &bikes {
        let wheel_positions = [bike.left_wheel, bike.right_wheel]
            .into_iter()
            .filter_map(|wheel| wheels.get(wheel).ok())
            .map(|pose| pose.translation);
        let reached = std::iter::once(body.translation)
            .chain(wheel_positions)
            .any(|point| finish.zone.contains(point));
        if reached {
            run.finished = true;
            if let Some(player) = active.0 {
                profiles.record_finish(player, level.id);
            }
            return;
        }
    }
}

/// Advances the clock, the crash delay, and the cues.
pub fn tick_run(time: Res<Time>, mut run: ResMut<Run>) {
    let dt = time.delta_secs();
    if !run.finished {
        run.elapsed += dt;
    }
    if let Some(left) = run.crash.as_mut() {
        *left -= dt;
    }
    if let Some((_, age)) = run.flip_cue.as_mut() {
        *age += dt;
    }
    if run.flip_cue.is_some_and(|(_, age)| age > CUE_SECONDS) {
        run.flip_cue = None;
    }
    if let Some(age) = run.checkpoint_cue.as_mut() {
        *age += dt;
    }
    if run.checkpoint_cue.is_some_and(|age| age > CUE_SECONDS) {
        run.checkpoint_cue = None;
    }
}
