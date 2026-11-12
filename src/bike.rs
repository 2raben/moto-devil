//! The bike: a body and two wheels joined by hinges, simulated by Rapier and
//! driven through [`Controls`].
//!
//! Gas pushes the bike along the ground while the rear wheel touches it, the
//! brake slows it, tilting turns the body, jumping pushes the whole bike up, and
//! turning around mirrors which wheel is the rear one. The assists from the pitch live here too: the bike rights itself
//! in the air when no tilt key is held, and it cannot lean past
//! [`BikeTuning::max_ground_lean`] while a wheel touches the ground.

use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::physics::{Pose, PreviousPose, Teleported, TrackedBody, bike_groups};
use crate::run::Run;

/// Player intent for the fixed step, written by the input layer.
///
/// `jump_pressed`, `turn_pressed`, and `restart` are latched until a fixed step
/// consumes them, so a press is never lost on a frame without a fixed step.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct Controls {
    /// Gas in `0.0..=1.0`.
    pub throttle: f32,
    /// Brake in `0.0..=1.0`.
    pub brake: f32,
    /// Tilt in `-1.0..=1.0`: negative tilts to the left (counter-clockwise),
    /// positive to the right (clockwise), whichever way the bike faces.
    pub tilt: f32,
    pub jump_pressed: bool,
    pub turn_pressed: bool,
    pub restart: bool,
}

/// Bike size and feel. These are play-test values: judge changes in the game.
#[derive(Resource, Clone, Copy, Debug)]
pub struct BikeTuning {
    pub wheel_radius: f32,
    /// Where the right wheel's hinge sits on the body; the left one is mirrored.
    pub wheel_offset: Vec2,
    /// Half width and half height of the body box.
    pub body_half_size: Vec2,
    /// The rider's head, seen from the body's centre with the bike facing right.
    pub head_offset: Vec2,
    pub head_radius: f32,
    pub body_mass: f32,
    pub wheel_mass: f32,
    pub wheel_friction: f32,
    /// Top speed under full gas, in world units per second.
    pub max_speed: f32,
    /// Acceleration under full gas along the ground, world units per second squared.
    pub drive_accel: f32,
    /// Deceleration under full brake.
    pub brake_accel: f32,
    /// Upward speed added by a jump.
    pub jump_speed: f32,
    pub coyote_time: f32,
    pub jump_buffer_time: f32,
    /// Spin speed while tilting in the air, radians per second.
    pub air_spin: f32,
    /// How quickly the bike reaches the wanted spin in the air.
    pub air_spin_accel: f32,
    /// Self-righting in the air without a tilt key: spin per radian of lean.
    pub air_righting: f32,
    /// Spin speed while tilting on the ground.
    pub ground_spin: f32,
    pub ground_spin_accel: f32,
    /// The bike never leans further than this from the ground while touching it.
    pub max_ground_lean: f32,
    /// How hard the bike is pushed back once it leans past the limit.
    pub lean_return: f32,
    /// A turn-around ends early once the bike is slower than this.
    pub turn_speed: f32,
    /// A turn-around takes at most this long.
    pub turn_time: f32,
    /// Seconds between a crash and the respawn.
    pub crash_delay: f32,
}

impl Default for BikeTuning {
    fn default() -> Self {
        Self {
            wheel_radius: 16.0,
            wheel_offset: Vec2::new(32.0, -14.0),
            body_half_size: Vec2::new(26.0, 7.0),
            head_offset: Vec2::new(6.0, 46.0),
            head_radius: 10.0,
            body_mass: 2.0,
            wheel_mass: 0.6,
            wheel_friction: 1.6,
            max_speed: 720.0,
            drive_accel: 900.0,
            brake_accel: 1400.0,
            jump_speed: 520.0,
            coyote_time: 0.1,
            jump_buffer_time: 0.12,
            air_spin: 8.0,
            air_spin_accel: 30.0,
            air_righting: 4.0,
            ground_spin: 3.5,
            ground_spin_accel: 25.0,
            max_ground_lean: 45f32.to_radians(),
            lean_return: 6.0,
            turn_speed: 60.0,
            turn_time: 0.45,
            crash_delay: 0.45,
        }
    }
}

/// The bike's body, which owns the two wheel entities.
#[derive(Component, Debug)]
#[require(BikeState)]
pub struct Bike {
    pub left_wheel: Entity,
    pub right_wheel: Entity,
}

/// One of the two wheels, named by its side when the bike faces right.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wheel {
    Left,
    Right,
}

impl Wheel {
    fn sign(self) -> f32 {
        match self {
            Self::Left => -1.0,
            Self::Right => 1.0,
        }
    }
}

/// What the bike is doing, updated every fixed step.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct BikeState {
    /// `1.0` facing right, `-1.0` facing left.
    pub facing: f32,
    /// Seconds into a turn-around, while one runs.
    pub turning: Option<f32>,
    pub left_contact: bool,
    pub right_contact: bool,
    /// The average ground normal under touching wheels; world up in the air.
    pub ground_normal: Vec2,
    /// Time left to jump after leaving the ground.
    pub coyote: f32,
    /// Time left for an early jump press to fire.
    pub jump_buffer: f32,
    /// Rotation since leaving the ground, in radians, counter-clockwise positive.
    pub air_turn: f32,
}

impl Default for BikeState {
    fn default() -> Self {
        Self {
            facing: 1.0,
            turning: None,
            left_contact: false,
            right_contact: false,
            ground_normal: Vec2::Y,
            coyote: 0.0,
            jump_buffer: 0.0,
            air_turn: 0.0,
        }
    }
}

impl BikeState {
    pub fn grounded(&self) -> bool {
        self.left_contact || self.right_contact
    }

    /// The rear wheel touches the ground; only then does gas push.
    pub fn rear_contact(&self) -> bool {
        if self.facing > 0.0 {
            self.left_contact
        } else {
            self.right_contact
        }
    }
}

/// A full turn counts as a flip when it is at least this close to 360°.
const FLIP_GRACE: f32 = 0.35;

/// How many whole flips a rotation of `air_turn` radians makes.
pub fn count_flips(air_turn: f32) -> u32 {
    ((air_turn.abs() + FLIP_GRACE) / TAU).floor() as u32
}

/// The signed angle from `from` to `to`, counter-clockwise positive.
pub fn signed_angle(from: Vec2, to: Vec2) -> f32 {
    from.perp_dot(to).atan2(from.dot(to))
}

fn move_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    current + (target - current).clamp(-max_delta, max_delta)
}

/// The spin the bike should have, given its lean from the reference normal.
/// `None` leaves the physics alone (on the ground with no tilt key, inside the
/// lean limit).
pub fn wanted_spin(tuning: &BikeTuning, grounded: bool, lean: f32, tilt: f32) -> Option<f32> {
    let tilting = tilt.abs() > 0.05;
    if !grounded {
        let spin = if tilting {
            -tilt * tuning.air_spin
        } else {
            let limit = 0.5 * tuning.air_spin;
            (-lean * tuning.air_righting).clamp(-limit, limit)
        };
        return Some(spin);
    }
    let max = tuning.max_ground_lean;
    let mut spin = tilting.then_some(-tilt * tuning.ground_spin);
    if lean > max {
        let back = -(lean - max) * tuning.lean_return;
        spin = Some(spin.map_or(back, |spin| spin.min(back)));
    } else if lean < -max {
        let back = (-max - lean) * tuning.lean_return;
        spin = Some(spin.map_or(back, |spin| spin.max(back)));
    }
    spin
}

/// The body's pose for a bike standing level with its wheels on `ground`.
pub fn body_pose_at(ground: Vec2, tuning: &BikeTuning) -> Pose {
    Pose::at(ground + Vec2::new(0.0, tuning.wheel_radius - tuning.wheel_offset.y))
}

/// Where a wheel's centre sits for a given body pose.
pub fn wheel_pose(body: Pose, wheel: Wheel, tuning: &BikeTuning) -> Pose {
    let local = Vec2::new(wheel.sign() * tuning.wheel_offset.x, tuning.wheel_offset.y);
    Pose::new(body.transform_point(local), body.angle)
}

/// The centre of the rider's head.
pub fn head_position(body: &Pose, facing: f32, tuning: &BikeTuning) -> Vec2 {
    body.transform_point(Vec2::new(
        facing * tuning.head_offset.x,
        tuning.head_offset.y,
    ))
}

/// Circles that cover the bike for hazard checks: both wheels, the head, and
/// three along the body.
pub fn hull(body: &Pose, facing: f32, wheels: [Vec2; 2], tuning: &BikeTuning) -> [(Vec2, f32); 6] {
    let half = tuning.body_half_size;
    let along = |x: f32| body.transform_point(Vec2::new(x, 0.0));
    [
        (wheels[0], tuning.wheel_radius),
        (wheels[1], tuning.wheel_radius),
        (head_position(body, facing, tuning), tuning.head_radius),
        (along(-0.7 * half.x), half.y),
        (along(0.0), half.y),
        (along(0.7 * half.x), half.y),
    ]
}

fn body_bundle(pose: Pose, tuning: &BikeTuning) -> impl Bundle {
    (
        Name::new("Bike"),
        TrackedBody,
        RigidBody::Dynamic,
        Collider::cuboid(tuning.body_half_size.x, tuning.body_half_size.y),
        ColliderMassProperties::Mass(tuning.body_mass),
        bike_groups(),
        Friction::coefficient(0.6),
        Restitution::coefficient(0.0),
        Velocity::zero(),
        Damping {
            linear_damping: 0.05,
            angular_damping: 0.3,
        },
        Sleeping::disabled(),
        Ccd::enabled(),
        (pose.to_transform(), pose, PreviousPose(pose)),
    )
}

fn wheel_bundle(body: Entity, body_pose: Pose, wheel: Wheel, tuning: &BikeTuning) -> impl Bundle {
    let pose = wheel_pose(body_pose, wheel, tuning);
    let anchor = Vec2::new(wheel.sign() * tuning.wheel_offset.x, tuning.wheel_offset.y);
    (
        Name::new("Wheel"),
        wheel,
        TrackedBody,
        RigidBody::Dynamic,
        Collider::ball(tuning.wheel_radius),
        ColliderMassProperties::Mass(tuning.wheel_mass),
        bike_groups(),
        Friction {
            coefficient: tuning.wheel_friction,
            combine_rule: CoefficientCombineRule::Max,
        },
        Restitution::coefficient(0.0),
        Velocity::zero(),
        Sleeping::disabled(),
        Ccd::enabled(),
        ImpulseJoint::new(
            body,
            RevoluteJointBuilder::new()
                .local_anchor1(anchor)
                .local_anchor2(Vec2::ZERO),
        ),
        (pose.to_transform(), pose, PreviousPose(pose)),
    )
}

/// Spawns a bike standing on `ground`, facing right. `marker` is added to all
/// three entities. Returns the body.
pub fn spawn_bike(
    commands: &mut Commands,
    ground: Vec2,
    tuning: &BikeTuning,
    marker: impl Bundle + Clone,
) -> Entity {
    let pose = body_pose_at(ground, tuning);
    let body = commands
        .spawn((body_bundle(pose, tuning), marker.clone()))
        .id();
    let left_wheel = commands
        .spawn((
            wheel_bundle(body, pose, Wheel::Left, tuning),
            marker.clone(),
        ))
        .id();
    let right_wheel = commands
        .spawn((wheel_bundle(body, pose, Wheel::Right, tuning), marker))
        .id();
    commands.entity(body).insert(Bike {
        left_wheel,
        right_wheel,
    });
    body
}

/// The parts of a physics body that a teleport resets.
pub type BodyParts<'a> = (
    &'a mut Transform,
    &'a mut Velocity,
    &'a mut Pose,
    &'a mut PreviousPose,
);

/// Puts a bike back on `ground`, level, at rest, and facing right.
pub fn place_bike(
    commands: &mut Commands,
    body: Entity,
    bike: &Bike,
    state: &mut BikeState,
    ground: Vec2,
    tuning: &BikeTuning,
    bodies: &mut Query<BodyParts, With<TrackedBody>>,
) {
    let body_pose = body_pose_at(ground, tuning);
    let parts = [
        (body, body_pose),
        (bike.left_wheel, wheel_pose(body_pose, Wheel::Left, tuning)),
        (
            bike.right_wheel,
            wheel_pose(body_pose, Wheel::Right, tuning),
        ),
    ];
    for (entity, pose) in parts {
        if let Ok((mut transform, mut velocity, mut current, mut previous)) = bodies.get_mut(entity)
        {
            *transform = pose.to_transform();
            *velocity = Velocity::zero();
            *current = pose;
            previous.0 = pose;
            commands.entity(entity).insert(Teleported);
        }
    }
    *state = BikeState::default();
}

/// Applies gas, brake, tilt, jump, and turn-around for one fixed step.
pub fn drive_bike(
    time: Res<Time>,
    tuning: Res<BikeTuning>,
    run: Res<Run>,
    mut controls: ResMut<Controls>,
    mut bikes: Query<(&Bike, &mut BikeState, &Pose, &mut Velocity), Without<Wheel>>,
    mut wheels: Query<(&mut Velocity, &Pose), With<Wheel>>,
) {
    let dt = time.delta_secs();
    let jump_pressed = std::mem::take(&mut controls.jump_pressed);
    let turn_pressed = std::mem::take(&mut controls.turn_pressed);
    let in_control = run.in_control();
    let (mut throttle, mut brake, tilt) = if in_control {
        (
            controls.throttle.clamp(0.0, 1.0),
            controls.brake.clamp(0.0, 1.0),
            controls.tilt.clamp(-1.0, 1.0),
        )
    } else if run.finished {
        (0.0, 1.0, 0.0)
    } else {
        (0.0, 0.0, 0.0)
    };

    for (bike, mut state, pose, mut body_velocity) in &mut bikes {
        let grounded = state.grounded();
        state.coyote = if grounded {
            tuning.coyote_time
        } else {
            (state.coyote - dt).max(0.0)
        };
        state.jump_buffer = if jump_pressed && in_control {
            tuning.jump_buffer_time
        } else {
            (state.jump_buffer - dt).max(0.0)
        };

        if turn_pressed && in_control && grounded && state.turning.is_none() {
            state.turning = Some(0.0);
        }
        if let Some(elapsed) = state.turning {
            let elapsed = elapsed + dt;
            throttle = 0.0;
            brake = 1.0;
            if body_velocity.linear.length() < tuning.turn_speed || elapsed >= tuning.turn_time {
                state.facing = -state.facing;
                state.turning = None;
            } else {
                state.turning = Some(elapsed);
            }
        }

        let jump = state.jump_buffer > 0.0 && state.coyote > 0.0;
        let jump_velocity = jump.then(|| body_velocity.linear.y.max(0.0) + tuning.jump_speed);
        if jump {
            state.jump_buffer = 0.0;
            state.coyote = 0.0;
        }

        // Gas and brake push the whole bike along the ground, so speed builds up
        // predictably instead of depending on wheel slip.
        let forward = -state.facing * state.ground_normal.perp();
        let along = body_velocity.linear.dot(forward);
        let mut push = 0.0;
        if brake > 0.0 && grounded {
            let slow = tuning.brake_accel * brake * dt;
            push = -along.signum() * slow.min(along.abs());
        } else if throttle > 0.0 && state.rear_contact() {
            let wanted = tuning.max_speed * throttle;
            push = (wanted - along).clamp(0.0, tuning.drive_accel * dt);
        }
        let delta = push * forward;

        // Tilting turns the whole bike as one piece: the wheels get the speed that
        // the turn gives their hinge points, so the joints do not undo it.
        let mut spin_change = 0.0;
        if in_control || run.finished {
            let lean = signed_angle(state.ground_normal, pose.up());
            let accel = if grounded {
                tuning.ground_spin_accel
            } else {
                tuning.air_spin_accel
            };
            if let Some(spin) = wanted_spin(&tuning, grounded && !jump, lean, tilt) {
                let turned = move_towards(body_velocity.angular, spin, accel * dt);
                spin_change = turned - body_velocity.angular;
                body_velocity.angular = turned;
            }
        }

        // On the ground the bike turns around the wheel that stays down (a
        // wheelie pivots on the rear wheel); in the air around its middle.
        let wheel_positions = [bike.left_wheel, bike.right_wheel].map(|wheel| {
            wheels
                .get(wheel)
                .map_or(pose.translation, |(_, wheel_pose)| wheel_pose.translation)
        });
        let pivot = match (grounded, spin_change > 0.0) {
            (false, _) => pose.translation,
            (true, true) => wheel_positions[0],
            (true, false) => wheel_positions[1],
        };
        let swing = |at: Vec2| spin_change * (at - pivot).perp();

        body_velocity.linear += delta + swing(pose.translation);
        if let Some(vy) = jump_velocity {
            body_velocity.linear.y = vy;
        }
        for (wheel, at) in [bike.left_wheel, bike.right_wheel]
            .into_iter()
            .zip(wheel_positions)
        {
            let Ok((mut velocity, _)) = wheels.get_mut(wheel) else {
                continue;
            };
            velocity.linear += delta + swing(at);
            if let Some(vy) = jump_velocity {
                velocity.linear.y = vy;
            }
        }
    }
}

/// Reads wheel contacts from Rapier, updates the ground normal, and counts flips
/// on landing.
pub fn sense_ground(
    time: Res<Time>,
    rapier: ReadRapierContext,
    mut run: ResMut<Run>,
    mut bikes: Query<(&Bike, &mut BikeState, &Velocity)>,
) {
    let Ok(context) = rapier.single() else {
        return;
    };
    let dt = time.delta_secs();
    for (bike, mut state, velocity) in &mut bikes {
        let mut normals = Vec2::ZERO;
        let mut touching = |wheel: Entity| {
            let mut touches = false;
            for pair in context.contact_pairs_with(wheel) {
                if !pair.has_any_active_contact() {
                    continue;
                }
                // Manifold normals point from the first collider to the second;
                // turn them so they point from the ground to the wheel.
                let toward_wheel = if pair.collider1() == Some(wheel) {
                    -1.0
                } else {
                    1.0
                };
                for manifold in pair.manifolds() {
                    if manifold.num_solver_contacts() > 0 {
                        normals += toward_wheel * manifold.normal();
                        touches = true;
                    }
                }
            }
            touches
        };
        state.left_contact = touching(bike.left_wheel);
        state.right_contact = touching(bike.right_wheel);
        state.ground_normal = normals.try_normalize().unwrap_or(Vec2::Y);

        if !state.grounded() {
            state.air_turn += velocity.angular * dt;
            continue;
        }
        let flips = count_flips(state.air_turn);
        state.air_turn = 0.0;
        if flips > 0 && run.in_control() {
            run.flips += flips;
            run.flip_cue = Some((flips, 0.0));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, PI};

    use super::*;

    #[test]
    fn flips_need_almost_a_full_turn() {
        assert_eq!(count_flips(PI), 0);
        assert_eq!(count_flips(TAU - 0.5), 0);
        assert_eq!(count_flips(TAU - 0.3), 1);
        assert_eq!(count_flips(-TAU), 1, "back flips count too");
        assert_eq!(count_flips(2.0 * TAU + 0.1), 2);
    }

    #[test]
    fn signed_angles_are_counter_clockwise() {
        assert!((signed_angle(Vec2::Y, Vec2::NEG_X) - FRAC_PI_2).abs() < 1e-5);
        assert!((signed_angle(Vec2::Y, Vec2::X) + FRAC_PI_2).abs() < 1e-5);
    }

    #[test]
    fn the_air_assist_rights_the_bike_only_without_tilt() {
        let tuning = BikeTuning::default();
        let leaning_left = 0.5;
        let righting = wanted_spin(&tuning, false, leaning_left, 0.0).unwrap();
        assert!(righting < 0.0, "turns back clockwise");
        let flipping = wanted_spin(&tuning, false, leaning_left, -1.0).unwrap();
        assert_eq!(
            flipping, tuning.air_spin,
            "tilting left keeps spinning left"
        );
    }

    #[test]
    fn the_ground_limit_stops_a_wheelie_from_going_over() {
        let tuning = BikeTuning::default();
        let max = tuning.max_ground_lean;
        assert_eq!(
            wanted_spin(&tuning, true, 0.0, 0.0),
            None,
            "no input, no assist"
        );
        assert!(wanted_spin(&tuning, true, 0.0, -1.0).unwrap() > 0.0);
        let past_limit = wanted_spin(&tuning, true, max + 0.2, -1.0).unwrap();
        assert!(past_limit < 0.0, "pushed back even while tilting further");
        assert!(wanted_spin(&tuning, true, -max - 0.2, 0.0).unwrap() > 0.0);
    }

    #[test]
    fn wheels_and_head_follow_the_body() {
        let tuning = BikeTuning::default();
        let body = body_pose_at(Vec2::new(100.0, 0.0), &tuning);
        let left = wheel_pose(body, Wheel::Left, &tuning);
        assert_eq!(left.translation, Vec2::new(68.0, tuning.wheel_radius));
        let facing_right = head_position(&body, 1.0, &tuning);
        let facing_left = head_position(&body, -1.0, &tuning);
        assert!(facing_right.x > facing_left.x);
        assert!(facing_right.y > body.translation.y);
    }
}
