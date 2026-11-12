//! Spawns a level's physics and rule entities: ground, solids, loops, spikes,
//! traps, the finish, and the bike. Every entity gets [`LevelEntity`] so the
//! whole course can be removed at once.

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::bike::{self, BikeTuning};
use crate::level::{Level, TrapKind};
use crate::physics::{Pose, PreviousPose, world_groups};
use crate::traps::{Finish, Hazard, HazardShape, LoopArc, LoopHalf, LoopState, Trap};

/// Belongs to the level that is being played.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct LevelEntity;

/// One per spawned level; presentation draws the static scenery when it appears.
#[derive(Component, Debug)]
pub struct LevelRoot;

/// Spikes that are always out.
#[derive(Component, Clone, Copy, Debug)]
pub struct StaticSpikes(pub crate::level::Spikes);

fn solid(collider: Collider, transform: Transform) -> impl Bundle {
    (
        LevelEntity,
        RigidBody::Fixed,
        collider,
        world_groups(),
        Friction::coefficient(1.0),
        transform,
    )
}

fn box_collider(rect: Rect) -> impl Bundle {
    let half = rect.half_size();
    solid(
        Collider::cuboid(half.x, half.y),
        Transform::from_translation(rect.center().extend(0.0)),
    )
}

/// Spawns everything a level needs to be played.
pub fn spawn_course(commands: &mut Commands, level: &Level, tuning: &BikeTuning) {
    commands.spawn((Name::new(level.name), LevelRoot, LevelEntity));
    commands.spawn((
        Name::new("Ground"),
        solid(
            Collider::polyline(level.ground.clone(), None),
            Transform::default(),
        ),
    ));
    for rect in &level.solids {
        commands.spawn((Name::new("Solid"), box_collider(*rect)));
    }
    for def in &level.loops {
        let owner = commands
            .spawn((
                Name::new("Loop"),
                LevelEntity,
                LoopState {
                    def: *def,
                    falling: false,
                },
            ))
            .id();
        for half in [LoopHalf::Rising, LoopHalf::Falling] {
            let mut arc = commands.spawn((
                Name::new("Loop track"),
                LoopArc { owner, half },
                solid(
                    Collider::polyline(half.points(def), None),
                    Transform::default(),
                ),
            ));
            if half == LoopHalf::Falling {
                arc.insert(ColliderDisabled);
            }
        }
    }
    for spikes in &level.spikes {
        commands.spawn((
            Name::new("Spikes"),
            LevelEntity,
            StaticSpikes(*spikes),
            Hazard(Some(HazardShape::Rect(spikes.hazard(1.0)))),
        ));
    }
    for def in &level.traps {
        let collider = match def.kind {
            TrapKind::DropSlab { slab } => Some(
                commands
                    .spawn((Name::new("Slab collider"), box_collider(slab)))
                    .id(),
            ),
            _ => None,
        };
        commands.spawn((Name::new("Trap"), LevelEntity, Trap::new(*def, collider)));
    }
    for def in &level.effects {
        commands.spawn((
            Name::new("Course effect"),
            LevelEntity,
            crate::effects::CourseEffect::new(*def),
        ));
    }
    let finish = Pose::at(level.finish.center());
    commands.spawn((
        Name::new("Finish"),
        LevelEntity,
        Finish {
            zone: level.finish,
            home: level.finish,
        },
        finish,
        PreviousPose(finish),
    ));
    bike::spawn_bike(commands, level.start, tuning, LevelEntity);
}
