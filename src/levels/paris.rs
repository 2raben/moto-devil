//! Paris: riding over the rooftops of the skyline. Gaps between houses fall to
//! the street; pigeons dive like ravens, barrels roll over the roofs, and
//! hatches give way.

use bevy::prelude::*;

use super::{ball, blank, checkpoint, finish_at, framed, pop_up_spikes, sky_block, trap};
use crate::effects::{EffectDef, EffectKind};
use crate::level::{Country, Level, Profile, TrapDef, TrapKind};

/// The street below the roofs.
const STREET: f32 = -500.0;

pub fn levels() -> Vec<Level> {
    vec![rooftops(), montmartre(), eiffel_tower()]
}

fn pigeon(from: Vec2, strike: Vec2, trigger: (f32, f32)) -> TrapDef {
    trap(TrapKind::Raven { from, strike }, trigger)
}

/// A hatch in a roof that gives way under the bike, with its trigger close to
/// the roof so jumping over it is safe.
fn hatch(left: f32, right: f32, roof: f32) -> TrapDef {
    TrapDef {
        kind: TrapKind::DropSlab {
            slab: Rect::new(left, roof - 20.0, right, roof),
        },
        trigger: Rect::new(left, roof, right, roof + 140.0),
    }
}

/// Gaps between houses, a chimney, a pigeon, a hatch, a barrel, antenna
/// spikes, and a finish that hops over the last gap.
fn rooftops() -> Level {
    let mut level = blank("paris-daecher", "Über den Dächern", Country::Paris);
    level.ground = Profile::start(-100.0, 400.0)
        .to(-100.0, 0.0)
        .pit(800.0, 1050.0, STREET)
        .to(2150.0, 0.0)
        .to(2300.0, 100.0)
        .to(3000.0, 100.0)
        .to(3000.0, 0.0)
        .pit(3400.0, 3750.0, STREET)
        .pit(4150.0, 4300.0, STREET)
        .pit(6850.0, 7100.0, STREET)
        .to(7700.0, 0.0)
        .to(7700.0, 400.0)
        .build();
    level.solids = vec![Rect::new(1550.0, 0.0, 1590.0, 40.0)];
    level.traps = vec![
        sky_block(450.0, 0.0, (150.0, 250.0)),
        pigeon(
            Vec2::new(2850.0, 380.0),
            Vec2::new(2750.0, 150.0),
            (2400.0, 2500.0),
        ),
        hatch(4150.0, 4300.0, 0.0),
        trap(
            ball(5700.0, 0.0, 30.0, 260.0, 0.0, 1000.0),
            (4950.0, 5050.0),
        ),
        pop_up_spikes(6150.0, 0.0, 4, (6050.0, 6130.0)),
        trap(
            TrapKind::JumpingFinish {
                to: finish_at(7250.0, 0.0),
            },
            (6400.0, 6500.0),
        ),
    ];
    level.checkpoints = vec![checkpoint(3150.0, 0.0), checkpoint(4800.0, 0.0)];
    level.finish = finish_at(6500.0, 0.0);
    framed(level, 800.0)
}

/// Climbing roofs: a gap, a hatch, a barrel on the high roof, a pigeon and a
/// chimney out of the sky together, a wide gap before TNT, and a finish that
/// jumps back.
fn montmartre() -> Level {
    let mut level = blank("paris-montmartre", "Montmartre", Country::Paris);
    level.ground = Profile::start(-100.0, 400.0)
        .to(-100.0, 0.0)
        .to(600.0, 0.0)
        .to(750.0, 120.0)
        .pit(1300.0, 1600.0, STREET)
        .pit(1950.0, 2100.0, STREET)
        .to(2700.0, 120.0)
        .to(2850.0, 240.0)
        .to(4000.0, 240.0)
        .to(4000.0, 60.0)
        .pit(5150.0, 5550.0, STREET)
        .to(6850.0, 60.0)
        .to(6850.0, 400.0)
        .build();
    level.traps = vec![
        hatch(1950.0, 2100.0, 120.0),
        pigeon(
            Vec2::new(3250.0, 560.0),
            Vec2::new(3150.0, 290.0),
            (2900.0, 2950.0),
        ),
        trap(
            ball(3900.0, 240.0, 30.0, 260.0, 0.0, 900.0),
            (3150.0, 3200.0),
        ),
        pigeon(
            Vec2::new(5050.0, 420.0),
            Vec2::new(4950.0, 110.0),
            (4650.0, 4720.0),
        ),
        trap(
            TrapKind::FallingPillar {
                pillar: Rect::new(5050.0, 700.0, 5110.0, 900.0),
                floor: 60.0,
            },
            (4800.0, 4900.0),
        ),
        trap(
            TrapKind::Tnt {
                crate_box: Rect::new(6100.0, 60.0, 6156.0, 116.0),
                blast_radius: 110.0,
            },
            (5600.0, 5700.0),
        ),
        trap(
            TrapKind::JumpingFinish {
                to: finish_at(6250.0, 60.0),
            },
            (6400.0, 6500.0),
        ),
    ];
    level.checkpoints = vec![checkpoint(4150.0, 60.0)];
    level.finish = finish_at(6550.0, 60.0);
    framed(level, 1100.0)
}

/// Trampolines over boulevards, one of which throws the bike back, a jet
/// flight up to the high roofs past a pigeon, and a finish that runs away over
/// a gap and back.
fn eiffel_tower() -> Level {
    let mut level = blank("paris-eiffelturm", "Eiffelturm", Country::Paris);
    level.ground = Profile::start(-100.0, 400.0)
        .to(-100.0, 0.0)
        .pit(850.0, 1150.0, STREET)
        .pit(2700.0, 3000.0, STREET)
        .to(3700.0, 0.0)
        .to(3700.0, STREET)
        .to(4700.0, STREET)
        .to(4700.0, 150.0)
        .pit(5650.0, 5800.0, STREET)
        .to(6400.0, 150.0)
        .to(6400.0, 600.0)
        .build();
    let pad = |left: f32, velocity: Vec2| EffectDef {
        zone: Rect::new(left, 0.0, left + 80.0, 8.0),
        kind: EffectKind::Trampoline { velocity },
    };
    level.effects = vec![
        pad(650.0, Vec2::new(560.0, 700.0)),
        pad(1900.0, Vec2::new(-450.0, 650.0)),
        EffectDef {
            zone: Rect::new(3400.0, 0.0, 3450.0, 75.0),
            kind: EffectKind::Jet,
        },
    ];
    level.traps = vec![
        sky_block(400.0, 0.0, (150.0, 250.0)),
        pigeon(
            Vec2::new(4300.0, 560.0),
            Vec2::new(4200.0, 280.0),
            (3250.0, 3300.0),
        ),
        trap(
            TrapKind::JumpingFinish {
                to: finish_at(5900.0, 150.0),
            },
            (5300.0, 5400.0),
        ),
        trap(
            TrapKind::JumpingFinish {
                to: finish_at(5450.0, 150.0),
            },
            (5850.0, 5880.0),
        ),
    ];
    level.checkpoints = vec![checkpoint(3200.0, 0.0)];
    level.finish = finish_at(5500.0, 150.0);
    framed(level, 1000.0)
}
