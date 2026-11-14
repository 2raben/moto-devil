//! A side-scrolling camera that follows the bike and stays inside the level.

use bevy::camera::ScalingMode;
use bevy::prelude::*;

use crate::level::Level;
use crate::visuals::BikeVisual;

/// The minimum visible area, in world units.
pub const VIEW: Vec2 = Vec2::new(960.0, 540.0);

/// The view leads the bike slightly to the right, where the level continues.
const LOOK_AHEAD: Vec2 = Vec2::new(140.0, 60.0);
const FOLLOW_RATE: f32 = 6.0;
/// Larger jumps, such as a respawn, snap instead of gliding.
const SNAP_DISTANCE: f32 = 600.0;

/// Camera movement in `PostUpdate`, after the bike's drawing is placed.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CameraSystems;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera).add_systems(
            PostUpdate,
            follow_bike
                .in_set(CameraSystems)
                .before(TransformSystems::Propagate),
        );
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: VIEW.x,
                min_height: VIEW.y,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

/// Where the camera should centre for a bike at `bike`, keeping a view of
/// `half_view` inside the level where it fits and centring it where it does not.
pub fn camera_target(bounds: Rect, bike: Vec2, half_view: Vec2) -> Vec2 {
    keep_inside(bounds, bike + LOOK_AHEAD, half_view)
}

/// The camera centre nearest to `wanted` that keeps a view of `half_view`
/// inside `bounds`, centred on an axis where the view is larger.
pub fn keep_inside(bounds: Rect, wanted: Vec2, half_view: Vec2) -> Vec2 {
    let axis = |wanted: f32, min: f32, max: f32, half: f32| {
        if max - min <= 2.0 * half {
            0.5 * (min + max)
        } else {
            wanted.clamp(min + half, max - half)
        }
    };
    Vec2::new(
        axis(wanted.x, bounds.min.x, bounds.max.x, half_view.x),
        axis(wanted.y, bounds.min.y, bounds.max.y, half_view.y),
    )
}

fn follow_bike(
    time: Res<Time>,
    level: Option<Res<Level>>,
    bikes: Query<&Transform, (With<BikeVisual>, Without<Camera2d>)>,
    mut cameras: Query<(&mut Transform, &Projection), With<Camera2d>>,
) {
    let (Some(level), Some(bike)) = (level, bikes.iter().next()) else {
        return;
    };
    for (mut camera, projection) in &mut cameras {
        let half_view = match projection {
            Projection::Orthographic(ortho) => ortho.area.half_size(),
            _ => VIEW * 0.5,
        };
        let target = camera_target(level.bounds, bike.translation.truncate(), half_view);
        let current = camera.translation.truncate();
        let next = if current.distance(target) > SNAP_DISTANCE {
            target
        } else {
            let blend = 1.0 - (-FOLLOW_RATE * time.delta_secs()).exp();
            current.lerp(target, blend)
        };
        camera.translation = next.extend(camera.translation.z);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_view_inside_the_level() {
        let bounds = Rect::new(-100.0, -220.0, 2500.0, 600.0);
        let half = VIEW * 0.5;
        let at_start = camera_target(bounds, Vec2::new(100.0, 30.0), half);
        assert_eq!(at_start.x, bounds.min.x + half.x);
        assert_eq!(at_start.y, 30.0 + LOOK_AHEAD.y);
        let at_end = camera_target(bounds, Vec2::new(2450.0, 30.0), half);
        assert_eq!(at_end.x, bounds.max.x - half.x);
        let short = Rect::new(0.0, 0.0, 500.0, 300.0);
        assert_eq!(camera_target(short, Vec2::ZERO, half), short.center());
    }
}
