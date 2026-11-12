//! Level data: a ground profile, extra solids, loops, spikes, traps, checkpoints,
//! the start, and the finish.
//!
//! A level is plain data in world units with y pointing up. Nothing here spawns
//! entities, so the geometry helpers are tested directly. The concrete levels live
//! in [`crate::levels`].

use bevy::prelude::*;

/// Width of one spike.
pub const SPIKE_WIDTH: f32 = 24.0;
/// Height of a raised spike.
pub const SPIKE_HEIGHT: f32 = 28.0;

/// A row of spikes standing on `base`, starting at `left`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spikes {
    pub left: f32,
    pub base: f32,
    pub count: u32,
}

impl Spikes {
    pub fn width(&self) -> f32 {
        self.count as f32 * SPIKE_WIDTH
    }

    /// The deadly area when the spikes are raised by `raised` (0 = hidden, 1 =
    /// fully up). It is a little lower than the tips so that grazing the very
    /// point of a spike does not count.
    pub fn hazard(&self, raised: f32) -> Rect {
        Rect::new(
            self.left + 2.0,
            self.base,
            self.left + self.width() - 2.0,
            self.base + raised * 0.75 * SPIKE_HEIGHT,
        )
    }
}

/// A vertical loop whose inner surface has `radius` and whose bottom touches the
/// ground at `center.y - radius`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Loop {
    pub center: Vec2,
    pub radius: f32,
}

/// The country a level belongs to. A country's levels are played in a row, and
/// presentation draws each country with its own sky, ground, and backdrop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Country {
    /// The first three levels, where the shared traps are learned.
    #[default]
    Training,
    Egypt,
    Australia,
    Usa,
    Paris,
    Switzerland,
}

impl Country {
    /// Shown to the player.
    pub fn name(self) -> &'static str {
        match self {
            Self::Training => "Übungsgelände",
            Self::Egypt => "Ägypten",
            Self::Australia => "Australien",
            Self::Usa => "USA",
            Self::Paris => "Paris",
            Self::Switzerland => "Schweiz",
        }
    }
}

/// What a trap does once the bike enters its trigger zone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrapKind {
    /// A solid slab that drops away shortly after the bike lands on it.
    DropSlab { slab: Rect },
    /// Hidden spikes that shoot up out of the ground.
    PopUpSpikes { spikes: Spikes },
    /// A pillar hanging from the ceiling that crashes down to `floor`, waits, and
    /// rises again.
    FallingPillar { pillar: Rect, floor: f32 },
    /// A block below the ground that rises, waits, and retracts.
    RisingPillar { pillar: Rect, rise: f32 },
    /// A fixed dive followed by a climb out of the course.
    Raven { from: Vec2, strike: Vec2 },
    /// A scripted rolling/bouncing ball; travel bounds its lifetime.
    Ball {
        start: Vec2,
        radius: f32,
        speed: f32,
        bounce_height: f32,
        travel: f32,
    },
    /// A TNT crate that explodes after a short fuse.
    Tnt { crate_box: Rect, blast_radius: f32 },
    /// The finish flag jumps to `to`, usually behind the bike.
    JumpingFinish { to: Rect },
    /// A snow slab breaks loose at `release`, slides down to the course at
    /// `from_x`, and rolls along the course surface to `to_x`, where it
    /// dissolves. `to_x < from_x` makes it come towards a bike riding right.
    Avalanche {
        release: Vec2,
        from_x: f32,
        to_x: f32,
        radius: f32,
        speed: f32,
    },
}

/// A trap and the invisible zone that fires it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrapDef {
    pub kind: TrapKind,
    pub trigger: Rect,
}

/// Touching `zone` makes `respawn` the new respawn point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Checkpoint {
    pub zone: Rect,
    pub respawn: Vec2,
}

/// One level. `ground` is a profile from left to right: its x values never
/// decrease, and vertical steps (two points with the same x) make walls and pits.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct Level {
    /// Stable identifier used in the save file; never reuse or renumber.
    pub id: &'static str,
    /// Shown to the player.
    pub name: &'static str,
    pub country: Country,
    pub ground: Vec<Vec2>,
    /// Extra solid blocks, such as a ceiling.
    pub solids: Vec<Rect>,
    pub loops: Vec<Loop>,
    /// Spikes that are always out.
    pub spikes: Vec<Spikes>,
    pub traps: Vec<TrapDef>,
    pub checkpoints: Vec<Checkpoint>,
    pub effects: Vec<crate::effects::EffectDef>,
    /// The ground point below the middle of the bike at the start.
    pub start: Vec2,
    /// Touching this zone finishes the level (unless a trap moves it).
    pub finish: Rect,
    /// The area the camera may show.
    pub bounds: Rect,
    /// A bike whose body sinks below this height has fallen out.
    pub fall_y: f32,
}

impl Level {
    /// The height of the ground profile at `x`, or `None` outside the profile.
    /// At a vertical step the higher side counts.
    pub fn ground_height(&self, x: f32) -> Option<f32> {
        self.ground
            .windows(2)
            .filter_map(|pair| {
                let (a, b) = (pair[0], pair[1]);
                if x < a.x || x > b.x {
                    return None;
                }
                if b.x - a.x <= f32::EPSILON {
                    return Some(a.y.max(b.y));
                }
                Some(a.y + (b.y - a.y) * (x - a.x) / (b.x - a.x))
            })
            .reduce(f32::max)
    }

    /// The height an object rolling along the course rests on at `x`: the ground,
    /// or the top of a solid above it, such as an avalanche gallery's roof.
    pub fn surface_height(&self, x: f32) -> Option<f32> {
        let roofs = self
            .solids
            .iter()
            .filter(|solid| (solid.min.x..=solid.max.x).contains(&x))
            .map(|solid| solid.max.y);
        self.ground_height(x)
            .into_iter()
            .chain(roofs)
            .reduce(f32::max)
    }

    /// The left and right end of the level's ground.
    pub fn x_range(&self) -> (f32, f32) {
        let first = self.ground.first().map_or(0.0, |p| p.x);
        let last = self.ground.last().map_or(0.0, |p| p.x);
        (first, last)
    }

    /// Checks the rules every level must follow; the built-in levels are tested
    /// with it.
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() || self.name.is_empty() {
            return Err("a level needs an id and a name".into());
        }
        if self.ground.len() < 2 {
            return Err(format!("{}: the ground needs at least two points", self.id));
        }
        if self.ground.windows(2).any(|pair| pair[1].x < pair[0].x) {
            return Err(format!("{}: ground x values must not decrease", self.id));
        }
        let inside = |point: Vec2| self.bounds.contains(point);
        if !inside(self.start) {
            return Err(format!("{}: the start is outside the bounds", self.id));
        }
        match self.ground_height(self.start.x) {
            Some(height) if (height - self.start.y).abs() < 1.0 => {}
            _ => return Err(format!("{}: the start is not on the ground", self.id)),
        }
        if !inside(self.finish.center()) {
            return Err(format!("{}: the finish is outside the bounds", self.id));
        }
        if self.fall_y >= self.start.y {
            return Err(format!("{}: the fall line is above the start", self.id));
        }
        for trap in &self.traps {
            if !inside(trap.trigger.center()) {
                return Err(format!("{}: a trap trigger is outside the bounds", self.id));
            }
        }
        for checkpoint in &self.checkpoints {
            if !inside(checkpoint.respawn) {
                return Err(format!("{}: a checkpoint is outside the bounds", self.id));
            }
        }
        Ok(())
    }
}

/// Builds a ground profile from left to right.
#[derive(Default)]
pub struct Profile(Vec<Vec2>);

impl Profile {
    pub fn start(x: f32, y: f32) -> Self {
        Self(vec![Vec2::new(x, y)])
    }

    /// A straight line to `(x, y)`.
    pub fn to(mut self, x: f32, y: f32) -> Self {
        self.0.push(Vec2::new(x, y));
        self
    }

    /// A smooth bump of `height` from the current point to `x`, ending at the
    /// current height.
    pub fn hill(mut self, x: f32, height: f32) -> Self {
        let from = *self.0.last().expect("a profile has a start");
        const STEPS: usize = 12;
        for step in 1..=STEPS {
            let t = step as f32 / STEPS as f32;
            let lift = 0.5 * (1.0 - (std::f32::consts::TAU * t).cos());
            self.0
                .push(Vec2::new(from.x + (x - from.x) * t, from.y + height * lift));
        }
        self
    }

    /// A pit from `left` to `right` with its bottom at `bottom`; the ground
    /// continues at the current height behind it.
    pub fn pit(self, left: f32, right: f32, bottom: f32) -> Self {
        let y = self.0.last().expect("a profile has a start").y;
        self.to(left, y)
            .to(left, bottom)
            .to(right, bottom)
            .to(right, y)
    }

    pub fn build(self) -> Vec<Vec2> {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat() -> Level {
        Level {
            id: "test",
            name: "Test",
            country: Country::Training,
            ground: Profile::start(0.0, 0.0)
                .to(100.0, 0.0)
                .to(100.0, -50.0)
                .to(200.0, -50.0)
                .to(300.0, 50.0)
                .build(),
            solids: vec![],
            loops: vec![],
            spikes: vec![],
            traps: vec![],
            checkpoints: vec![],
            effects: vec![],
            start: Vec2::new(50.0, 0.0),
            finish: Rect::new(280.0, 50.0, 290.0, 100.0),
            bounds: Rect::new(0.0, -100.0, 300.0, 300.0),
            fall_y: -80.0,
        }
    }

    #[test]
    fn ground_height_follows_the_profile() {
        let level = flat();
        assert_eq!(level.ground_height(50.0), Some(0.0));
        assert_eq!(
            level.ground_height(100.0),
            Some(0.0),
            "the higher side of a step"
        );
        assert_eq!(level.ground_height(150.0), Some(-50.0));
        assert_eq!(level.ground_height(250.0), Some(0.0));
        assert_eq!(level.ground_height(400.0), None);
    }

    #[test]
    fn validation_catches_broken_levels() {
        assert_eq!(flat().validate(), Ok(()));
        let mut floating = flat();
        floating.start.y = 20.0;
        assert!(floating.validate().is_err());
        let mut backwards = flat();
        backwards.ground.push(Vec2::new(10.0, 0.0));
        assert!(backwards.validate().is_err());
    }

    #[test]
    fn surfaces_include_roofs_above_the_ground() {
        let mut level = flat();
        level.solids.push(Rect::new(20.0, 120.0, 80.0, 150.0));
        assert_eq!(level.surface_height(50.0), Some(150.0));
        assert_eq!(level.surface_height(90.0), Some(0.0));
    }

    #[test]
    fn hills_rise_and_return() {
        let profile = Profile::start(0.0, 0.0).hill(100.0, 40.0).build();
        let top = profile.iter().map(|p| p.y).fold(f32::MIN, f32::max);
        assert!((top - 40.0).abs() < 1e-3);
        assert!(profile.last().unwrap().y.abs() < 1e-3);
    }
}
