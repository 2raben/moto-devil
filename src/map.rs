//! The level map: a road that winds through the countries from left to right,
//! with a medallion for every level. Finished levels show a tick, the open
//! ones glow, and locked ones a padlock on a footpath. A small bike rides along
//! the road to the chosen level, and the camera follows it. The bar at the
//! bottom names the chosen level and how far the player got in it.
//!
//! The choice itself lives in [`crate::flow::level_map`]; this module only
//! draws it, like the other screens.

use std::ops::Range;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::backdrop::{self, Canvas, SNOW, Theme, noise};
use crate::camera::{CameraSystems, keep_inside};
use crate::flow::{CurrentLevel, Levels, MapArrival, Screen};
use crate::hud::{colored_text, text};
use crate::level::Country;
use crate::progress::{ActivePlayer, Profiles};
use crate::visuals::palette;

/// How wide one country is on the map.
const REGION_WIDTH: f32 = 640.0;
/// Where the sky meets the land.
const HORIZON: f32 = 60.0;
/// The road swings this far above and below its middle line, from one level
/// to the next.
const ROAD_Y: f32 = -70.0;
const ROAD_SWING: f32 = 46.0;
const ROAD_WIDTH: f32 = 18.0;
/// The road starts this far inside the map and ends twice as far from its
/// right edge, leaving room for the flag.
const ROAD_END: f32 = 32.0;
/// Road points between two levels.
const SAMPLES: usize = 16;
/// Sky and land reach this far past the map, for wide or tall windows.
const EXTENT: f32 = 1600.0;
/// Neighbouring countries' colours blend over twice this width.
const BLEND: f32 = 80.0;
/// The visible height the camera keeps inside the map.
const MAP_HEIGHT: f32 = 540.0;
/// The bike rides at least this fast between levels, in map units per second.
const MARKER_SPEED: f32 = 360.0;
const CAMERA_RATE: f32 = 5.0;
/// The bike parks this high on a medallion and starts to climb it this far
/// along the road before.
const HOP_HEIGHT: f32 = 22.0;
const HOP_REACH: f32 = 48.0;
/// The unlock announcement shows this long.
const UNLOCK_SECONDS: f32 = 3.0;

const GOLD: Color = Color::srgb(0.98, 0.78, 0.2);

/// The fill of a locked medallion.
const LOCKED: Color = Color::srgb(0.52, 0.54, 0.58);

/// Draw order on the map, back to front.
mod layer {
    pub const SKY: f32 = 0.0;
    pub const SCENERY: f32 = 1.0;
    pub const LAND: f32 = 3.0;
    pub const DECOR: f32 = 4.0;
    pub const ROAD: f32 = 5.0;
    pub const GLOW: f32 = 7.0;
    pub const NODE: f32 = 8.0;
    pub const LABEL: f32 = 9.0;
    pub const BIKE: f32 = 10.0;
    pub const BANNER: f32 = 11.0;
}

/// One country's stretch of the map.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub country: Country,
    pub left: f32,
    /// The indices into [`Levels`] of the country's levels.
    pub levels: Range<usize>,
}

impl Region {
    pub fn center(&self) -> f32 {
        self.left + 0.5 * REGION_WIDTH
    }
}

/// Where everything lies on the map. Plain geometry, computed from the level
/// list when the map opens.
#[derive(Resource, Clone, Debug, Default)]
pub struct MapLayout {
    pub regions: Vec<Region>,
    /// One medallion per level, in play order.
    pub nodes: Vec<Vec2>,
    /// The road from the start through every level to the flag.
    pub road: Vec<Vec2>,
    /// The distance along the road of each road point.
    along: Vec<f32>,
}

impl MapLayout {
    pub fn new(levels: &Levels) -> Self {
        let mut regions: Vec<Region> = Vec::new();
        for (index, level) in levels.0.iter().enumerate() {
            match regions.last_mut() {
                Some(region) if region.country == level.country => region.levels.end = index + 1,
                _ => regions.push(Region {
                    country: level.country,
                    left: regions.len() as f32 * REGION_WIDTH,
                    levels: index..index + 1,
                }),
            }
        }
        let nodes: Vec<Vec2> = regions
            .iter()
            .flat_map(|region| {
                let count = region.levels.len() as f32;
                region.levels.clone().enumerate().map(move |(k, index)| {
                    let swing = if index % 2 == 0 {
                        ROAD_SWING
                    } else {
                        -ROAD_SWING
                    };
                    Vec2::new(
                        region.left + REGION_WIDTH * (k as f32 + 0.5) / count,
                        ROAD_Y + swing,
                    )
                })
            })
            .collect();
        let width = regions.len() as f32 * REGION_WIDTH;
        let mut stops = vec![Vec2::new(ROAD_END, ROAD_Y)];
        stops.extend(&nodes);
        stops.push(Vec2::new(
            width.max(2.0 * ROAD_END) - 2.0 * ROAD_END,
            ROAD_Y,
        ));

        let mut road = Vec::new();
        for i in 0..stops.len() - 1 {
            let before = stops[i.saturating_sub(1)];
            let after = stops[(i + 2).min(stops.len() - 1)];
            for step in 0..SAMPLES {
                let t = step as f32 / SAMPLES as f32;
                road.push(catmull_rom(before, stops[i], stops[i + 1], after, t));
            }
        }
        road.extend(stops.last());
        let mut along = vec![0.0];
        for pair in road.windows(2) {
            along.push(along[along.len() - 1] + pair[0].distance(pair[1]));
        }
        Self {
            regions,
            nodes,
            road,
            along,
        }
    }

    pub fn width(&self) -> f32 {
        self.regions.len() as f32 * REGION_WIDTH
    }

    /// The road point at a level's medallion.
    fn node_point(level: usize) -> usize {
        (level + 1) * SAMPLES
    }

    /// The road points from the level before `level` (or the start) to `level`.
    fn leg(level: usize) -> Range<usize> {
        level * SAMPLES..Self::node_point(level) + 1
    }

    /// How far along the road a level's medallion lies.
    pub fn node_along(&self, level: usize) -> f32 {
        let point = Self::node_point(level).min(self.along.len().saturating_sub(1));
        self.along.get(point).copied().unwrap_or(0.0)
    }

    /// The road position and its direction at `distance` along the road.
    pub fn point_at(&self, distance: f32) -> (Vec2, Vec2) {
        let segment = self
            .along
            .partition_point(|&d| d <= distance)
            .clamp(1, self.road.len().max(2) - 1);
        let (Some(&a), Some(&b)) = (self.road.get(segment - 1), self.road.get(segment)) else {
            return (Vec2::new(0.0, ROAD_Y), Vec2::X);
        };
        let length = self.along[segment] - self.along[segment - 1];
        let t = ((distance - self.along[segment - 1]) / length.max(f32::EPSILON)).clamp(0.0, 1.0);
        (a.lerp(b, t), (b - a).normalize_or(Vec2::X))
    }

    /// The country under `x`; the first and last countries reach past the map.
    fn region_at(&self, x: f32) -> Option<&Region> {
        let index = (x / REGION_WIDTH).floor().max(0.0) as usize;
        self.regions
            .get(index.min(self.regions.len().saturating_sub(1)))
    }

    /// The area the camera keeps its view inside.
    fn bounds(&self) -> Rect {
        Rect::new(0.0, -0.5 * MAP_HEIGHT, self.width(), 0.5 * MAP_HEIGHT)
    }
}

fn catmull_rom(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let (t2, t3) = (t * t, t * t * t);
    0.5 * (2.0 * p1
        + (p2 - p0) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t3)
}

/// Where a level stands for the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Standing {
    Done,
    Open,
    Locked,
}

/// The bike that rides along the road to the chosen level.
#[derive(Component)]
struct Marker {
    along: f32,
}

/// A level's medallion.
#[derive(Component)]
struct Medallion(usize);

/// A medallion that just appeared, growing out of the road.
#[derive(Component)]
struct Unlocking {
    age: f32,
}

/// The light behind the chosen medallion.
#[derive(Component)]
struct Glow;

#[derive(Component)]
struct ChosenTitle;

#[derive(Component)]
struct ChosenStatus;

#[derive(Component)]
struct UnlockCue {
    age: f32,
}

/// A line of the unlock announcement, with its full colour.
#[derive(Component)]
struct UnlockText(Color);

pub struct LevelMapPlugin;

impl Plugin for LevelMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapLayout>()
            .add_systems(OnEnter(Screen::LevelMap), spawn_map)
            .add_systems(
                Update,
                (
                    ride_marker,
                    pulse_medallions,
                    fade_unlock_cue,
                    describe_choice.run_if(
                        resource_changed::<CurrentLevel>
                            .or_else(any_match_filter::<Added<ChosenTitle>>),
                    ),
                )
                    .run_if(in_state(Screen::LevelMap)),
            )
            .add_systems(
                PostUpdate,
                follow_marker
                    .in_set(CameraSystems)
                    .before(TransformSystems::Propagate)
                    .run_if(in_state(Screen::LevelMap)),
            );
    }
}

/// Which levels are finished, open, or locked for the active player.
fn standings(profiles: &Profiles, active: &ActivePlayer, levels: &Levels) -> Vec<Standing> {
    let ids = levels.ids();
    let unlocked = profiles.unlocked(active.0, &ids);
    let player = active.0.and_then(|player| profiles.players.get(player));
    ids.iter()
        .zip(unlocked)
        .map(
            |(id, open)| match (player.is_some_and(|p| p.is_done(id)), open) {
                (true, _) => Standing::Done,
                (false, true) => Standing::Open,
                (false, false) => Standing::Locked,
            },
        )
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn spawn_map(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    levels: Res<Levels>,
    profiles: Res<Profiles>,
    active: Res<ActivePlayer>,
    current: Res<CurrentLevel>,
    arrival: Res<MapArrival>,
) {
    let layout = MapLayout::new(&levels);
    let standing = standings(&profiles, &active, &levels);
    let root = commands
        .spawn((
            Name::new("Level map"),
            DespawnOnExit(Screen::LevelMap),
            Transform::default(),
            Visibility::default(),
        ))
        .id();
    let finished_all = standing.last() == Some(&Standing::Done);
    let end = layout.road.last().copied().unwrap_or_default();
    let drawings = [
        (layer::SKY, vec![(Color::WHITE, sky(&layout).mesh())]),
        (layer::LAND, vec![(Color::WHITE, land(&layout).mesh())]),
        (layer::SCENERY, scenery(&layout).meshes()),
        (layer::DECOR, decor(&layout).meshes()),
        (layer::ROAD, road(&layout, &standing).meshes()),
        (layer::NODE, flag(end, finished_all).meshes()),
    ];
    for (z, drawing) in drawings {
        for (i, (color, mesh)) in drawing.into_iter().enumerate() {
            commands.spawn((
                ChildOf(root),
                Mesh2d(meshes.add(mesh)),
                MeshMaterial2d(materials.add(color)),
                Transform::from_xyz(0.0, 0.0, z + i as f32 * 0.001),
            ));
        }
    }

    let circle = meshes.add(Circle::new(1.0));
    let mut disc = |color: Color, radius: f32, at: Vec2, z: f32| {
        (
            Mesh2d(circle.clone()),
            MeshMaterial2d(materials.add(color)),
            Transform::from_translation(at.extend(z)).with_scale(Vec3::splat(radius)),
        )
    };

    for (index, (&at, &state)) in layout.nodes.iter().zip(&standing).enumerate() {
        // Numbers run on through all countries.
        let number = index + 1;
        let fill = match state {
            Standing::Done => palette::FINISH,
            Standing::Open => palette::CHECKPOINT,
            Standing::Locked => LOCKED,
        };
        let mut placed = Transform::from_translation(at.extend(layer::NODE));
        let mut node = commands.spawn((ChildOf(root), Medallion(index), Visibility::default()));
        if arrival.unlocked == Some(index) {
            placed.scale = Vec3::ZERO;
            node.insert(Unlocking { age: 0.0 });
        }
        node.insert(placed);
        node.with_children(|parts| {
            parts.spawn(disc(
                Color::srgba(0.0, 0.0, 0.0, 0.25),
                26.0,
                Vec2::new(2.0, -3.0),
                0.0,
            ));
            parts.spawn(disc(palette::POLE, 24.0, Vec2::ZERO, 0.1));
            parts.spawn(disc(fill, 19.0, Vec2::ZERO, 0.2));
            match state {
                Standing::Locked => padlock(parts, &mut disc),
                _ => {
                    parts.spawn((
                        label(number.to_string(), 22.0, palette::TEXT),
                        Transform::from_xyz(0.0, 0.0, 0.3),
                    ));
                }
            }
            if state == Standing::Done {
                parts.spawn(disc(palette::POLE, 10.0, Vec2::new(17.0, 17.0), 0.4));
                parts.spawn((
                    label("✓", 14.0, palette::FINISH),
                    Transform::from_xyz(17.0, 17.0, 0.5),
                ));
            }
        });

        let name = levels.0[index].name;
        let below = at - Vec2::Y * 44.0;
        let color = if state == Standing::Locked {
            palette::MUTED
        } else {
            palette::TEXT
        };
        commands.spawn((
            ChildOf(root),
            Sprite::from_color(
                Color::srgba(0.05, 0.08, 0.12, 0.6),
                Vec2::new(text_width(name, 14.0) + 16.0, 22.0),
            ),
            Transform::from_translation(below.extend(layer::LABEL)),
            children![(label(name, 14.0, color), Transform::from_xyz(0.0, 0.0, 0.1))],
        ));
    }

    for region in &layout.regions {
        let done = region
            .levels
            .clone()
            .filter(|&index| standing[index] == Standing::Done)
            .count();
        let count = region.levels.len();
        let locked = standing[region.levels.start] == Standing::Locked;
        let caption = if done == count {
            format!("{}  {done}/{count} ✓", region.country.name())
        } else {
            format!("{}  {done}/{count}", region.country.name())
        };
        let (panel, color) = if locked {
            (Color::srgba(0.05, 0.08, 0.12, 0.45), palette::MUTED)
        } else {
            (palette::PANEL, palette::TEXT)
        };
        commands.spawn((
            ChildOf(root),
            Sprite::from_color(panel, Vec2::new(text_width(&caption, 20.0) + 32.0, 34.0)),
            Transform::from_xyz(region.center(), HORIZON + 186.0, layer::BANNER),
            children![(
                label(caption, 20.0, color),
                Transform::from_xyz(0.0, 0.0, 0.1)
            )],
        ));
    }

    commands.spawn((
        ChildOf(root),
        label("START", 13.0, palette::TEXT_DARK),
        Transform::from_xyz(ROAD_END, ROAD_Y - 26.0, layer::LABEL),
    ));

    commands.spawn((
        ChildOf(root),
        Glow,
        disc(
            Color::srgba(1.0, 0.95, 0.7, 0.45),
            34.0,
            Vec2::ZERO,
            layer::GLOW,
        ),
    ));

    let start = arrival.from.unwrap_or(current.0);
    let along = layout.node_along(start);
    commands
        .spawn((
            ChildOf(root),
            Marker { along },
            Transform::from_translation(layout.point_at(along).0.extend(layer::BIKE)),
            Visibility::default(),
        ))
        .with_children(|bike| mini_bike(bike, &mut disc));

    commands.insert_resource(layout);
    spawn_info(&mut commands, &levels, &arrival);
}

/// A rough width of a line of the UI font, for the panels behind map labels.
fn text_width(text: &str, size: f32) -> f32 {
    text.chars().count() as f32 * size * 0.62
}

fn label(content: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text2d::new(content),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

fn padlock<B: Bundle>(
    parts: &mut ChildSpawnerCommands,
    disc: &mut impl FnMut(Color, f32, Vec2, f32) -> B,
) {
    let metal = Color::srgb(0.2, 0.21, 0.24);
    parts.spawn(disc(metal, 6.5, Vec2::new(0.0, 3.0), 0.3));
    parts.spawn(disc(LOCKED, 3.8, Vec2::new(0.0, 3.0), 0.35));
    parts.spawn((
        Sprite::from_color(metal, Vec2::new(16.0, 11.0)),
        Transform::from_xyz(0.0, -3.5, 0.4),
    ));
}

/// The rider on a small bike, standing on the origin and facing right.
fn mini_bike<B: Bundle>(
    bike: &mut ChildSpawnerCommands,
    disc: &mut impl FnMut(Color, f32, Vec2, f32) -> B,
) {
    let part = |color: Color, size: Vec2, at: Vec2, angle: f32, z: f32| {
        (
            Sprite::from_color(color, size),
            Transform::from_translation(at.extend(z)).with_rotation(Quat::from_rotation_z(angle)),
        )
    };
    for x in [-12.0, 12.0] {
        bike.spawn(disc(palette::TYRE, 7.5, Vec2::new(x, 7.5), 0.0));
        bike.spawn(disc(palette::HUB, 3.0, Vec2::new(x, 7.5), 0.1));
    }
    bike.spawn(part(
        palette::FRAME,
        Vec2::new(28.0, 6.0),
        Vec2::new(0.0, 14.0),
        0.1,
        0.2,
    ));
    bike.spawn(part(
        palette::FRAME,
        Vec2::new(3.0, 12.0),
        Vec2::new(11.0, 17.0),
        -0.4,
        0.2,
    ));
    bike.spawn(part(
        palette::RIDER,
        Vec2::new(7.0, 15.0),
        Vec2::new(-2.0, 24.0),
        -0.35,
        0.3,
    ));
    bike.spawn(part(
        palette::RIDER,
        Vec2::new(12.0, 3.5),
        Vec2::new(5.0, 26.0),
        -0.3,
        0.3,
    ));
    bike.spawn(disc(palette::HELMET, 5.5, Vec2::new(1.5, 34.0), 0.4));
    bike.spawn(part(
        palette::HORN,
        Vec2::new(2.5, 6.0),
        Vec2::new(-1.5, 39.5),
        0.4,
        0.35,
    ));
    bike.spawn(part(
        palette::HORN,
        Vec2::new(2.5, 6.0),
        Vec2::new(4.5, 39.5),
        -0.4,
        0.35,
    ));
}

/// The chequered flag at the end of the road, with a cup under it once every
/// level is done.
fn flag(at: Vec2, gold: bool) -> Canvas {
    let mut canvas = Canvas::default();
    let pole = Rect::new(at.x - 2.0, at.y, at.x + 2.0, at.y + 70.0);
    canvas.rect(palette::POLE, pole);
    for row in 0..3 {
        for col in 0..5 {
            let color = if (row + col) % 2 == 0 {
                palette::TEXT_DARK
            } else {
                palette::POLE
            };
            let min = Vec2::new(
                at.x + 2.0 + col as f32 * 8.0,
                at.y + 46.0 + row as f32 * 8.0,
            );
            canvas.rect(color, Rect::from_corners(min, min + Vec2::splat(8.0)));
        }
    }
    if gold {
        canvas.circle(GOLD, at + Vec2::new(24.0, 30.0), 13.0);
        canvas.rect(
            GOLD,
            Rect::new(at.x + 20.0, at.y + 6.0, at.x + 28.0, at.y + 20.0),
        );
        canvas.rect(
            GOLD,
            Rect::new(at.x + 12.0, at.y + 2.0, at.x + 36.0, at.y + 8.0),
        );
    }
    canvas
}

/// The bar at the bottom and the unlock announcement.
fn spawn_info(commands: &mut Commands, levels: &Levels, arrival: &MapArrival) {
    commands.spawn((
        Name::new("Level map info"),
        DespawnOnExit(Screen::LevelMap),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
            width: Val::Percent(100.0),
            padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(3.0),
            ..default()
        },
        BackgroundColor(palette::PANEL),
        children![
            (ChosenTitle, text("", 22.0)),
            (ChosenStatus, colored_text("", 15.0, palette::MUTED)),
            colored_text(
                "←/→ Level wählen   Enter fahren   Esc Spielerauswahl",
                14.0,
                palette::MUTED,
            ),
        ],
    ));
    let Some(level) = arrival.unlocked.and_then(|index| levels.0.get(index)) else {
        return;
    };
    commands.spawn((
        Name::new("Unlock cue"),
        UnlockCue { age: 0.0 },
        DespawnOnExit(Screen::LevelMap),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(22.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            Node {
                padding: UiRect::axes(Val::Px(28.0), Val::Px(12.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                border_radius: BorderRadius::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            children![
                (
                    UnlockText(GOLD),
                    colored_text("Neues Level freigeschaltet!", 30.0, GOLD)
                ),
                (UnlockText(palette::TEXT), text(level.name, 24.0)),
            ],
        )],
    ));
}

/// Quads with a colour at each corner, blended by the GPU.
#[derive(Default)]
struct Gradient {
    positions: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl Gradient {
    /// A rectangle whose colour runs from `left` to `right` (each given as
    /// bottom, top) across it.
    fn quad(&mut self, rect: Rect, left: (Color, Color), right: (Color, Color)) {
        let base = self.positions.len() as u32;
        let corners = [
            (Vec2::new(rect.min.x, rect.min.y), left.0),
            (Vec2::new(rect.max.x, rect.min.y), right.0),
            (Vec2::new(rect.max.x, rect.max.y), right.1),
            (Vec2::new(rect.min.x, rect.max.y), left.1),
        ];
        for (at, color) in corners {
            self.positions.push([at.x, at.y, 0.0]);
            self.colors.push(color.to_linear().to_f32_array());
        }
        self.indices
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn mesh(self) -> Mesh {
        let count = self.positions.len();
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; count])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; count])
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// Columns across the map where colours are pure: each country's middle,
/// with blends of `2 * BLEND` at the borders.
fn columns(layout: &MapLayout) -> Vec<f32> {
    let mut xs = vec![-EXTENT];
    for region in layout.regions.iter().skip(1) {
        xs.extend([region.left - BLEND, region.left + BLEND]);
    }
    xs.push(layout.width() + EXTENT);
    xs
}

/// Bands of colour from bottom to top, painted across all countries. Band `i`
/// runs from colour `i` at its bottom to colour `i + 1` at its top.
fn painted(
    layout: &MapLayout,
    bands: &[(f32, f32)],
    color: impl Fn(&Theme, usize) -> Color,
) -> Gradient {
    let mut gradient = Gradient::default();
    let xs = columns(layout);
    let theme_at = |x: f32| {
        layout.region_at(x).map_or_else(
            || backdrop::theme(Country::Training),
            |r| backdrop::theme(r.country),
        )
    };
    for pair in xs.windows(2) {
        let (left, right) = (theme_at(pair[0]), theme_at(pair[1]));
        for (band, &(bottom, top)) in bands.iter().enumerate() {
            gradient.quad(
                Rect::new(pair[0], bottom, pair[1], top),
                (color(&left, band), color(&left, band + 1)),
                (color(&right, band), color(&right, band + 1)),
            );
        }
    }
    gradient
}

/// Each country's sky, hazy at the horizon and deeper above.
fn sky(layout: &MapLayout) -> Gradient {
    let deep = Color::srgb(0.18, 0.32, 0.6);
    painted(
        layout,
        &[(HORIZON - 20.0, HORIZON + 240.0), (HORIZON + 240.0, EXTENT)],
        |theme, level| match level {
            0 => theme.sky.mix(&Color::WHITE, 0.45),
            1 => theme.sky,
            _ => theme.sky.mix(&deep, 0.45),
        },
    )
}

/// Each country's land, hazy far away and richer close by.
fn land(layout: &MapLayout) -> Gradient {
    painted(
        layout,
        &[(-EXTENT, -300.0), (-300.0, HORIZON)],
        |theme, level| match level {
            0 => theme.surface.mix(&theme.ground, 0.55),
            1 => theme.surface,
            _ => theme.surface.mix(&theme.sky, 0.35),
        },
    )
}

/// Far hills, landmarks, and clouds above the horizon.
fn scenery(layout: &MapLayout) -> Canvas {
    let mut canvas = Canvas::default();
    let white = Color::srgba(1.0, 1.0, 1.0, 0.85);
    for (r, region) in layout.regions.iter().enumerate() {
        let theme = backdrop::theme(region.country);
        let sky = theme.sky;
        let far = |color: Color| backdrop::haze(color, sky, 0.3);
        let (left, cx) = (region.left, region.center());
        let ground = |x: f32| Vec2::new(x, HORIZON);
        let seed = r as u32 * 17 + 3;
        for i in 0..2 {
            let x = left + 120.0 + 380.0 * i as f32 + 60.0 * noise(seed, i);
            let y = HORIZON + 200.0 + 40.0 * noise(seed + 1, i);
            backdrop::cloud(&mut canvas, white, Vec2::new(x, y), 34.0);
        }
        match region.country {
            Country::Training => {
                for (i, x) in [left + 70.0, cx - 150.0, cx + 120.0, left + 580.0]
                    .into_iter()
                    .enumerate()
                {
                    backdrop::tree(
                        &mut canvas,
                        far(Color::srgb(0.35, 0.25, 0.18)),
                        far(Color::srgb(0.2, 0.48, 0.22)),
                        ground(x),
                        60.0 + 30.0 * noise(seed, i as i32),
                    );
                }
                // A practice ramp with a cone.
                canvas.triangle(
                    far(palette::SLAB),
                    ground(cx - 40.0),
                    ground(cx + 40.0),
                    Vec2::new(cx + 40.0, HORIZON + 34.0),
                );
                canvas.triangle(
                    far(palette::CHECKPOINT),
                    ground(cx + 56.0),
                    ground(cx + 72.0),
                    Vec2::new(cx + 64.0, HORIZON + 22.0),
                );
            }
            Country::Egypt => {
                canvas.circle(
                    far(Color::srgb(1.0, 0.95, 0.7)),
                    Vec2::new(cx + 200.0, HORIZON + 150.0),
                    34.0,
                );
                for (x, size) in [(cx - 110.0, 120.0), (cx + 40.0, 90.0), (cx + 150.0, 60.0)] {
                    let top = Vec2::new(x, HORIZON + size * 0.9);
                    canvas.triangle(
                        far(Color::srgb(0.84, 0.66, 0.4)),
                        ground(x - size),
                        ground(x),
                        top,
                    );
                    canvas.triangle(
                        far(Color::srgb(0.7, 0.52, 0.32)),
                        ground(x),
                        ground(x + size),
                        top,
                    );
                }
                for x in [left + 50.0, left + 590.0] {
                    backdrop::palm(
                        &mut canvas,
                        far(Color::srgb(0.5, 0.35, 0.2)),
                        far(Color::srgb(0.25, 0.5, 0.2)),
                        ground(x),
                        80.0,
                    );
                }
            }
            Country::Australia => {
                let rock = far(Color::srgb(0.78, 0.34, 0.16));
                let outline: Vec<Vec2> = (0..=20)
                    .map(|k| {
                        let a = std::f32::consts::PI * k as f32 / 20.0;
                        Vec2::new(
                            cx - 20.0 + 190.0 * a.cos(),
                            HORIZON + 80.0 * a.sin().max(0.0).powf(0.3),
                        )
                    })
                    .collect();
                canvas.polygon(rock, &outline);
                for (i, x) in [left + 60.0, left + 570.0].into_iter().enumerate() {
                    backdrop::tree(
                        &mut canvas,
                        far(Color::srgb(0.9, 0.88, 0.8)),
                        far(Color::srgb(0.45, 0.58, 0.4)),
                        ground(x),
                        70.0 + 20.0 * noise(seed, i as i32),
                    );
                }
            }
            Country::Usa => {
                canvas.circle(
                    far(Color::srgb(1.0, 0.88, 0.55)),
                    Vec2::new(cx - 210.0, HORIZON + 140.0),
                    36.0,
                );
                let mesa = far(Color::srgb(0.74, 0.38, 0.22));
                let shade = far(Color::srgb(0.62, 0.3, 0.2));
                for (x, width, height) in [(cx - 150.0, 150.0, 90.0), (cx + 90.0, 110.0, 120.0)] {
                    canvas.polygon(
                        mesa,
                        &[
                            ground(x - 0.5 * width - 30.0),
                            ground(x + 0.5 * width + 30.0),
                            Vec2::new(x + 0.5 * width, HORIZON + height),
                            Vec2::new(x - 0.5 * width, HORIZON + height),
                        ],
                    );
                    canvas.rect(
                        shade,
                        Rect::new(
                            x - 0.5 * width,
                            HORIZON + height - 12.0,
                            x + 0.5 * width,
                            HORIZON + height,
                        ),
                    );
                }
                for (i, x) in [left + 60.0, cx - 20.0, left + 590.0]
                    .into_iter()
                    .enumerate()
                {
                    backdrop::saguaro(
                        &mut canvas,
                        far(Color::srgb(0.36, 0.5, 0.32)),
                        ground(x),
                        55.0 + 20.0 * noise(seed, i as i32),
                    );
                }
            }
            Country::Paris => {
                backdrop::rooftops(
                    &mut canvas,
                    seed,
                    far(Color::srgb(0.84, 0.78, 0.66)),
                    far(Color::srgb(0.38, 0.42, 0.5)),
                    (left, left + REGION_WIDTH),
                    HORIZON - 10.0,
                    HORIZON + 30.0,
                );
                backdrop::eiffel_tower(
                    &mut canvas,
                    far(Color::srgb(0.3, 0.28, 0.3)),
                    ground(cx + 140.0),
                    170.0,
                );
            }
            Country::Switzerland => {
                let rock = far(Color::srgb(0.46, 0.5, 0.6));
                let snow = backdrop::haze(SNOW, sky, 0.1);
                for (x, half, height) in [
                    (cx - 170.0, 120.0, 120.0),
                    (cx + 10.0, 110.0, 165.0),
                    (cx + 180.0, 120.0, 105.0),
                ] {
                    let peak = Vec2::new(x + 10.0, HORIZON + height);
                    canvas.triangle(rock, ground(x - half), ground(x + half), peak);
                    canvas.triangle(
                        snow,
                        peak.lerp(ground(x - half), 0.32),
                        peak.lerp(ground(x + half), 0.32),
                        peak,
                    );
                }
                for (i, x) in [left + 40.0, left + 80.0, left + 560.0, left + 600.0]
                    .into_iter()
                    .enumerate()
                {
                    backdrop::fir(
                        &mut canvas,
                        far(Color::srgb(0.35, 0.25, 0.18)),
                        far(Color::srgb(0.12, 0.38, 0.2)),
                        ground(x),
                        55.0 + 20.0 * noise(seed, i as i32),
                    );
                }
            }
        }
    }
    canvas
}

/// Small things on the land between the road and the horizon.
fn decor(layout: &MapLayout) -> Canvas {
    let mut canvas = Canvas::default();
    for (r, region) in layout.regions.iter().enumerate() {
        let theme = backdrop::theme(region.country);
        let shade = theme.surface.mix(&theme.ground, 0.45);
        let light = theme.surface.mix(&Color::WHITE, 0.25);
        let seed = r as u32 * 31 + 7;
        for i in 0..12 {
            let x = region.left + 20.0 + 600.0 * noise(seed, i);
            let y = HORIZON - 12.0 - 40.0 * noise(seed + 1, i);
            let size = 3.0 + 4.0 * noise(seed + 2, i);
            match region.country {
                Country::Paris => {
                    canvas.rect(shade, Rect::new(x - 1.0, y, x + 1.0, y + 14.0));
                    canvas.circle(Color::srgb(1.0, 0.9, 0.55), Vec2::new(x, y + 15.0), 2.5);
                }
                Country::Switzerland if i % 3 == 0 => {
                    canvas.circle(SNOW, Vec2::new(x, y), size + 2.0);
                }
                _ => {
                    canvas.circle(shade, Vec2::new(x, y), size);
                    canvas.circle(light, Vec2::new(x - 1.0, y + 1.0), size * 0.5);
                }
            }
        }
    }
    canvas
}

/// The road: paved up to every level that is open, a dotted footpath to the
/// locked ones, and on to the flag once the last level is done.
fn road(layout: &MapLayout, standing: &[Standing]) -> Canvas {
    let mut canvas = Canvas::default();
    let edge = Color::srgb(0.2, 0.2, 0.23);
    let lines = Color::srgb(0.98, 0.92, 0.62);
    let path = Color::srgba(0.25, 0.2, 0.15, 0.55);
    let legs = standing.len() + 1;
    for leg in 0..legs {
        let paved = match standing.get(leg) {
            Some(state) => *state != Standing::Locked,
            None => standing.last() == Some(&Standing::Done),
        };
        let range = MapLayout::leg(leg);
        let Some(points) = layout
            .road
            .get(range.start..range.end.min(layout.road.len()))
        else {
            continue;
        };
        if paved {
            stroke(&mut canvas, edge, points, ROAD_WIDTH + 7.0);
            stroke(&mut canvas, palette::TRACK, points, ROAD_WIDTH);
            for pair in points.windows(2).step_by(2) {
                let (a, b) = (pair[0], pair[0].lerp(pair[1], 0.6));
                bar(&mut canvas, lines, a, b, 2.5);
            }
        } else {
            for pair in points.windows(2) {
                canvas.circle(path, pair[0].lerp(pair[1], 0.5), 3.2);
            }
        }
    }
    canvas
}

fn bar(canvas: &mut Canvas, color: Color, a: Vec2, b: Vec2, width: f32) {
    let side = (b - a).perp().normalize_or_zero() * 0.5 * width;
    canvas.polygon(color, &[a - side, b - side, b + side, a + side]);
}

/// A line through `points` with round joints.
fn stroke(canvas: &mut Canvas, color: Color, points: &[Vec2], width: f32) {
    for pair in points.windows(2) {
        bar(canvas, color, pair[0], pair[1], width);
    }
    for &point in points {
        canvas.circle(color, point, 0.5 * width);
    }
}

/// Rides the bike along the road towards the chosen level, faster when it is
/// far away, and turns it to face the way it rides.
fn ride_marker(
    time: Res<Time>,
    layout: Res<MapLayout>,
    current: Res<CurrentLevel>,
    mut markers: Query<(&mut Marker, &mut Transform)>,
) {
    let target = layout.node_along(current.0);
    let dt = time.delta_secs();
    for (mut marker, mut transform) in &mut markers {
        let gap = target - marker.along;
        let step = (MARKER_SPEED * dt).max(gap.abs() * (1.0 - (-4.0 * dt).exp()));
        marker.along += gap.clamp(-step, step);
        let (at, direction) = layout.point_at(marker.along);
        let facing = if gap < -0.5 {
            -1.0
        } else if gap > 0.5 {
            1.0
        } else {
            transform.scale.x.signum()
        };
        // Near a medallion the bike hops up and parks on top of it, level.
        let nearest = (0..layout.nodes.len())
            .map(|level| (layout.node_along(level) - marker.along).abs())
            .fold(f32::INFINITY, f32::min);
        let hop = (1.0 - nearest / HOP_REACH).clamp(0.0, 1.0);
        let hop = hop * hop * (3.0 - 2.0 * hop);
        transform.translation = (at + Vec2::Y * HOP_HEIGHT * hop).extend(layer::BIKE);
        // A mirrored bike lies along the road at the same angle.
        transform.rotation = Quat::from_rotation_z(direction.to_angle() * (1.0 - hop));
        transform.scale = Vec3::new(facing * 1.3, 1.3, 1.0);
    }
}

/// The chosen medallion breathes and has a glow; a newly unlocked one grows
/// in with a little overshoot.
fn pulse_medallions(
    time: Res<Time>,
    layout: Res<MapLayout>,
    current: Res<CurrentLevel>,
    mut commands: Commands,
    mut medallions: Query<
        (Entity, &Medallion, &mut Transform, Option<&mut Unlocking>),
        Without<Glow>,
    >,
    mut glows: Query<&mut Transform, With<Glow>>,
) {
    let t = time.elapsed_secs();
    let breath = 1.0 + 0.07 * (t * 4.0).sin();
    for (entity, medallion, mut transform, unlocking) in &mut medallions {
        let mut scale = if medallion.0 == current.0 {
            breath
        } else {
            1.0
        };
        if let Some(mut unlocking) = unlocking {
            unlocking.age += time.delta_secs();
            // Waits for the bike, then grows past full size and settles.
            let grow = ((unlocking.age - 0.5) / 0.6).clamp(0.0, 1.0);
            scale *= grow * (1.0 + 0.5 * (std::f32::consts::PI * grow).sin());
            if grow >= 1.0 {
                commands.entity(entity).remove::<Unlocking>();
            }
        }
        transform.scale = Vec3::new(scale, scale, 1.0);
    }
    let Some(&at) = layout.nodes.get(current.0) else {
        return;
    };
    for mut glow in &mut glows {
        let size = 34.0 * (1.0 + 0.12 * (t * 4.0).sin());
        glow.translation = at.extend(layer::GLOW);
        glow.scale = Vec3::new(size, size, 1.0);
    }
}

/// Shows the unlock announcement for a while and then fades it out.
fn fade_unlock_cue(
    time: Res<Time>,
    mut cues: Query<(&mut UnlockCue, &mut Visibility, &Children)>,
    mut panels: Query<&mut BackgroundColor>,
    mut lines: Query<(&UnlockText, &mut TextColor)>,
) {
    for (mut cue, mut visibility, children) in &mut cues {
        cue.age += time.delta_secs();
        let alpha = ((UNLOCK_SECONDS - cue.age) / 0.6).clamp(0.0, 1.0);
        for &panel in children {
            if let Ok(mut background) = panels.get_mut(panel) {
                background.0 = palette::PANEL.with_alpha(palette::PANEL.alpha() * alpha);
            }
        }
        for (line, mut color) in &mut lines {
            color.0 = line.0.with_alpha(alpha);
        }
        if alpha <= 0.0 {
            visibility.set_if_neq(Visibility::Hidden);
        }
    }
}

/// The chosen level's number across all countries, its country, and its name,
/// unless the name only repeats the number.
fn chosen_title(index: usize, country: Country, name: &str) -> String {
    let number = format!("Level {}", index + 1);
    if name == number {
        format!("{number} · {}", country.name())
    } else {
        format!("{number} · {} · {name}", country.name())
    }
}

/// The bottom bar: the chosen level, where it stands, and the player's tally.
fn describe_choice(
    levels: Res<Levels>,
    current: Res<CurrentLevel>,
    profiles: Res<Profiles>,
    active: Res<ActivePlayer>,
    mut titles: Query<&mut Text, (With<ChosenTitle>, Without<ChosenStatus>)>,
    mut statuses: Query<&mut Text, (With<ChosenStatus>, Without<ChosenTitle>)>,
) {
    let Some(level) = levels.0.get(current.0) else {
        return;
    };
    let title = chosen_title(current.0, level.country, level.name);
    let player = active.0.and_then(|index| profiles.players.get(index));
    let progress = player.and_then(|player| player.level(level.id));
    let state = match progress {
        Some(progress) if progress.done => "Geschafft ✓".to_string(),
        Some(progress) => progress
            .farthest_crash
            .map_or("Noch nicht geschafft".to_string(), |x| {
                let share = (x - level.start.x) / (level.finish.min.x - level.start.x).max(1.0);
                format!(
                    "Noch nicht geschafft · dein Grabstein steht bei {} % der Strecke",
                    (100.0 * share).clamp(0.0, 99.0).round()
                )
            }),
        None => "Neu – noch nie gefahren".to_string(),
    };
    let tally = match (active.0, player) {
        (Some(index), Some(player)) => format!(
            "   ·   {}: {} von {} Leveln geschafft",
            player.name,
            profiles.levels_done(index, &levels.ids()),
            levels.0.len()
        ),
        _ => String::new(),
    };
    for mut text in &mut titles {
        text.0.clone_from(&title);
    }
    for mut text in &mut statuses {
        text.0 = format!("{state}{tally}");
    }
}

/// Keeps the bike in view, inside the map.
fn follow_marker(
    time: Res<Time>,
    layout: Res<MapLayout>,
    markers: Query<&Transform, (With<Marker>, Without<Camera2d>)>,
    mut cameras: Query<(&mut Transform, &Projection), With<Camera2d>>,
    mut placed: Local<bool>,
) {
    let Some(marker) = markers.iter().next() else {
        *placed = false;
        return;
    };
    for (mut camera, projection) in &mut cameras {
        let half_view = match projection {
            Projection::Orthographic(ortho) => ortho.area.half_size(),
            _ => crate::camera::VIEW * 0.5,
        };
        let target = keep_inside(layout.bounds(), marker.translation.truncate(), half_view);
        let next = if *placed {
            let blend = 1.0 - (-CAMERA_RATE * time.delta_secs()).exp();
            camera.translation.truncate().lerp(target, blend)
        } else {
            target
        };
        camera.translation = next.extend(camera.translation.z);
    }
    *placed = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_level_has_a_medallion_on_the_road_in_its_country() {
        let levels = Levels::default();
        let layout = MapLayout::new(&levels);
        assert_eq!(layout.nodes.len(), levels.0.len());
        assert_eq!(layout.regions.len(), 6, "six countries");
        for (index, node) in layout.nodes.iter().enumerate() {
            let region = layout.region_at(node.x).unwrap();
            assert!(region.levels.contains(&index), "level {index}");
            assert_eq!(region.country, levels.0[index].country);
            let (on_road, _) = layout.point_at(layout.node_along(index));
            assert!(on_road.distance(*node) < 0.01, "level {index}");
        }
        for pair in layout.nodes.windows(2) {
            assert!(pair[0].x < pair[1].x);
        }
        for index in 1..levels.0.len() {
            assert!(layout.node_along(index - 1) < layout.node_along(index));
        }
    }

    #[test]
    fn the_road_stays_on_the_land_and_between_its_ends() {
        let layout = MapLayout::new(&Levels::default());
        for point in &layout.road {
            assert!(point.y < HORIZON - ROAD_WIDTH, "{point}");
            assert!(point.y > ROAD_Y - 2.0 * ROAD_SWING, "{point}");
            assert!((0.0..=layout.width()).contains(&point.x), "{point}");
        }
        let (start, _) = layout.point_at(-10.0);
        let (end, _) = layout.point_at(f32::MAX);
        assert_eq!(start, layout.road[0]);
        assert_eq!(end, *layout.road.last().unwrap());
    }

    #[test]
    fn locked_levels_get_a_footpath_and_open_ones_a_road() {
        let layout = MapLayout::new(&Levels::default());
        let mut standing = vec![Standing::Locked; layout.nodes.len()];
        standing[0] = Standing::Done;
        standing[1] = Standing::Open;
        let canvas = road(&layout, &standing);
        let colors: Vec<Color> = canvas
            .meshes()
            .into_iter()
            .map(|(color, _)| color)
            .collect();
        assert!(colors.contains(&palette::TRACK));
        assert!(
            colors.iter().any(|color| color.alpha() < 1.0),
            "the footpath"
        );
    }

    #[test]
    fn the_title_numbers_levels_across_countries() {
        assert_eq!(
            chosen_title(0, Country::Training, "Level 1"),
            "Level 1 · Übungsgelände"
        );
        assert_eq!(
            chosen_title(4, Country::Egypt, "Tal der Könige"),
            "Level 5 · Ägypten · Tal der Könige"
        );
    }

    #[test]
    fn an_empty_level_list_still_has_a_map() {
        let layout = MapLayout::new(&Levels(Vec::new()));
        assert!(layout.nodes.is_empty());
        assert!(layout.road.len() >= 2);
        assert!(layout.node_along(3).is_finite());
        assert!(layout.point_at(100.0).0.is_finite());
    }
}
