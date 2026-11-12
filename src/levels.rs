//! The built-in levels, in play order. See the pitch sketches `level-1` and
//! `level-trolls` in `docs/pitches/moto-devil/`.
//!
//! Positions are play-test values: change them in the game, then run the tests
//! that drive a scripted bike through each level.

use bevy::prelude::*;

use crate::level::{Checkpoint, Level, Loop, Profile, Spikes, TrapDef, TrapKind};

/// All levels in the order they are played.
pub fn all() -> Vec<Level> {
    vec![level_one(), level_two(), flight_school()]
}

/// Very simple: ride, a small hill, a small ramp over a ditch, and one trap.
pub fn level_one() -> Level {
    Level {
        id: "level-1",
        name: "Level 1",
        ground: Profile::start(-100.0, 400.0)
            .to(-100.0, 0.0)
            .to(360.0, 0.0)
            .hill(720.0, 50.0)
            .to(860.0, 0.0)
            .to(980.0, 60.0)
            .to(980.0, -180.0)
            .to(1120.0, -180.0)
            .to(1120.0, 0.0)
            .to(2500.0, 0.0)
            .to(2500.0, 400.0)
            .build(),
        solids: vec![],
        loops: vec![],
        spikes: vec![],
        effects: vec![],
        traps: vec![TrapDef {
            kind: TrapKind::PopUpSpikes {
                spikes: Spikes {
                    left: 1840.0,
                    base: 0.0,
                    count: 4,
                },
            },
            trigger: Rect::new(1700.0, 0.0, 1940.0, 160.0),
        }],
        checkpoints: vec![],
        start: Vec2::new(100.0, 0.0),
        finish: Rect::new(2200.0, 0.0, 2240.0, 220.0),
        bounds: Rect::new(-100.0, -220.0, 2500.0, 600.0),
        fall_y: -100.0,
    }
}

/// Many lies: a dropping landing slab, a loop with spikes behind it, a falling
/// pillar, a spiky tower with TNT behind it, and a finish that jumps behind you.
pub fn level_two() -> Level {
    Level {
        id: "level-2",
        name: "Level 2",
        ground: Profile::start(-100.0, 400.0)
            .to(-100.0, 0.0)
            .to(560.0, 0.0)
            .to(720.0, 90.0)
            .to(720.0, -170.0)
            .to(1060.0, -170.0)
            .to(1060.0, 40.0)
            .to(2250.0, 40.0)
            .to(2380.0, 120.0)
            .to(2380.0, -170.0)
            .to(2650.0, -170.0)
            .to(2650.0, 60.0)
            .to(2730.0, 60.0)
            .to(2730.0, 0.0)
            .to(3450.0, 0.0)
            .to(3450.0, 400.0)
            .build(),
        solids: vec![Rect::new(1900.0, 450.0, 2300.0, 480.0)],
        loops: vec![Loop {
            center: Vec2::new(1330.0, 200.0),
            radius: 160.0,
        }],
        effects: vec![],
        spikes: vec![
            Spikes {
                left: 730.0,
                base: -170.0,
                count: 13,
            },
            Spikes {
                left: 2390.0,
                base: -170.0,
                count: 10,
            },
            Spikes {
                left: 2654.0,
                base: 60.0,
                count: 3,
            },
        ],
        traps: vec![
            TrapDef {
                kind: TrapKind::DropSlab {
                    slab: Rect::new(950.0, 16.0, 1060.0, 40.0),
                },
                trigger: Rect::new(940.0, 40.0, 1070.0, 140.0),
            },
            TrapDef {
                kind: TrapKind::PopUpSpikes {
                    spikes: Spikes {
                        left: 1700.0,
                        base: 40.0,
                        count: 4,
                    },
                },
                trigger: Rect::new(1600.0, 40.0, 1820.0, 200.0),
            },
            TrapDef {
                kind: TrapKind::FallingPillar {
                    pillar: Rect::new(2070.0, 250.0, 2126.0, 450.0),
                    floor: 40.0,
                },
                trigger: Rect::new(1950.0, 40.0, 2050.0, 240.0),
            },
            TrapDef {
                kind: TrapKind::Tnt {
                    crate_box: Rect::new(2870.0, 0.0, 2926.0, 56.0),
                    blast_radius: 110.0,
                },
                trigger: Rect::new(2740.0, 0.0, 3010.0, 180.0),
            },
            TrapDef {
                kind: TrapKind::JumpingFinish {
                    to: Rect::new(3050.0, 0.0, 3090.0, 220.0),
                },
                trigger: Rect::new(3190.0, 0.0, 3290.0, 200.0),
            },
        ],
        checkpoints: vec![Checkpoint {
            zone: Rect::new(1840.0, 40.0, 1880.0, 240.0),
            respawn: Vec2::new(1910.0, 40.0),
        }],
        start: Vec2::new(100.0, 0.0),
        finish: Rect::new(3310.0, 0.0, 3350.0, 220.0),
        bounds: Rect::new(-100.0, -220.0, 3450.0, 600.0),
        fall_y: -100.0,
    }
}

/// Each section introduces its parts before combining them. All required
/// pickups sit after the section's checkpoint, including the final refill.
pub fn flight_school() -> Level {
    use crate::effects::{EffectDef, EffectKind};
    let effect = |left, right, height, kind| EffectDef {
        zone: Rect::new(left, 0.0, right, height),
        kind,
    };
    let trigger = |left, right| Rect::new(left, 0.0, right, 500.0);
    let mut ground = Profile::start(-100.0, 400.0).to(-100.0, 0.0);
    for (left, right) in [
        (1400.0, 1600.0),
        (2900.0, 3120.0),
        (8500.0, 8750.0),
        (9800.0, 10100.0),
        (12000.0, 13000.0),
        (13900.0, 14900.0),
    ] {
        ground = ground
            .to(left, 0.0)
            .to(left, -300.0)
            .to(right, -300.0)
            .to(right, 0.0);
    }
    Level {
        id: "fallen-flugstunden",
        name: "Fallen & Flugstunden",
        ground: ground.to(15400.0, 0.0).to(15400.0, 400.0).build(),
        solids: vec![],
        loops: vec![],
        spikes: vec![],
        effects: vec![
            effect(440.0, 480.0, 70.0, EffectKind::Cactus),
            effect(750.0, 980.0, 8.0, EffectKind::Mud),
            effect(2400.0, 2600.0, 8.0, EffectKind::Mud),
            effect(
                9600.0,
                9680.0,
                8.0,
                EffectKind::Trampoline {
                    velocity: Vec2::new(560.0, 700.0),
                },
            ),
            effect(
                10700.0,
                10780.0,
                8.0,
                EffectKind::Trampoline {
                    velocity: Vec2::new(420.0, 620.0),
                },
            ),
            effect(11140.0, 11220.0, 100.0, EffectKind::Cactus),
            effect(11700.0, 11750.0, 75.0, EffectKind::Jet),
            effect(13400.0, 13450.0, 75.0, EffectKind::Jet),
        ],
        traps: vec![
            TrapDef {
                kind: TrapKind::PopUpSpikes {
                    spikes: Spikes {
                        left: 2100.0,
                        base: 0.0,
                        count: 4,
                    },
                },
                trigger: trigger(1980.0, 2200.0),
            },
            TrapDef {
                kind: TrapKind::Raven {
                    from: Vec2::new(3960.0, 280.0),
                    strike: Vec2::new(3900.0, 45.0),
                },
                trigger: trigger(3600.0, 3740.0),
            },
            TrapDef {
                kind: TrapKind::FallingPillar {
                    pillar: Rect::new(4550.0, 260.0, 4610.0, 460.0),
                    floor: 0.0,
                },
                trigger: trigger(4200.0, 4330.0),
            },
            TrapDef {
                kind: TrapKind::RisingPillar {
                    pillar: Rect::new(5200.0, -210.0, 5260.0, -10.0),
                    rise: 210.0,
                },
                trigger: trigger(4850.0, 4980.0),
            },
            TrapDef {
                kind: TrapKind::Tnt {
                    crate_box: Rect::new(5800.0, 0.0, 5856.0, 56.0),
                    blast_radius: 120.0,
                },
                trigger: trigger(5350.0, 5490.0),
            },
            TrapDef {
                kind: TrapKind::Ball {
                    start: Vec2::new(6600.0, 32.0),
                    radius: 32.0,
                    speed: 230.0,
                    bounce_height: 0.0,
                    travel: 900.0,
                },
                trigger: trigger(6100.0, 6200.0),
            },
            TrapDef {
                kind: TrapKind::Ball {
                    start: Vec2::new(7700.0, 32.0),
                    radius: 32.0,
                    speed: 230.0,
                    bounce_height: 200.0,
                    travel: 950.0,
                },
                trigger: trigger(7100.0, 7200.0),
            },
            TrapDef {
                kind: TrapKind::DropSlab {
                    slab: Rect::new(8500.0, -20.0, 8750.0, 0.0),
                },
                trigger: trigger(8440.0, 8560.0),
            },
            TrapDef {
                kind: TrapKind::Raven {
                    from: Vec2::new(14100.0, 270.0),
                    strike: Vec2::new(14000.0, 60.0),
                },
                trigger: trigger(13500.0, 13600.0),
            },
        ],
        checkpoints: vec![
            Checkpoint {
                zone: trigger(3400.0, 3460.0),
                respawn: Vec2::new(3500.0, 0.0),
            },
            Checkpoint {
                zone: trigger(9000.0, 9060.0),
                respawn: Vec2::new(9100.0, 0.0),
            },
        ],
        start: Vec2::new(100.0, 0.0),
        finish: Rect::new(15100.0, 0.0, 15150.0, 220.0),
        bounds: Rect::new(-100.0, -350.0, 15400.0, 850.0),
        fall_y: -100.0,
    }
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
    fn level_one_has_a_single_trap_and_no_checkpoint() {
        let level = level_one();
        assert_eq!(level.traps.len(), 1);
        assert!(level.checkpoints.is_empty());
    }
}
