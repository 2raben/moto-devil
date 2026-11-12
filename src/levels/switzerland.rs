//! Schweiz: steep climbs and descents, avalanche galleries, and the summit.
//! Avalanches are the country's own trap: outrun one from behind, or shelter
//! under a gallery roof from one ahead.

use bevy::prelude::*;

use super::{ball, blank, checkpoint, finish_at, framed, pop_up_spikes, runaway_finish, trap};
use crate::effects::{EffectDef, EffectKind};
use crate::level::{Country, Level, Profile, TrapDef, TrapKind};

/// The bottom of crevasses and gorges.
const DEPTH: f32 = -400.0;

pub fn levels() -> Vec<Level> {
    vec![alp_path(), pass(), summit()]
}

fn avalanche(
    release: Vec2,
    from_x: f32,
    to_x: f32,
    radius: f32,
    speed: f32,
    trigger: (f32, f32),
) -> TrapDef {
    trap(
        TrapKind::Avalanche {
            release,
            from_x,
            to_x,
            radius,
            speed,
        },
        trigger,
    )
}

/// A first avalanche from ahead with a gallery to wait in, one from behind on
/// a long descent with spikes at its foot.
fn alp_path() -> Level {
    let mut level = blank("schweiz-alpweg", "Alpweg", Country::Switzerland);
    level.ground = Profile::start(-100.0, 400.0)
        .to(-100.0, 0.0)
        .to(500.0, 0.0)
        .to(1200.0, 300.0)
        .to(1900.0, 300.0)
        .to(2700.0, 700.0)
        .to(3000.0, 700.0)
        .to(4600.0, 100.0)
        .to(6100.0, 100.0)
        .to(6100.0, 500.0)
        .build();
    level.solids = vec![Rect::new(1250.0, 440.0, 1880.0, 470.0)];
    level.traps = vec![
        trap(ball(480.0, 0.0, 32.0, 240.0, 0.0, 400.0), (150.0, 250.0)),
        runaway_finish(5450.0, 100.0, (5700.0, 5780.0)),
        avalanche(
            Vec2::new(2900.0, 1000.0),
            2700.0,
            900.0,
            60.0,
            450.0,
            (1300.0, 1400.0),
        ),
        avalanche(
            Vec2::new(2750.0, 1000.0),
            2900.0,
            4700.0,
            70.0,
            480.0,
            (3100.0, 3200.0),
        ),
        pop_up_spikes(4800.0, 100.0, 4, (4700.0, 4780.0)),
    ];
    level.checkpoints = vec![checkpoint(2850.0, 700.0), checkpoint(5200.0, 100.0)];
    level.finish = finish_at(5800.0, 100.0);
    framed(level, 1300.0)
}

/// A snowball on the pass, an avalanche from behind that drives the bike onto
/// a bridge that gives way, a raven at a cliff, and a gallery whose way out
/// is guarded by a block from the sky.
fn pass() -> Level {
    let mut level = blank("schweiz-pass", "Pass", Country::Switzerland);
    level.ground = Profile::start(-100.0, 400.0)
        .to(-100.0, 0.0)
        .to(400.0, 0.0)
        .to(1100.0, 450.0)
        .to(2000.0, 450.0)
        .to(3200.0, 50.0)
        .pit(3300.0, 3550.0, DEPTH)
        .to(4000.0, 50.0)
        .to(4500.0, 350.0)
        .to(4800.0, 350.0)
        .to(5300.0, 0.0)
        .to(6100.0, 0.0)
        .to(6900.0, 500.0)
        .to(7600.0, 500.0)
        .to(7600.0, 900.0)
        .build();
    level.solids = vec![Rect::new(5650.0, 140.0, 6100.0, 170.0)];
    level.traps = vec![
        runaway_finish(6950.0, 500.0, (7050.0, 7150.0)),
        trap(
            ball(1950.0, 450.0, 32.0, 260.0, 0.0, 600.0),
            (1350.0, 1400.0),
        ),
        avalanche(
            Vec2::new(1850.0, 850.0),
            1950.0,
            3500.0,
            70.0,
            480.0,
            (2050.0, 2150.0),
        ),
        TrapDef {
            kind: TrapKind::DropSlab {
                slab: Rect::new(3300.0, 30.0, 3550.0, 50.0),
            },
            trigger: Rect::new(3300.0, 50.0, 3550.0, 190.0),
        },
        trap(
            TrapKind::Raven {
                from: Vec2::new(4550.0, 650.0),
                strike: Vec2::new(4450.0, 400.0),
            },
            (4200.0, 4300.0),
        ),
        avalanche(
            Vec2::new(7100.0, 900.0),
            6900.0,
            5400.0,
            60.0,
            450.0,
            (5700.0, 5800.0),
        ),
        trap(
            TrapKind::FallingPillar {
                pillar: Rect::new(6300.0, 650.0, 6360.0, 850.0),
                floor: 125.0,
            },
            (6100.0, 6150.0),
        ),
    ];
    level.checkpoints = vec![checkpoint(3800.0, 50.0)];
    level.finish = finish_at(7200.0, 500.0);
    framed(level, 1200.0)
}

/// The steepest climb with an avalanche from ahead, a trampoline over a
/// crevasse, an avalanche from behind too fast to outrun, and a summit
/// finish that jumps back behind spikes.
fn summit() -> Level {
    let mut level = blank("schweiz-gipfel", "Gipfel", Country::Switzerland);
    level.ground = Profile::start(-100.0, 400.0)
        .to(-100.0, 0.0)
        .to(400.0, 0.0)
        .to(1000.0, 400.0)
        .to(1850.0, 400.0)
        .to(2500.0, 900.0)
        .to(2900.0, 900.0)
        .pit(2900.0, 3200.0, DEPTH)
        .to(3600.0, 900.0)
        .to(4400.0, 300.0)
        .to(4950.0, 300.0)
        .to(5500.0, 100.0)
        .to(6500.0, 100.0)
        .to(6500.0, 500.0)
        .build();
    level.solids = vec![
        Rect::new(1450.0, 540.0, 1820.0, 570.0),
        Rect::new(4450.0, 440.0, 4950.0, 470.0),
    ];
    level.effects = vec![EffectDef {
        zone: Rect::new(2800.0, 900.0, 2880.0, 908.0),
        kind: EffectKind::Trampoline {
            velocity: Vec2::new(650.0, 600.0),
        },
    }];
    level.traps = vec![
        trap(ball(380.0, 0.0, 32.0, 240.0, 0.0, 400.0), (150.0, 250.0)),
        avalanche(
            Vec2::new(2700.0, 1250.0),
            2500.0,
            900.0,
            60.0,
            450.0,
            (1450.0, 1500.0),
        ),
        avalanche(
            Vec2::new(3450.0, 1250.0),
            3600.0,
            5300.0,
            70.0,
            800.0,
            (3700.0, 3800.0),
        ),
        pop_up_spikes(5800.0, 100.0, 4, (5700.0, 5770.0)),
        trap(
            TrapKind::JumpingFinish {
                to: finish_at(5650.0, 100.0),
            },
            (6050.0, 6150.0),
        ),
    ];
    level.checkpoints = vec![checkpoint(2650.0, 900.0)];
    level.finish = finish_at(6200.0, 100.0);
    framed(level, 1400.0)
}
