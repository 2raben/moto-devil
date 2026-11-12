//! Rapier inside the fixed step, and the poses the rest of the game reads.
//!
//! Rapier owns the `Transform` of every physics body. Game systems run in two
//! sets around Rapier's own [`PhysicsSet`]: [`GameSystems::Drive`] sets
//! velocities and colliders before the step, and [`GameSystems::React`] reads the
//! result after it. After each step the bodies' poses are copied into [`Pose`],
//! which is what game rules, tests, and presentation read. Presentation never
//! touches physics entities; it draws separate entities from [`Pose`] and
//! [`PreviousPose`]. See ADR 0004.

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

/// Downward acceleration in world units per second squared.
pub const GRAVITY: f32 = 1200.0;
/// Rapier's internal length scale; world units are pixels.
pub const PIXELS_PER_METER: f32 = 100.0;
/// Solver substeps per fixed step; more keeps fast wheels on thin ground.
pub const SUBSTEPS: usize = 4;

/// Collision group of level geometry.
pub const WORLD: Group = Group::GROUP_1;
/// Collision group of the bike; bike parts never collide with each other.
pub const BIKE: Group = Group::GROUP_2;

pub fn world_groups() -> CollisionGroups {
    CollisionGroups::new(WORLD, Group::ALL)
}

pub fn bike_groups() -> CollisionGroups {
    CollisionGroups::new(BIKE, WORLD)
}

/// The fixed-step game systems, before and after the physics step.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameSystems {
    /// Inputs, respawns, and traps: everything that changes bodies or colliders.
    Drive,
    /// Rules that read the step's result: contacts, crashes, triggers, finish.
    React,
}

/// Where an entity is in the simulation: a position and a counter-clockwise
/// angle in radians.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct Pose {
    pub translation: Vec2,
    pub angle: f32,
}

impl Pose {
    pub fn new(translation: Vec2, angle: f32) -> Self {
        Self { translation, angle }
    }

    pub fn at(translation: Vec2) -> Self {
        Self::new(translation, 0.0)
    }

    /// A point given in the entity's own frame, in world space.
    pub fn transform_point(&self, local: Vec2) -> Vec2 {
        self.translation + Vec2::from_angle(self.angle).rotate(local)
    }

    /// The direction the entity's local "up" points to.
    pub fn up(&self) -> Vec2 {
        Vec2::from_angle(self.angle).rotate(Vec2::Y)
    }

    pub fn to_transform(self) -> Transform {
        Transform::from_translation(self.translation.extend(0.0))
            .with_rotation(Quat::from_rotation_z(self.angle))
    }

    fn from_transform(transform: &Transform) -> Self {
        let (angle, _, _) = transform.rotation.to_euler(EulerRot::ZYX);
        Self::new(transform.translation.truncate(), angle)
    }
}

/// The pose before the latest fixed step, used to interpolate rendering.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PreviousPose(pub Pose);

/// A Rapier body whose [`Pose`] is copied from its transform after each step.
#[derive(Component, Debug)]
#[require(Pose, PreviousPose)]
pub struct TrackedBody;

/// Set on a body that was just moved by hand, so rendering does not slide it
/// from its old place.
#[derive(Component, Debug)]
pub struct Teleported;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        let dt = Time::<Fixed>::default().timestep().as_secs_f32();
        let initialization = RapierContextInitialization::InitializeDefaultRapierContext {
            integration_parameters: IntegrationParameters {
                length_unit: PIXELS_PER_METER,
                ..default()
            },
            rapier_configuration: RapierConfiguration {
                gravity: Vec2::NEG_Y * GRAVITY,
                ..RapierConfiguration::new(PIXELS_PER_METER)
            },
            broad_phase_optimization_strategy: default(),
        };
        app.insert_resource(TimestepMode::Fixed {
            dt,
            substeps: SUBSTEPS,
        })
        .add_plugins(
            RapierPhysicsPlugin::<NoUserData>::default()
                .with_custom_initialization(initialization)
                .in_fixed_schedule(),
        )
        .configure_sets(
            FixedUpdate,
            (
                GameSystems::Drive.before(PhysicsSet::SyncBackend),
                GameSystems::React.after(PhysicsSet::Writeback),
            ),
        );
    }
}

/// Keeps last step's pose so rendering can blend between the two. Runs first
/// in [`GameSystems::Drive`].
pub fn remember_poses(mut poses: Query<(&Pose, &mut PreviousPose)>) {
    for (pose, mut previous) in &mut poses {
        previous.0 = *pose;
    }
}

/// What [`read_body_poses`] reads and writes on each body.
type BodyPoses<'a> = (
    Entity,
    &'a Transform,
    &'a mut Pose,
    &'a mut PreviousPose,
    Has<Teleported>,
);

/// Copies the bodies' transforms into [`Pose`]. Runs first in
/// [`GameSystems::React`].
pub fn read_body_poses(mut commands: Commands, mut bodies: Query<BodyPoses, With<TrackedBody>>) {
    for (entity, transform, mut pose, mut previous, teleported) in &mut bodies {
        *pose = Pose::from_transform(transform);
        if teleported {
            previous.0 = *pose;
            commands.entity(entity).remove::<Teleported>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poses_rotate_counter_clockwise() {
        let pose = Pose::new(Vec2::new(10.0, 0.0), std::f32::consts::FRAC_PI_2);
        assert!(pose.transform_point(Vec2::X).distance(Vec2::new(10.0, 1.0)) < 1e-5);
        assert!(pose.up().distance(Vec2::NEG_X) < 1e-5);
        let round_trip = Pose::from_transform(&pose.to_transform());
        assert!((round_trip.angle - pose.angle).abs() < 1e-5);
    }
}
