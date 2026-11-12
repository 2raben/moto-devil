//! The built-in levels, grouped by country and listed in play order. Each level
//! asks a little more than the one before; see the
//! [Länder pitch](../../docs/pitches/laender/pitch--laender.md).
//!
//! Positions are play-test values: change them in the game, then run the tests
//! that drive a scripted bike through each level.

mod australia;
mod egypt;
mod paris;
mod switzerland;
mod training;
mod usa;

pub use training::{flight_school, level_one, level_two};

use bevy::prelude::*;

use crate::effects::{EffectDef, EffectKind};
use crate::level::{Country, Level, Spikes, TrapDef, TrapKind};

/// All levels in the order they are played. A country's levels follow each
/// other.
pub fn all() -> Vec<Level> {
    let mut levels = vec![level_one(), level_two(), flight_school()];
    levels.extend(egypt::levels());
    levels.extend(australia::levels());
    levels.extend(usa::levels());
    levels.extend(paris::levels());
    levels.extend(switzerland::levels());
    levels
}

/// A level without anything in it; the country modules fill in the rest. The
/// bike starts at `(100, 0)` and falling below `fall_y` crashes.
fn blank(id: &'static str, name: &'static str, country: Country) -> Level {
    Level {
        id,
        name,
        country,
        ground: Vec::new(),
        solids: Vec::new(),
        loops: Vec::new(),
        spikes: Vec::new(),
        traps: Vec::new(),
        checkpoints: Vec::new(),
        effects: Vec::new(),
        start: Vec2::new(100.0, 0.0),
        finish: Rect::default(),
        bounds: Rect::default(),
        fall_y: -100.0,
    }
}

/// A tall trigger zone between two x positions.
fn zone(left: f32, right: f32) -> Rect {
    Rect::new(left, -2000.0, right, 3000.0)
}

/// A finish flag standing on the ground at `x`.
fn finish_at(x: f32, ground: f32) -> Rect {
    Rect::new(x, ground, x + 50.0, ground + 220.0)
}

fn trap(kind: TrapKind, trigger: (f32, f32)) -> TrapDef {
    TrapDef {
        kind,
        trigger: zone(trigger.0, trigger.1),
    }
}

fn pop_up_spikes(left: f32, base: f32, count: u32, trigger: (f32, f32)) -> TrapDef {
    trap(
        TrapKind::PopUpSpikes {
            spikes: Spikes { left, base, count },
        },
        trigger,
    )
}

/// A block hidden high above the course, out of view, that crashes down onto
/// the ground at `x` when the bike enters the trigger: wait for it.
fn sky_block(x: f32, ground: f32, trigger: (f32, f32)) -> TrapDef {
    trap(
        TrapKind::FallingPillar {
            pillar: Rect::new(x, ground + 650.0, x + 60.0, ground + 850.0),
            floor: ground,
        },
        trigger,
    )
}

/// A block hidden in the ground at `x` that shoots up when the bike enters
/// the trigger: wait for it to sink again.
fn rising_block(x: f32, ground: f32, trigger: (f32, f32)) -> TrapDef {
    trap(
        TrapKind::RisingPillar {
            pillar: Rect::new(x, ground - 210.0, x + 60.0, ground - 10.0),
            rise: 210.0,
        },
        trigger,
    )
}

/// The finish flag jumps to `x` when the bike enters the trigger.
fn runaway_finish(x: f32, ground: f32, trigger: (f32, f32)) -> TrapDef {
    trap(
        TrapKind::JumpingFinish {
            to: finish_at(x, ground),
        },
        trigger,
    )
}

fn checkpoint(x: f32, ground: f32) -> crate::level::Checkpoint {
    crate::level::Checkpoint {
        zone: Rect::new(x - 30.0, ground, x + 30.0, ground + 400.0),
        respawn: Vec2::new(x + 80.0, ground),
    }
}

/// A flat effect strip lying on the ground between `left` and `right`.
fn strip(left: f32, right: f32, ground: f32, kind: EffectKind) -> EffectDef {
    EffectDef {
        zone: Rect::new(left, ground, right, ground + 8.0),
        kind,
    }
}

/// A rolling (`bounce_height == 0`) or hopping ball that starts at `x`, comes
/// towards the bike, and is gone after `travel`.
fn ball(x: f32, ground: f32, radius: f32, speed: f32, bounce_height: f32, travel: f32) -> TrapKind {
    TrapKind::Ball {
        start: Vec2::new(x, ground + radius),
        radius,
        speed,
        bounce_height,
        travel,
    }
}

/// Fills in the bounds from the ground so the camera shows the whole course.
fn framed(mut level: Level, top: f32) -> Level {
    let (left, right) = level.x_range();
    let lowest = level.ground.iter().map(|p| p.y).fold(f32::MAX, f32::min);
    level.bounds = Rect::new(left, lowest.min(level.fall_y) - 120.0, right, top);
    level
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_levels_are_valid_and_unique() {
        let levels = all();
        for level in &levels {
            assert_eq!(level.validate(), Ok(()), "{}", level.id);
        }
        for (i, a) in levels.iter().enumerate() {
            for b in &levels[i + 1..] {
                assert_ne!(a.id, b.id);
            }
        }
    }

    #[test]
    fn countries_come_in_a_row_with_three_levels_each() {
        let levels = all();
        let mut order: Vec<(Country, usize)> = Vec::new();
        for level in &levels {
            match order.last_mut() {
                Some((country, count)) if *country == level.country => *count += 1,
                _ => {
                    assert!(
                        order.iter().all(|(c, _)| *c != level.country),
                        "{:?} appears twice",
                        level.country
                    );
                    order.push((level.country, 1));
                }
            }
        }
        assert_eq!(
            order,
            vec![
                (Country::Training, 3),
                (Country::Egypt, 3),
                (Country::Australia, 3),
                (Country::Usa, 3),
                (Country::Paris, 3),
                (Country::Switzerland, 3),
            ]
        );
    }

    #[test]
    fn the_training_levels_keep_their_ids_and_order() {
        let ids: Vec<_> = all().iter().take(3).map(|level| level.id).collect();
        assert_eq!(ids, ["level-1", "level-2", "fallen-flugstunden"]);
    }

    #[test]
    fn level_one_has_a_single_trap_and_no_checkpoint() {
        let level = level_one();
        assert_eq!(level.traps.len(), 1);
        assert!(level.checkpoints.is_empty());
    }

    #[test]
    fn each_country_uses_its_own_trap() {
        let levels = all();
        let has = |country: Country, test: &dyn Fn(&Level) -> bool| {
            levels
                .iter()
                .filter(|level| level.country == country)
                .all(test)
        };
        assert!(has(Country::Egypt, &|level| {
            level
                .effects
                .iter()
                .any(|effect| effect.kind == EffectKind::Quicksand)
        }));
        assert!(has(Country::Switzerland, &|level| {
            level
                .traps
                .iter()
                .any(|trap| matches!(trap.kind, TrapKind::Avalanche { .. }))
        }));
    }
}
