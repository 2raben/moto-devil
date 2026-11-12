//! Ägypten: dunes, quicksand, tombs with low ceilings, and a pyramid. Quicksand
//! is the country's own trap; the shared traps hide in the sand.

use bevy::prelude::*;

use super::{
    ball, blank, checkpoint, finish_at, framed, pop_up_spikes, rising_block, sky_block, strip, trap,
};
use crate::effects::EffectKind::Quicksand;
use crate::level::{Country, Level, Profile, TrapKind};

pub fn levels() -> Vec<Level> {
    vec![dunes(), valley_of_the_kings(), pyramid()]
}

/// Quicksand alone, harmless sand that hides spikes, and a finish that hops
/// on behind a second strip of quicksand.
fn dunes() -> Level {
    let mut level = blank("aegypten-duenen", "Dünen", Country::Egypt);
    level.ground = Profile::start(-100.0, 400.0)
        .to(-100.0, 0.0)
        .to(600.0, 0.0)
        .hill(1100.0, 60.0)
        .to(2600.0, 0.0)
        .pit(2600.0, 2950.0, -400.0)
        .to(4300.0, 0.0)
        .hill(4800.0, 90.0)
        .to(6000.0, 0.0)
        .to(6000.0, 400.0)
        .build();
    level.effects = vec![
        strip(1300.0, 1500.0, 0.0, Quicksand),
        strip(3500.0, 4000.0, 0.0, Quicksand),
        strip(5450.0, 5650.0, 0.0, Quicksand),
    ];
    level.traps = vec![
        rising_block(450.0, 0.0, (150.0, 250.0)),
        pop_up_spikes(2000.0, 0.0, 4, (1880.0, 1960.0)),
        trap(
            TrapKind::Raven {
                from: Vec2::new(4720.0, 330.0),
                strike: Vec2::new(4640.0, 130.0),
            },
            (4350.0, 4420.0),
        ),
        sky_block(5130.0, 0.0, (4850.0, 4900.0)),
        trap(
            TrapKind::JumpingFinish {
                to: finish_at(5750.0, 0.0),
            },
            (5150.0, 5250.0),
        ),
    ];
    level.checkpoints = vec![checkpoint(3200.0, 0.0), checkpoint(4150.0, 0.0)];
    level.finish = finish_at(5300.0, 0.0);
    framed(level, 700.0)
}

/// A rolling boulder, a tomb whose low ceiling forbids jumping, a floor that
/// gives way over quicksand, TNT between dunes, and a finish that jumps back.
fn valley_of_the_kings() -> Level {
    let mut level = blank("aegypten-tal-der-koenige", "Tal der Könige", Country::Egypt);
    level.ground = Profile::start(-100.0, 400.0)
        .to(-100.0, 0.0)
        .to(500.0, 0.0)
        .hill(1000.0, 50.0)
        .to(3500.0, 0.0)
        .to(3600.0, -70.0)
        .to(3800.0, -70.0)
        .to(3900.0, 0.0)
        .to(6000.0, 0.0)
        .to(6000.0, 400.0)
        .build();
    level.solids = vec![Rect::new(2500.0, 170.0, 3300.0, 520.0)];
    level.effects = vec![
        strip(3000.0, 3200.0, 0.0, Quicksand),
        strip(3600.0, 3800.0, -70.0, Quicksand),
    ];
    level.traps = vec![
        trap(
            ball(2100.0, 0.0, 34.0, 260.0, 0.0, 1300.0),
            (1100.0, 1200.0),
        ),
        trap(
            TrapKind::FallingPillar {
                pillar: Rect::new(2900.0, 170.0, 2960.0, 370.0),
                floor: 0.0,
            },
            (2450.0, 2550.0),
        ),
        crate::level::TrapDef {
            kind: TrapKind::DropSlab {
                slab: Rect::new(3500.0, -20.0, 3900.0, 0.0),
            },
            trigger: Rect::new(3500.0, 0.0, 3900.0, 140.0),
        },
        trap(
            TrapKind::Tnt {
                crate_box: Rect::new(4900.0, 0.0, 4956.0, 56.0),
                blast_radius: 110.0,
            },
            (4500.0, 4600.0),
        ),
        rising_block(4520.0, 0.0, (4230.0, 4260.0)),
        pop_up_spikes(5150.0, 0.0, 4, (5050.0, 5150.0)),
        trap(
            TrapKind::JumpingFinish {
                to: finish_at(5300.0, 0.0),
            },
            (5550.0, 5650.0),
        ),
    ];
    level.checkpoints = vec![checkpoint(4200.0, 0.0)];
    level.finish = finish_at(5750.0, 0.0);
    framed(level, 700.0)
}

/// Quicksand before a ramp, boulders on the pyramid's terraces, spikes and a
/// hidden pit in one jump, and a summit finish that jumps back down.
fn pyramid() -> Level {
    let mut level = blank("aegypten-pyramide", "Die Pyramide", Country::Egypt);
    level.ground = Profile::start(-100.0, 400.0)
        .to(-100.0, 0.0)
        .to(1150.0, 0.0)
        .to(1300.0, 70.0)
        .to(1300.0, -400.0)
        .to(1700.0, -400.0)
        .to(1700.0, 0.0)
        .to(2300.0, 0.0)
        .to(2700.0, 200.0)
        .to(3700.0, 200.0)
        .to(4000.0, 380.0)
        .pit(4200.0, 4350.0, -400.0)
        .to(4900.0, 380.0)
        .to(5100.0, 500.0)
        .to(5500.0, 500.0)
        .to(5500.0, 900.0)
        .build();
    level.effects = vec![strip(700.0, 1150.0, 0.0, Quicksand)];
    level.traps = vec![
        pop_up_spikes(450.0, 0.0, 3, (330.0, 400.0)),
        sky_block(3000.0, 200.0, (2750.0, 2800.0)),
        trap(
            ball(3650.0, 200.0, 34.0, 260.0, 0.0, 900.0),
            (2950.0, 3000.0),
        ),
        pop_up_spikes(4100.0, 380.0, 3, (4020.0, 4080.0)),
        crate::level::TrapDef {
            kind: TrapKind::DropSlab {
                slab: Rect::new(4200.0, 360.0, 4350.0, 380.0),
            },
            trigger: Rect::new(4200.0, 380.0, 4350.0, 520.0),
        },
        trap(
            TrapKind::JumpingFinish {
                to: finish_at(4500.0, 380.0),
            },
            (5200.0, 5300.0),
        ),
    ];
    level.checkpoints = vec![checkpoint(2000.0, 0.0)];
    level.finish = finish_at(5350.0, 500.0);
    framed(level, 1100.0)
}
