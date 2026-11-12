//! Each country's look: its sky, ground colours, and a backdrop of two parallax
//! layers made of simple shapes (pyramids and dunes, mesas and saguaros, the
//! Paris skyline, snowy peaks, ...).
//!
//! This is presentation only. The backdrop has no collision, is never a hazard,
//! and is drawn behind the course in muted colours so the track and its traps
//! stay readable.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::course::LevelRoot;
use crate::level::{Country, Level};
use crate::visuals::VisualOf;

/// Colours of one country.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub sky: Color,
    /// The ground below the surface.
    pub ground: Color,
    /// The strip along the top of the ground: grass, sand, red earth, roofs.
    pub surface: Color,
    /// Where the surface lies higher than this, it is snow.
    pub snow_line: Option<f32>,
    /// Solid blocks and pillars.
    pub rock: Color,
    /// Paris draws the ground as buildings with rows of windows.
    pub windows: Option<Color>,
    pub quicksand: Color,
}

pub const SNOW: Color = Color::srgb(0.95, 0.97, 1.0);

pub fn theme(country: Country) -> Theme {
    let base = Theme {
        sky: Color::srgb(0.55, 0.76, 0.93),
        ground: Color::srgb(0.45, 0.31, 0.21),
        surface: Color::srgb(0.38, 0.7, 0.3),
        snow_line: None,
        rock: Color::srgb(0.42, 0.42, 0.46),
        windows: None,
        quicksand: Color::srgb(0.93, 0.8, 0.52),
    };
    match country {
        Country::Training => base,
        Country::Egypt => Theme {
            sky: Color::srgb(0.98, 0.86, 0.62),
            ground: Color::srgb(0.72, 0.53, 0.3),
            surface: Color::srgb(0.86, 0.7, 0.42),
            rock: Color::srgb(0.6, 0.47, 0.32),
            ..base
        },
        Country::Australia => Theme {
            sky: Color::srgb(0.6, 0.8, 0.96),
            ground: Color::srgb(0.6, 0.27, 0.15),
            surface: Color::srgb(0.8, 0.42, 0.2),
            rock: Color::srgb(0.47, 0.3, 0.24),
            ..base
        },
        Country::Usa => Theme {
            sky: Color::srgb(0.98, 0.78, 0.6),
            ground: Color::srgb(0.55, 0.3, 0.2),
            surface: Color::srgb(0.75, 0.5, 0.3),
            rock: Color::srgb(0.45, 0.3, 0.26),
            ..base
        },
        Country::Paris => Theme {
            sky: Color::srgb(0.66, 0.74, 0.9),
            ground: Color::srgb(0.84, 0.78, 0.66),
            surface: Color::srgb(0.38, 0.43, 0.5),
            rock: Color::srgb(0.5, 0.46, 0.44),
            windows: Some(Color::srgb(0.32, 0.38, 0.5)),
            ..base
        },
        Country::Switzerland => Theme {
            sky: Color::srgb(0.52, 0.76, 0.98),
            ground: Color::srgb(0.42, 0.36, 0.32),
            surface: Color::srgb(0.36, 0.66, 0.32),
            snow_line: Some(450.0),
            rock: Color::srgb(0.48, 0.48, 0.52),
            ..base
        },
    }
}

/// Moves with the camera by `1 - factor`: a factor of 0 stays put on screen
/// like the sun, a factor of 1 would be part of the course.
#[derive(Component, Clone, Copy, Debug)]
pub struct Parallax {
    pub factor: Vec2,
    /// The camera height at which the layer's local origin is at the screen
    /// centre.
    pub reference_y: f32,
}

impl Parallax {
    /// Where the layer's origin goes for a camera at `camera`.
    pub fn offset(&self, camera: Vec2) -> Vec2 {
        Vec2::new(
            camera.x * (1.0 - self.factor.x),
            camera.y * (1.0 - self.factor.y) + self.factor.y * self.reference_y,
        )
    }
}

/// Draw depth of the two layers, behind everything on the course.
const FAR_Z: f32 = -30.0;
const NEAR_Z: f32 = -20.0;

pub struct BackdropPlugin;

impl Plugin for BackdropPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_backdrop).add_systems(
            PostUpdate,
            follow_camera
                .after(crate::camera::CameraSystems)
                .before(TransformSystems::Propagate),
        );
    }
}

/// Triangles collected per colour, so each colour becomes one mesh.
#[derive(Default)]
pub struct Canvas {
    groups: Vec<(Color, Vec<[f32; 3]>, Vec<u32>)>,
}

impl Canvas {
    fn group(&mut self, color: Color) -> &mut (Color, Vec<[f32; 3]>, Vec<u32>) {
        let index = match self.groups.iter().position(|(c, ..)| *c == color) {
            Some(index) => index,
            None => {
                self.groups.push((color, Vec::new(), Vec::new()));
                self.groups.len() - 1
            }
        };
        &mut self.groups[index]
    }

    pub fn polygon(&mut self, color: Color, points: &[Vec2]) {
        let (_, positions, indices) = self.group(color);
        let base = positions.len() as u32;
        positions.extend(points.iter().map(|p| [p.x, p.y, 0.0]));
        for i in 1..points.len().saturating_sub(1) as u32 {
            indices.extend([base, base + i, base + i + 1]);
        }
    }

    pub fn triangle(&mut self, color: Color, a: Vec2, b: Vec2, c: Vec2) {
        self.polygon(color, &[a, b, c]);
    }

    pub fn rect(&mut self, color: Color, rect: Rect) {
        self.polygon(
            color,
            &[
                rect.min,
                Vec2::new(rect.max.x, rect.min.y),
                rect.max,
                Vec2::new(rect.min.x, rect.max.y),
            ],
        );
    }

    pub fn circle(&mut self, color: Color, center: Vec2, radius: f32) {
        const SEGMENTS: usize = 20;
        let points: Vec<Vec2> = (0..SEGMENTS)
            .map(|i| {
                center
                    + radius * Vec2::from_angle(i as f32 / SEGMENTS as f32 * std::f32::consts::TAU)
            })
            .collect();
        self.polygon(color, &points);
    }

    /// The area under a contour, down to `floor`.
    pub fn fill_below(&mut self, color: Color, contour: &[Vec2], floor: f32) {
        for pair in contour.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            self.polygon(color, &[Vec2::new(a.x, floor), Vec2::new(b.x, floor), b, a]);
        }
    }

    /// The meshes, back to front, with their colours.
    pub fn meshes(self) -> Vec<(Color, Mesh)> {
        self.groups
            .into_iter()
            .filter(|(_, positions, _)| !positions.is_empty())
            .map(|(color, positions, indices)| {
                let count = positions.len();
                let mesh = Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::default(),
                )
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
                .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; count])
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; count])
                .with_inserted_indices(Indices::U32(indices));
                (color, mesh)
            })
            .collect()
    }
}

/// A repeatable pseudo-random number in `0.0..1.0`; the same inputs always
/// draw the same backdrop.
pub fn noise(seed: u32, i: i32) -> f32 {
    let mut h = (i as u32).wrapping_mul(0x9E37_79B9) ^ seed.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h & 0xFFFF) as f32 / 65536.0
}

/// A contour of `count` points from `left`, `step` apart, between `low` and
/// `high`.
fn ridge(seed: u32, left: f32, right: f32, step: f32, low: f32, high: f32) -> Vec<Vec2> {
    let count = ((right - left) / step).ceil() as i32 + 1;
    (0..=count)
        .map(|i| {
            let y = low + (high - low) * noise(seed, i);
            Vec2::new(left + i as f32 * step, y)
        })
        .collect()
}

/// A smooth contour of rolling hills or dunes.
fn rolling(seed: u32, left: f32, right: f32, wave: f32, low: f32, high: f32) -> Vec<Vec2> {
    let tops = ridge(seed, left, right, wave, low, high);
    let mut points = Vec::new();
    for pair in tops.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        for step in 0..8 {
            let t = step as f32 / 8.0;
            let ease = 0.5 - 0.5 * (std::f32::consts::PI * t).cos();
            points.push(Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * ease));
        }
    }
    points.extend(tops.last());
    points
}

/// The local x range a layer must cover for the whole level and wide windows.
fn span(level: &Level, factor: f32) -> (f32, f32) {
    const MARGIN: f32 = 1800.0;
    (
        level.bounds.min.x * factor - MARGIN,
        level.bounds.max.x * factor + MARGIN,
    )
}

/// Positions of props along a layer, roughly `gap` apart.
fn spots(seed: u32, (left, right): (f32, f32), gap: f32) -> impl Iterator<Item = (i32, f32)> {
    let count = ((right - left) / gap) as i32;
    (0..count).map(move |i| (i, left + (i as f32 + 0.2 + 0.6 * noise(seed, i)) * gap))
}

/// Mixes `color` towards the sky so far shapes look hazy.
fn haze(color: Color, sky: Color, amount: f32) -> Color {
    color.mix(&sky, amount)
}

fn tree(canvas: &mut Canvas, trunk: Color, crown: Color, at: Vec2, size: f32) {
    canvas.rect(
        trunk,
        Rect::new(
            at.x - 0.06 * size,
            at.y - 400.0,
            at.x + 0.06 * size,
            at.y + 0.6 * size,
        ),
    );
    canvas.circle(crown, at + Vec2::new(0.0, 0.75 * size), 0.38 * size);
    canvas.circle(crown, at + Vec2::new(-0.3 * size, 0.55 * size), 0.26 * size);
    canvas.circle(crown, at + Vec2::new(0.3 * size, 0.6 * size), 0.28 * size);
}

fn palm(canvas: &mut Canvas, trunk: Color, leaves: Color, at: Vec2, size: f32) {
    let top = at + Vec2::new(0.15 * size, size);
    canvas.polygon(
        trunk,
        &[
            Vec2::new(at.x - 0.05 * size, at.y - 400.0),
            Vec2::new(at.x + 0.05 * size, at.y - 400.0),
            top + Vec2::new(0.03 * size, 0.0),
            top - Vec2::new(0.03 * size, 0.0),
        ],
    );
    for angle in [0.2f32, 0.9, 1.6, 2.3, 2.95] {
        let tip = top + 0.55 * size * Vec2::from_angle(angle) - Vec2::Y * 0.12 * size;
        let side = (tip - top).perp().normalize_or_zero() * 0.07 * size;
        canvas.triangle(leaves, top + side, top - side, tip);
    }
}

fn saguaro(canvas: &mut Canvas, color: Color, at: Vec2, size: f32) {
    let w = 0.14 * size;
    canvas.rect(
        color,
        Rect::new(at.x - w, at.y - 400.0, at.x + w, at.y + size),
    );
    canvas.circle(color, at + Vec2::Y * size, w);
    for (side, height) in [(-1.0, 0.45), (1.0, 0.6)] {
        let elbow = at + Vec2::new(side * 0.35 * size, height * size);
        canvas.rect(
            color,
            Rect::from_corners(
                at + Vec2::new(0.0, height * size - w * 0.8),
                elbow + Vec2::new(0.0, w * 0.8),
            ),
        );
        canvas.rect(
            color,
            Rect::new(
                elbow.x - w * 0.8,
                elbow.y,
                elbow.x + w * 0.8,
                elbow.y + 0.3 * size,
            ),
        );
        canvas.circle(color, elbow + Vec2::Y * 0.3 * size, w * 0.8);
    }
}

fn fir(canvas: &mut Canvas, trunk: Color, needles: Color, at: Vec2, size: f32) {
    canvas.rect(
        trunk,
        Rect::new(
            at.x - 0.05 * size,
            at.y - 400.0,
            at.x + 0.05 * size,
            at.y + 0.2 * size,
        ),
    );
    for (i, width) in [0.42f32, 0.34, 0.25].into_iter().enumerate() {
        let base = at.y + 0.15 * size + i as f32 * 0.28 * size;
        canvas.triangle(
            needles,
            Vec2::new(at.x - width * size, base),
            Vec2::new(at.x + width * size, base),
            Vec2::new(at.x, base + 0.5 * size),
        );
    }
}

fn cloud(canvas: &mut Canvas, color: Color, at: Vec2, size: f32) {
    canvas.circle(color, at, 0.5 * size);
    canvas.circle(color, at + Vec2::new(0.55 * size, -0.1 * size), 0.38 * size);
    canvas.circle(
        color,
        at + Vec2::new(-0.55 * size, -0.12 * size),
        0.34 * size,
    );
}

fn eiffel_tower(canvas: &mut Canvas, color: Color, at: Vec2, height: f32) {
    let w = 0.32 * height;
    // Four legs meeting at the first platform, with the arch between them.
    let platform = at.y + 0.28 * height;
    canvas.polygon(
        color,
        &[
            Vec2::new(at.x - w, at.y),
            Vec2::new(at.x - 0.62 * w, at.y),
            Vec2::new(at.x - 0.38 * w, platform),
            Vec2::new(at.x - 0.55 * w, platform),
        ],
    );
    canvas.polygon(
        color,
        &[
            Vec2::new(at.x + 0.62 * w, at.y),
            Vec2::new(at.x + w, at.y),
            Vec2::new(at.x + 0.55 * w, platform),
            Vec2::new(at.x + 0.38 * w, platform),
        ],
    );
    canvas.rect(
        color,
        Rect::new(
            at.x - 0.62 * w,
            platform - 0.03 * height,
            at.x + 0.62 * w,
            platform + 0.03 * height,
        ),
    );
    let second = at.y + 0.55 * height;
    canvas.polygon(
        color,
        &[
            Vec2::new(at.x - 0.5 * w, platform),
            Vec2::new(at.x + 0.5 * w, platform),
            Vec2::new(at.x + 0.2 * w, second),
            Vec2::new(at.x - 0.2 * w, second),
        ],
    );
    canvas.rect(
        color,
        Rect::new(
            at.x - 0.28 * w,
            second - 0.02 * height,
            at.x + 0.28 * w,
            second + 0.02 * height,
        ),
    );
    canvas.triangle(
        color,
        Vec2::new(at.x - 0.2 * w, second),
        Vec2::new(at.x + 0.2 * w, second),
        Vec2::new(at.x, at.y + height),
    );
}

/// Haussmann-style roofs: blocks with sloped tops and chimneys.
fn rooftops(
    canvas: &mut Canvas,
    seed: u32,
    wall: Color,
    roof: Color,
    span: (f32, f32),
    low: f32,
    high: f32,
) {
    let mut x = span.0;
    let mut i = 0;
    while x < span.1 {
        let width = 160.0 + 140.0 * noise(seed, i);
        let top = low + (high - low) * noise(seed + 1, i);
        canvas.rect(wall, Rect::new(x, top - 3000.0, x + width, top));
        canvas.polygon(
            roof,
            &[
                Vec2::new(x, top),
                Vec2::new(x + width, top),
                Vec2::new(x + width - 22.0, top + 40.0),
                Vec2::new(x + 22.0, top + 40.0),
            ],
        );
        let chimney = x + 30.0 + (width - 70.0) * noise(seed + 2, i);
        canvas.rect(
            roof,
            Rect::new(chimney, top + 20.0, chimney + 14.0, top + 62.0),
        );
        x += width + 6.0;
        i += 1;
    }
}

/// The two layers of a country's backdrop: far, then near.
pub fn layers(level: &Level, theme: &Theme) -> [(Parallax, Canvas); 2] {
    let reference_y = level.start.y + 60.0;
    let far = Parallax {
        factor: Vec2::new(0.12, 0.06),
        reference_y,
    };
    let near = Parallax {
        factor: Vec2::new(0.35, 0.15),
        reference_y,
    };
    let far_span = span(level, far.factor.x);
    let near_span = span(level, near.factor.x);
    let sky = theme.sky;
    let mut back = Canvas::default();
    let mut front = Canvas::default();
    let white = Color::srgba(1.0, 1.0, 1.0, 0.85);
    let floor = -4000.0;
    match level.country {
        Country::Training => {
            for (i, x) in spots(11, far_span, 700.0) {
                cloud(
                    &mut back,
                    white,
                    Vec2::new(x, 150.0 + 90.0 * noise(12, i)),
                    90.0,
                );
            }
            let hills = rolling(13, far_span.0, far_span.1, 420.0, -40.0, 90.0);
            back.fill_below(haze(Color::srgb(0.3, 0.6, 0.35), sky, 0.55), &hills, floor);
            let near_hills = rolling(14, near_span.0, near_span.1, 360.0, -70.0, 10.0);
            front.fill_below(
                haze(Color::srgb(0.3, 0.58, 0.28), sky, 0.3),
                &near_hills,
                floor,
            );
            for (i, x) in spots(15, near_span, 380.0) {
                let ground = sample(&near_hills, x);
                tree(
                    &mut front,
                    haze(Color::srgb(0.35, 0.25, 0.18), sky, 0.3),
                    haze(Color::srgb(0.2, 0.48, 0.22), sky, 0.25),
                    Vec2::new(x, ground - 10.0),
                    90.0 + 40.0 * noise(16, i),
                );
            }
        }
        Country::Egypt => {
            back.circle(
                haze(Color::srgb(1.0, 0.95, 0.7), sky, 0.1),
                Vec2::new(level.bounds.center().x * far.factor.x, 190.0),
                70.0,
            );
            let dunes = rolling(21, far_span.0, far_span.1, 600.0, -50.0, 10.0);
            let stone = haze(Color::srgb(0.78, 0.6, 0.38), sky, 0.45);
            let shade = haze(Color::srgb(0.66, 0.48, 0.3), sky, 0.45);
            for (i, x) in spots(22, far_span, 900.0) {
                let base = sample(&dunes, x) - 10.0;
                let size = 160.0 + 120.0 * noise(23, i);
                back.triangle(
                    stone,
                    Vec2::new(x - size, base),
                    Vec2::new(x + size, base),
                    Vec2::new(x, base + size * 0.95),
                );
                back.triangle(
                    shade,
                    Vec2::new(x, base),
                    Vec2::new(x + size, base),
                    Vec2::new(x, base + size * 0.95),
                );
            }
            back.fill_below(haze(Color::srgb(0.9, 0.74, 0.48), sky, 0.4), &dunes, floor);
            let near_dunes = rolling(24, near_span.0, near_span.1, 450.0, -80.0, -20.0);
            front.fill_below(
                haze(Color::srgb(0.88, 0.7, 0.42), sky, 0.2),
                &near_dunes,
                floor,
            );
            for (i, x) in spots(25, near_span, 520.0) {
                if noise(26, i) < 0.6 {
                    let ground = sample(&near_dunes, x);
                    palm(
                        &mut front,
                        haze(Color::srgb(0.5, 0.35, 0.2), sky, 0.25),
                        haze(Color::srgb(0.25, 0.5, 0.2), sky, 0.25),
                        Vec2::new(x, ground - 8.0),
                        120.0 + 40.0 * noise(27, i),
                    );
                }
            }
        }
        Country::Australia => {
            for (i, x) in spots(31, far_span, 900.0) {
                cloud(
                    &mut back,
                    white,
                    Vec2::new(x, 170.0 + 70.0 * noise(32, i)),
                    70.0,
                );
            }
            let plains = rolling(33, far_span.0, far_span.1, 700.0, -40.0, -10.0);
            let rock = haze(Color::srgb(0.75, 0.33, 0.16), sky, 0.35);
            for (i, x) in spots(34, far_span, 1600.0) {
                let base = sample(&plains, x) - 10.0;
                let width = 260.0 + 160.0 * noise(35, i);
                let height = 120.0 + 40.0 * noise(36, i);
                let outline: Vec<Vec2> = (0..=16)
                    .map(|k| {
                        let a = std::f32::consts::PI * k as f32 / 16.0;
                        let round = a.sin().powf(0.35);
                        Vec2::new(x + width * a.cos(), base + height * round)
                    })
                    .collect();
                back.fill_below(rock, &outline.into_iter().rev().collect::<Vec<_>>(), base);
            }
            back.fill_below(
                haze(Color::srgb(0.82, 0.48, 0.25), sky, 0.45),
                &plains,
                floor,
            );
            let near_plains = rolling(37, near_span.0, near_span.1, 500.0, -75.0, -35.0);
            front.fill_below(
                haze(Color::srgb(0.78, 0.42, 0.22), sky, 0.25),
                &near_plains,
                floor,
            );
            for (i, x) in spots(38, near_span, 420.0) {
                let ground = sample(&near_plains, x);
                if noise(39, i) < 0.55 {
                    tree(
                        &mut front,
                        haze(Color::srgb(0.9, 0.88, 0.8), sky, 0.3),
                        haze(Color::srgb(0.45, 0.58, 0.4), sky, 0.3),
                        Vec2::new(x, ground - 8.0),
                        110.0 + 40.0 * noise(40, i),
                    );
                } else {
                    front.circle(
                        haze(Color::srgb(0.6, 0.6, 0.35), sky, 0.3),
                        Vec2::new(x, ground + 6.0),
                        24.0,
                    );
                }
            }
        }
        Country::Usa => {
            back.circle(
                haze(Color::srgb(1.0, 0.9, 0.6), sky, 0.1),
                Vec2::new(level.bounds.center().x * far.factor.x - 300.0, 120.0),
                80.0,
            );
            let mut mesas = vec![Vec2::new(far_span.0, -50.0)];
            let mut x = far_span.0;
            let mut i = 0;
            while x < far_span.1 {
                let gap = 80.0 + 300.0 * noise(41, i);
                let top = 30.0 + 150.0 * noise(42, i);
                let width = 180.0 + 240.0 * noise(43, i);
                x += gap;
                mesas.extend([
                    Vec2::new(x, -50.0),
                    Vec2::new(x + 70.0, top),
                    Vec2::new(x + 70.0 + width, top),
                    Vec2::new(x + 140.0 + width, -50.0),
                ]);
                x += 140.0 + width;
                i += 1;
            }
            back.fill_below(haze(Color::srgb(0.72, 0.36, 0.22), sky, 0.4), &mesas, floor);
            let near_desert = rolling(44, near_span.0, near_span.1, 520.0, -75.0, -35.0);
            front.fill_below(
                haze(Color::srgb(0.8, 0.55, 0.32), sky, 0.25),
                &near_desert,
                floor,
            );
            for (i, x) in spots(45, near_span, 360.0) {
                let ground = sample(&near_desert, x);
                saguaro(
                    &mut front,
                    haze(Color::srgb(0.42, 0.5, 0.36), sky, 0.5),
                    Vec2::new(x, ground - 8.0),
                    90.0 + 50.0 * noise(46, i),
                );
            }
        }
        Country::Paris => {
            for (i, x) in spots(51, far_span, 800.0) {
                cloud(
                    &mut back,
                    white,
                    Vec2::new(x, 190.0 + 60.0 * noise(52, i)),
                    60.0,
                );
            }
            let skyline = haze(Color::srgb(0.48, 0.52, 0.64), sky, 0.45);
            let mut x = far_span.0;
            let mut i = 0;
            let mut contour = Vec::new();
            while x < far_span.1 {
                let width = 90.0 + 90.0 * noise(53, i);
                let top = -20.0 + 90.0 * noise(54, i);
                contour.extend([Vec2::new(x, top), Vec2::new(x + width, top)]);
                x += width;
                i += 1;
            }
            back.fill_below(skyline, &contour, floor);
            let towers = ((far_span.1 - far_span.0) / 2600.0).max(1.0) as i32;
            for k in 0..towers {
                let x = far_span.0 + 1800.0 + k as f32 * 2600.0;
                eiffel_tower(
                    &mut back,
                    haze(Color::srgb(0.32, 0.3, 0.32), sky, 0.35),
                    Vec2::new(x, -30.0),
                    420.0,
                );
            }
            rooftops(
                &mut front,
                55,
                haze(Color::srgb(0.78, 0.72, 0.62), sky, 0.3),
                haze(Color::srgb(0.38, 0.42, 0.5), sky, 0.3),
                near_span,
                -70.0,
                20.0,
            );
        }
        Country::Switzerland => {
            for (i, x) in spots(61, far_span, 850.0) {
                cloud(
                    &mut back,
                    white,
                    Vec2::new(x, 260.0 + 60.0 * noise(62, i)),
                    70.0,
                );
            }
            let peaks = ridge(63, far_span.0, far_span.1, 230.0, -60.0, 260.0);
            let mut zigzag = Vec::new();
            for (i, point) in peaks.iter().enumerate() {
                let y = if i % 2 == 0 {
                    point.y.min(40.0) - 60.0
                } else {
                    point.y + 80.0
                };
                zigzag.push(Vec2::new(point.x, y));
            }
            back.fill_below(haze(Color::srgb(0.45, 0.5, 0.6), sky, 0.4), &zigzag, floor);
            for trio in zigzag.windows(3) {
                let (a, peak, c) = (trio[0], trio[1], trio[2]);
                if peak.y > a.y && peak.y > c.y && peak.y > 120.0 {
                    let k = 0.35;
                    back.triangle(
                        haze(SNOW, sky, 0.15),
                        peak.lerp(a, k),
                        peak.lerp(c, k),
                        peak,
                    );
                }
            }
            let meadows = rolling(64, near_span.0, near_span.1, 380.0, -80.0, 30.0);
            front.fill_below(
                haze(Color::srgb(0.32, 0.6, 0.3), sky, 0.25),
                &meadows,
                floor,
            );
            for (i, x) in spots(65, near_span, 300.0) {
                let ground = sample(&meadows, x);
                if noise(66, i) < 0.85 {
                    fir(
                        &mut front,
                        haze(Color::srgb(0.35, 0.25, 0.18), sky, 0.3),
                        haze(Color::srgb(0.12, 0.38, 0.2), sky, 0.25),
                        Vec2::new(x, ground - 10.0),
                        90.0 + 50.0 * noise(67, i),
                    );
                } else {
                    let wood = haze(Color::srgb(0.55, 0.35, 0.2), sky, 0.25);
                    let roof = haze(Color::srgb(0.35, 0.22, 0.15), sky, 0.25);
                    front.rect(
                        wood,
                        Rect::new(x - 50.0, ground - 200.0, x + 50.0, ground + 50.0),
                    );
                    front.triangle(
                        roof,
                        Vec2::new(x - 68.0, ground + 45.0),
                        Vec2::new(x + 68.0, ground + 45.0),
                        Vec2::new(x, ground + 95.0),
                    );
                    front.rect(
                        haze(SNOW, sky, 0.3),
                        Rect::new(x - 20.0, ground + 5.0, x + 20.0, ground + 30.0),
                    );
                }
            }
        }
    }
    [(far, back), (near, front)]
}

/// The height of a contour at `x`.
fn sample(contour: &[Vec2], x: f32) -> f32 {
    contour
        .windows(2)
        .find(|pair| (pair[0].x..=pair[1].x).contains(&x))
        .map_or(0.0, |pair| {
            let t = (x - pair[0].x) / (pair[1].x - pair[0].x).max(f32::EPSILON);
            pair[0].y + (pair[1].y - pair[0].y) * t
        })
}

fn spawn_backdrop(
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
        let theme = theme(level.country);
        commands.insert_resource(ClearColor(theme.sky));
        for (z, (parallax, canvas)) in [FAR_Z, NEAR_Z].into_iter().zip(layers(&level, &theme)) {
            commands
                .spawn((
                    VisualOf(root),
                    parallax,
                    Transform::from_xyz(0.0, 0.0, z),
                    Visibility::default(),
                ))
                .with_children(|parent| {
                    for (i, (color, mesh)) in canvas.meshes().into_iter().enumerate() {
                        parent.spawn((
                            Mesh2d(meshes.add(mesh)),
                            MeshMaterial2d(materials.add(color)),
                            Transform::from_xyz(0.0, 0.0, i as f32 * 0.01),
                        ));
                    }
                });
        }
    }
}

fn follow_camera(
    cameras: Query<&Transform, (With<Camera2d>, Without<Parallax>)>,
    mut layers: Query<(&Parallax, &mut Transform)>,
) {
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    for (parallax, mut transform) in &mut layers {
        let offset = parallax.offset(camera.translation.truncate());
        transform.translation.x = offset.x;
        transform.translation.y = offset.y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn far_layers_move_less_than_near_ones() {
        let far = Parallax {
            factor: Vec2::new(0.1, 0.0),
            reference_y: 0.0,
        };
        let near = Parallax {
            factor: Vec2::new(0.4, 0.0),
            reference_y: 0.0,
        };
        let camera = Vec2::new(1000.0, 0.0);
        // On screen, a layer shifts left by `factor * camera.x`.
        assert!((camera.x - far.offset(camera).x) < (camera.x - near.offset(camera).x));
        assert_eq!(far.offset(Vec2::ZERO), Vec2::ZERO);
    }

    #[test]
    fn noise_is_repeatable_and_in_range() {
        for i in -50..50 {
            let value = noise(7, i);
            assert!((0.0..1.0).contains(&value));
            assert_eq!(value, noise(7, i));
        }
        assert_ne!(noise(7, 1), noise(8, 1));
    }

    #[test]
    fn every_country_has_a_backdrop_covering_the_whole_level() {
        for level in crate::levels::all() {
            let theme = theme(level.country);
            for (parallax, canvas) in layers(&level, &theme) {
                let (left, right) = span(&level, parallax.factor.x);
                let mut min = f32::MAX;
                let mut max = f32::MIN;
                let meshes = canvas.meshes();
                assert!(!meshes.is_empty(), "{}", level.id);
                for (_, mesh) in meshes {
                    let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
                        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                    else {
                        panic!("positions");
                    };
                    for p in positions {
                        min = min.min(p[0]);
                        max = max.max(p[0]);
                    }
                }
                assert!(
                    min <= left + 400.0 && max >= right - 400.0,
                    "{}: {min}..{max} for {left}..{right}",
                    level.id
                );
            }
        }
    }
}
