//! Drawing: the scenery of the loaded level, the bike, traps, flags, and
//! gravestones, and interpolation of simulated poses into transforms.
//!
//! Simulation entities carry no sprites, and physics entities are never moved by
//! this module. Each drawn thing is a separate entity with [`VisualOf`] pointing
//! at what it draws, so it disappears together with that entity. See ADR 0004.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::bike::{Bike, BikeState, BikeTuning, Wheel};
use crate::course::{LevelRoot, StaticSpikes};
use crate::level::{Level, SPIKE_HEIGHT, SPIKE_WIDTH, Spikes, TrapKind};
use crate::physics::{Pose, PreviousPose};
use crate::progress::{ActivePlayer, Grave, Profiles};
use crate::run::Run;
use crate::traps::{Finish, TNT_BLAST, TNT_FUSE, Trap};

pub mod palette {
    use bevy::prelude::Color;

    pub const SKY: Color = Color::srgb(0.55, 0.76, 0.93);
    pub const GROUND: Color = Color::srgb(0.45, 0.31, 0.21);
    pub const GRASS: Color = Color::srgb(0.38, 0.7, 0.3);
    pub const ROCK: Color = Color::srgb(0.42, 0.42, 0.46);
    pub const TRACK: Color = Color::srgb(0.3, 0.3, 0.34);
    pub const SPIKE: Color = Color::srgb(0.88, 0.2, 0.2);
    pub const SLAB: Color = Color::srgb(0.6, 0.46, 0.33);
    pub const TNT: Color = Color::srgb(0.85, 0.15, 0.12);
    pub const BLAST: Color = Color::srgba(1.0, 0.6, 0.1, 0.75);
    pub const POLE: Color = Color::srgb(0.92, 0.92, 0.94);
    pub const FINISH: Color = Color::srgb(0.2, 0.62, 0.27);
    pub const CHECKPOINT: Color = Color::srgb(0.91, 0.35, 0.05);
    pub const FRAME: Color = Color::srgb(0.8, 0.12, 0.12);
    pub const TYRE: Color = Color::srgb(0.12, 0.12, 0.14);
    pub const HUB: Color = Color::srgb(0.7, 0.7, 0.74);
    pub const RIDER: Color = Color::srgb(0.1, 0.44, 0.76);
    pub const HELMET: Color = Color::srgb(0.98, 0.98, 0.98);
    pub const HORN: Color = Color::srgb(0.85, 0.1, 0.1);
    pub const CRASH: Color = Color::srgb(1.0, 0.3, 0.3);
    pub const STONE: Color = Color::srgb(0.78, 0.79, 0.82);
    pub const OWN_STONE: Color = Color::srgb(0.6, 0.78, 0.98);
    pub const TEXT_DARK: Color = Color::srgb(0.1, 0.1, 0.12);
    pub const OWN_TEXT: Color = Color::srgb(0.08, 0.35, 0.7);
    pub const PANEL: Color = Color::srgba(0.05, 0.08, 0.12, 0.8);
    pub const TEXT: Color = Color::srgb(0.95, 0.96, 0.98);
    pub const MUTED: Color = Color::srgb(0.85, 0.88, 0.92);
    pub const HIGHLIGHT: Color = Color::srgb(0.16, 0.45, 0.8);
}

/// Draw order, back to front.
mod layer {
    /// Spikes sit behind the ground so pop-up spikes hide inside it.
    pub const SPIKES: f32 = -1.0;
    pub const GROUND: f32 = 0.0;
    pub const SCENERY: f32 = 1.0;
    pub const GRAVE: f32 = 2.0;
    pub const TRAP: f32 = 3.0;
    pub const WHEEL: f32 = 4.0;
    pub const BIKE: f32 = 5.0;
}

const GRASS_DEPTH: f32 = 6.0;
const LOOP_THICKNESS: f32 = 14.0;

/// Draws on behalf of a simulation entity and is despawned with it.
#[derive(Component, Debug)]
#[relationship(relationship_target = Visuals)]
pub struct VisualOf(pub Entity);

/// The drawings that belong to a simulation entity.
#[derive(Component, Debug, Default)]
#[relationship_target(relationship = VisualOf, linked_spawn)]
pub struct Visuals(Vec<Entity>);

/// Placed every frame at `base` plus the interpolated pose of the entity it
/// draws.
#[derive(Component, Debug)]
struct Follow {
    base: Vec2,
    z: f32,
}

/// The drawing that follows the bike's body; the camera follows it.
#[derive(Component, Debug)]
pub struct BikeVisual;

/// Mirrored to show which way the bike faces.
#[derive(Component, Debug)]
struct Mirror;

/// The rider's helmet, tinted on a crash.
#[derive(Component, Debug)]
struct Helmet;

#[derive(Component, Debug)]
struct GraveVisual;

#[derive(Component, Debug)]
struct TntCrate;

#[derive(Component, Debug)]
struct TntBlast;

pub struct VisualsPlugin;

impl Plugin for VisualsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(palette::SKY))
            .add_systems(
                Update,
                (
                    draw_scenery,
                    draw_static_spikes,
                    draw_traps,
                    draw_finish,
                    draw_bike,
                    draw_wheels,
                    draw_graves.run_if(
                        resource_changed::<Profiles>.or_else(any_match_filter::<Added<LevelRoot>>),
                    ),
                ),
            )
            .add_systems(
                PostUpdate,
                (follow_poses, face_bike, tint_crash, animate_tnt)
                    .before(TransformSystems::Propagate)
                    .before(crate::camera::CameraSystems),
            );
    }
}

fn mesh_from(positions: Vec<[f32; 3]>, indices: Vec<u32>) -> Mesh {
    let count = positions.len();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; count])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; count])
    .with_inserted_indices(Indices::U32(indices))
}

/// Quads from each sloped or flat ground segment down to `floor`, and a grass
/// strip along the top.
fn ground_meshes(points: &[Vec2], floor: f32) -> (Mesh, Mesh) {
    let mut fill = (Vec::new(), Vec::new());
    let mut grass = (Vec::new(), Vec::new());
    let quad = |(positions, indices): &mut (Vec<[f32; 3]>, Vec<u32>), corners: [Vec2; 4]| {
        let base = positions.len() as u32;
        positions.extend(corners.map(|c| [c.x, c.y, 0.0]));
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    };
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if b.x - a.x <= f32::EPSILON {
            continue;
        }
        quad(
            &mut fill,
            [Vec2::new(a.x, floor), Vec2::new(b.x, floor), b, a],
        );
        let down = Vec2::new(0.0, GRASS_DEPTH);
        quad(&mut grass, [a - down, b - down, b, a]);
    }
    (mesh_from(fill.0, fill.1), mesh_from(grass.0, grass.1))
}

/// A row of spike triangles with its bottom-left corner at the origin.
fn spikes_mesh(spikes: &Spikes) -> Mesh {
    let mut positions = Vec::new();
    for i in 0..spikes.count {
        let left = i as f32 * SPIKE_WIDTH;
        positions.extend([
            [left, 0.0, 0.0],
            [left + SPIKE_WIDTH, 0.0, 0.0],
            [left + 0.5 * SPIKE_WIDTH, SPIKE_HEIGHT, 0.0],
        ]);
    }
    let indices = (0..positions.len() as u32).collect();
    mesh_from(positions, indices)
}

fn sprite(color: Color, size: Vec2, center: Vec2, z: f32) -> impl Bundle {
    (
        Sprite::from_color(color, size),
        Transform::from_translation(center.extend(z)),
    )
}

/// A thin bar from `a` to `b`.
fn bar(color: Color, a: Vec2, b: Vec2, width: f32, z: f32) -> impl Bundle {
    let along = b - a;
    (
        Sprite::from_color(color, Vec2::new(along.length(), width)),
        Transform::from_translation(((a + b) * 0.5).extend(z))
            .with_rotation(Quat::from_rotation_z(along.to_angle())),
    )
}

fn label(text: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text2d::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

/// A flag on a pole standing at the origin.
fn flag(parent: &mut ChildSpawnerCommands, color: Color, height: f32) {
    parent.spawn(sprite(
        palette::POLE,
        Vec2::new(5.0, height),
        Vec2::new(0.0, 0.5 * height),
        0.0,
    ));
    parent.spawn(sprite(
        color,
        Vec2::new(40.0, 26.0),
        Vec2::new(22.0, height - 15.0),
        0.1,
    ));
}

fn draw_scenery(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    level: Option<Res<Level>>,
    roots: Query<Entity, Added<LevelRoot>>,
) {
    let Some(level) = level else {
        return;
    };
    for root in &roots {
        let floor = level.bounds.min.y - 600.0;
        let (fill, grass) = ground_meshes(&level.ground, floor);
        commands.spawn((
            VisualOf(root),
            Mesh2d(meshes.add(fill)),
            MeshMaterial2d(materials.add(palette::GROUND)),
            Transform::from_xyz(0.0, 0.0, layer::GROUND),
        ));
        commands.spawn((
            VisualOf(root),
            Mesh2d(meshes.add(grass)),
            MeshMaterial2d(materials.add(palette::GRASS)),
            Transform::from_xyz(0.0, 0.0, layer::GROUND + 0.1),
        ));
        for solid in &level.solids {
            commands.spawn((
                VisualOf(root),
                sprite(palette::ROCK, solid.size(), solid.center(), layer::SCENERY),
            ));
        }
        for track in &level.loops {
            commands.spawn((
                VisualOf(root),
                Mesh2d(meshes.add(Annulus::new(track.radius, track.radius + LOOP_THICKNESS))),
                MeshMaterial2d(materials.add(palette::TRACK)),
                Transform::from_translation(track.center.extend(layer::SCENERY)),
            ));
        }
        for checkpoint in &level.checkpoints {
            commands
                .spawn((
                    VisualOf(root),
                    Transform::from_translation(
                        Vec2::new(checkpoint.zone.center().x, checkpoint.zone.min.y)
                            .extend(layer::SCENERY),
                    ),
                    Visibility::default(),
                ))
                .with_children(|parent| flag(parent, palette::CHECKPOINT, 90.0));
        }
    }
}

fn draw_static_spikes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    spikes: Query<(Entity, &StaticSpikes), Added<StaticSpikes>>,
) {
    for (entity, StaticSpikes(row)) in &spikes {
        commands.spawn((
            VisualOf(entity),
            Mesh2d(meshes.add(spikes_mesh(row))),
            MeshMaterial2d(materials.add(palette::SPIKE)),
            Transform::from_xyz(row.left, row.base, layer::SPIKES),
        ));
    }
}

fn draw_traps(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    traps: Query<(Entity, &Trap), Added<Trap>>,
) {
    for (entity, trap) in &traps {
        match trap.def.kind {
            TrapKind::DropSlab { slab } => {
                commands.spawn((
                    VisualOf(entity),
                    Follow {
                        base: slab.center(),
                        z: layer::TRAP,
                    },
                    Sprite::from_color(palette::SLAB, slab.size()),
                    Transform::default(),
                ));
            }
            TrapKind::PopUpSpikes { spikes } => {
                commands.spawn((
                    VisualOf(entity),
                    Follow {
                        base: Vec2::new(spikes.left, spikes.base),
                        z: layer::SPIKES,
                    },
                    Mesh2d(meshes.add(spikes_mesh(&spikes))),
                    MeshMaterial2d(materials.add(palette::SPIKE)),
                    Transform::default(),
                ));
            }
            TrapKind::FallingPillar { pillar, .. } => {
                commands.spawn((
                    VisualOf(entity),
                    Follow {
                        base: pillar.center(),
                        z: layer::TRAP,
                    },
                    Sprite::from_color(palette::ROCK, pillar.size()),
                    Transform::default(),
                ));
            }
            TrapKind::Tnt {
                crate_box,
                blast_radius,
            } => {
                commands
                    .spawn((
                        VisualOf(entity),
                        TntCrate,
                        sprite(
                            palette::TNT,
                            crate_box.size(),
                            crate_box.center(),
                            layer::TRAP,
                        ),
                        Visibility::default(),
                    ))
                    .with_child((
                        label("TNT", 18.0, palette::TEXT),
                        Transform::from_xyz(0.0, 0.0, 0.1),
                    ));
                commands.spawn((
                    VisualOf(entity),
                    TntBlast,
                    Mesh2d(meshes.add(Circle::new(blast_radius))),
                    MeshMaterial2d(materials.add(palette::BLAST)),
                    Transform::from_translation(crate_box.center().extend(layer::BIKE + 1.0)),
                    Visibility::Hidden,
                ));
            }
            TrapKind::JumpingFinish { .. } => {}
        }
    }
}

fn draw_finish(mut commands: Commands, finishes: Query<(Entity, &Finish), Added<Finish>>) {
    for (entity, finish) in &finishes {
        commands
            .spawn((
                VisualOf(entity),
                Follow {
                    base: Vec2::new(0.0, -0.5 * finish.home.height()),
                    z: layer::SCENERY,
                },
                Transform::default(),
                Visibility::default(),
            ))
            .with_children(|parent| {
                flag(parent, palette::FINISH, 120.0);
                parent.spawn((
                    label("ZIEL", 16.0, palette::TEXT),
                    Transform::from_xyz(22.0, 105.0, 0.2),
                ));
            });
    }
}

fn draw_bike(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    tuning: Res<BikeTuning>,
    bikes: Query<Entity, Added<Bike>>,
) {
    let head = tuning.head_offset;
    let wheel = tuning.wheel_offset;
    for entity in &bikes {
        let helmet = meshes.add(Circle::new(tuning.head_radius));
        let horn = meshes.add(Triangle2d::new(
            Vec2::new(-3.0, 0.0),
            Vec2::new(3.0, 0.0),
            Vec2::new(1.0, 9.0),
        ));
        let helmet_material = materials.add(palette::HELMET);
        let horn_material = materials.add(palette::HORN);
        let root = commands
            .spawn((
                VisualOf(entity),
                BikeVisual,
                Follow {
                    base: Vec2::ZERO,
                    z: layer::BIKE,
                },
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        commands
            .spawn((
                Mirror,
                ChildOf(root),
                Transform::default(),
                Visibility::default(),
            ))
            .with_children(|parts| {
                // Frame: a bar between the wheels, the fork, the tank, and the seat.
                parts.spawn(bar(
                    palette::FRAME,
                    Vec2::new(-wheel.x, wheel.y),
                    Vec2::new(-4.0, 6.0),
                    6.0,
                    0.0,
                ));
                parts.spawn(bar(
                    palette::FRAME,
                    Vec2::new(-4.0, 6.0),
                    Vec2::new(22.0, 6.0),
                    8.0,
                    0.0,
                ));
                parts.spawn(bar(
                    palette::HUB,
                    Vec2::new(wheel.x, wheel.y),
                    Vec2::new(24.0, 18.0),
                    4.0,
                    0.05,
                ));
                parts.spawn(sprite(
                    palette::FRAME,
                    Vec2::new(18.0, 10.0),
                    Vec2::new(10.0, 12.0),
                    0.1,
                ));
                parts.spawn(sprite(
                    palette::TYRE,
                    Vec2::new(22.0, 5.0),
                    Vec2::new(-10.0, 13.0),
                    0.1,
                ));
                parts.spawn(bar(
                    palette::TYRE,
                    Vec2::new(20.0, 20.0),
                    Vec2::new(28.0, 20.0),
                    3.0,
                    0.1,
                ));
                // Rider: leg, body, arm, and a helmet with devil horns.
                parts.spawn(bar(
                    palette::RIDER,
                    Vec2::new(-8.0, 16.0),
                    Vec2::new(6.0, 4.0),
                    6.0,
                    0.2,
                ));
                parts.spawn(bar(
                    palette::RIDER,
                    Vec2::new(-8.0, 16.0),
                    Vec2::new(head.x - 2.0, head.y - 12.0),
                    9.0,
                    0.2,
                ));
                parts.spawn(bar(
                    palette::RIDER,
                    Vec2::new(head.x - 3.0, head.y - 16.0),
                    Vec2::new(22.0, 20.0),
                    4.0,
                    0.3,
                ));
                parts.spawn((
                    Helmet,
                    Mesh2d(helmet),
                    MeshMaterial2d(helmet_material),
                    Transform::from_translation(head.extend(0.4)),
                ));
                for side in [-1.0, 1.0] {
                    parts.spawn((
                        Mesh2d(horn.clone()),
                        MeshMaterial2d(horn_material.clone()),
                        Transform::from_translation(
                            (head + Vec2::new(side * 5.0, 6.0)).extend(0.35),
                        )
                        .with_rotation(Quat::from_rotation_z(-side * 0.35)),
                    ));
                }
            });
    }
}

fn draw_wheels(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    tuning: Res<BikeTuning>,
    wheels: Query<Entity, Added<Wheel>>,
) {
    let radius = tuning.wheel_radius;
    for entity in &wheels {
        let tyre = (
            Mesh2d(meshes.add(Circle::new(radius))),
            MeshMaterial2d(materials.add(palette::TYRE)),
        );
        let hub = (
            Mesh2d(meshes.add(Circle::new(0.55 * radius))),
            MeshMaterial2d(materials.add(palette::HUB)),
        );
        commands
            .spawn((
                VisualOf(entity),
                Follow {
                    base: Vec2::ZERO,
                    z: layer::WHEEL,
                },
                tyre,
                Transform::default(),
            ))
            .with_children(|parent| {
                parent.spawn((hub, Transform::from_xyz(0.0, 0.0, 0.1)));
                parent.spawn(sprite(
                    palette::TYRE,
                    Vec2::new(3.0, 1.1 * radius),
                    Vec2::ZERO,
                    0.2,
                ));
                parent.spawn((
                    Sprite::from_color(palette::TYRE, Vec2::new(3.0, 1.1 * radius)),
                    Transform::from_xyz(0.0, 0.0, 0.2)
                        .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                ));
            });
    }
}

/// Redraws every player's gravestone in the loaded level.
#[allow(clippy::too_many_arguments)]
fn draw_graves(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    level: Option<Res<Level>>,
    profiles: Res<Profiles>,
    active: Res<ActivePlayer>,
    roots: Query<Entity, With<LevelRoot>>,
    old: Query<Entity, With<GraveVisual>>,
) {
    for entity in &old {
        commands.entity(entity).despawn();
    }
    let (Some(level), Ok(root)) = (level, roots.single()) else {
        return;
    };
    let mut graves: Vec<(f32, usize, bool)> = profiles
        .graves(level.id)
        .map(|(player, grave)| match grave {
            Grave::Crash(x) => (x, player, false),
            Grave::Finished => (level.finish.max.x + 30.0, player, true),
        })
        .collect();
    graves.sort_by(|a, b| a.0.total_cmp(&b.0));

    let top = meshes.add(Circle::new(12.0));
    let stone = materials.add(palette::STONE);
    let own_stone = materials.add(palette::OWN_STONE);
    // Stones closer than this share a spot: their names stack upwards.
    const CROWDED: f32 = 50.0;
    let mut previous_x = f32::NEG_INFINITY;
    let mut stack = 0;
    let mut finished_count = 0;
    for (x, player, finished) in graves {
        let x = if finished {
            finished_count += 1;
            x + (finished_count - 1) as f32 * 34.0
        } else {
            x
        };
        stack = if x - previous_x < CROWDED {
            stack + 1
        } else {
            0
        };
        previous_x = x;
        let ground = level.ground_height(x).unwrap_or(level.start.y);
        let own = active.0 == Some(player);
        let (material, text) = if own {
            (own_stone.clone(), palette::OWN_TEXT)
        } else {
            (stone.clone(), palette::TEXT_DARK)
        };
        let color = if own {
            palette::OWN_STONE
        } else {
            palette::STONE
        };
        let name = &profiles.players[player].name;
        let caption = if finished {
            format!("{name} ✓")
        } else {
            name.clone()
        };
        commands
            .spawn((
                VisualOf(root),
                GraveVisual,
                Transform::from_xyz(x, ground, layer::GRAVE),
                Visibility::default(),
            ))
            .with_children(|parent| {
                parent.spawn(sprite(
                    color,
                    Vec2::new(24.0, 20.0),
                    Vec2::new(0.0, 10.0),
                    0.0,
                ));
                parent.spawn((
                    Mesh2d(top.clone()),
                    MeshMaterial2d(material),
                    Transform::from_xyz(0.0, 20.0, 0.0),
                ));
                parent.spawn(sprite(
                    palette::TEXT_DARK,
                    Vec2::new(3.0, 16.0),
                    Vec2::new(0.0, 18.0),
                    0.1,
                ));
                parent.spawn(sprite(
                    palette::TEXT_DARK,
                    Vec2::new(11.0, 3.0),
                    Vec2::new(0.0, 21.0),
                    0.1,
                ));
                parent.spawn((
                    label(caption, 15.0, text),
                    Transform::from_xyz(0.0, 46.0 + 18.0 * stack as f32, 0.2),
                ));
            });
    }
}

/// The angle halfway between `a` and `b` by `t`, taking the short way round.
fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let delta =
        (b - a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    a + delta * t
}

/// Places drawings between the last two fixed-step poses, so motion stays smooth
/// when the display rate differs from the fixed rate.
fn follow_poses(
    fixed: Res<Time<Fixed>>,
    poses: Query<(&Pose, &PreviousPose)>,
    mut visuals: Query<(&VisualOf, &Follow, &mut Transform)>,
) {
    let blend = fixed.overstep_fraction();
    for (target, follow, mut transform) in &mut visuals {
        let Ok((pose, previous)) = poses.get(target.0) else {
            continue;
        };
        let at = previous.0.translation.lerp(pose.translation, blend) + follow.base;
        let angle = lerp_angle(previous.0.angle, pose.angle, blend);
        *transform = Transform::from_translation(at.extend(follow.z))
            .with_rotation(Quat::from_rotation_z(angle));
    }
}

/// Mirrors the bike to its facing and squeezes it through a turn-around.
fn face_bike(
    tuning: Res<BikeTuning>,
    bikes: Query<(&BikeState, &Visuals)>,
    mut mirrors: Query<(&ChildOf, &mut Transform), With<Mirror>>,
) {
    for (state, visuals) in &bikes {
        let scale = match state.turning {
            Some(t) => state.facing * (1.0 - 2.0 * (t / tuning.turn_time).min(1.0)),
            None => state.facing,
        };
        // Never squash to zero, which would make the transform singular.
        let scale = if scale.abs() < 0.05 {
            0.05 * scale.signum()
        } else {
            scale
        };
        for (parent, mut transform) in &mut mirrors {
            if visuals.iter().any(|visual| visual == parent.parent()) {
                transform.scale.x = scale;
            }
        }
    }
}

fn tint_crash(
    run: Option<Res<Run>>,
    helmets: Query<&MeshMaterial2d<ColorMaterial>, With<Helmet>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let crashed = run.is_some_and(|run| run.crash.is_some());
    let color = if crashed {
        palette::CRASH
    } else {
        palette::HELMET
    };
    for handle in &helmets {
        if let Some(mut material) = materials.get_mut(&handle.0)
            && material.color != color
        {
            material.color = color;
        }
    }
}

fn animate_tnt(
    traps: Query<(&Trap, &Visuals)>,
    mut crates: Query<&mut Visibility, (With<TntCrate>, Without<TntBlast>)>,
    mut blasts: Query<(&mut Visibility, &mut Transform), With<TntBlast>>,
) {
    for (trap, visuals) in &traps {
        let TrapKind::Tnt { crate_box, .. } = trap.def.kind else {
            continue;
        };
        let exploded = trap.exploded();
        for visual in visuals.iter() {
            if let Ok(mut visibility) = crates.get_mut(visual) {
                visibility.set_if_neq(if exploded {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                });
            }
            if let Ok((mut visibility, mut transform)) = blasts.get_mut(visual) {
                let progress = trap
                    .fired
                    .map(|t| (t - TNT_FUSE) / TNT_BLAST)
                    .filter(|p| (0.0..1.0).contains(p));
                *visibility = if progress.is_some() {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                let grow = progress.map_or(0.0, |p| 0.4 + 0.6 * p);
                transform.scale = Vec3::splat(grow.max(0.01));
                transform.translation = crate_box.center().extend(transform.translation.z);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angles_blend_the_short_way_round() {
        let half = lerp_angle(3.0, -3.0, 0.5);
        assert!((half.abs() - std::f32::consts::PI).abs() < 0.01, "{half}");
        assert!((lerp_angle(0.0, 1.0, 0.25) - 0.25).abs() < 1e-6);
    }
}
