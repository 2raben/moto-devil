//! Australien: red outback, canyons, and a great red rock. Kangaroos hop
//! towards the bike: hopping balls drawn as kangaroos.

use bevy::prelude::*;

use super::{
    ball, blank, checkpoint, finish_at, framed, pop_up_spikes, rising_block, sky_block, strip, trap,
};
use crate::effects::{EffectDef, EffectKind};
use crate::level::{Country, Level, Profile, TrapDef, TrapKind};

pub fn levels() -> Vec<Level> {
    vec![outback(), gorges(), red_rock()]
}

fn kangaroo(x: f32, ground: f32) -> TrapKind {
    ball(x, ground, 36.0, 220.0, 200.0, 1500.0)
}

/// A kangaroo, red mud before a gorge, a boulder, a raven, and spikes in
/// front of the finish.
fn outback() -> Level {
    let mut level = blank("australien-outback", "Outback", Country::Australia);
    level.ground = Profile::start(-100.0, 400.0)
        .to(-100.0, 0.0)
        .to(2400.0, 0.0)
        .pit(2400.0, 2800.0, -400.0)
        .to(6200.0, 0.0)
        .to(6200.0, 400.0)
        .build();
    level.effects = vec![strip(1700.0, 2000.0, 0.0, EffectKind::Mud)];
    level.traps = vec![
        sky_block(350.0, 0.0, (150.0, 200.0)),
        trap(kangaroo(1500.0, 0.0), (500.0, 600.0)),
        trap(
            ball(4300.0, 0.0, 34.0, 260.0, 0.0, 1100.0),
            (3300.0, 3400.0),
        ),
        trap(
            TrapKind::Raven {
                from: Vec2::new(4950.0, 330.0),
                strike: Vec2::new(4850.0, 50.0),
            },
            (4500.0, 4600.0),
        ),
        rising_block(5150.0, 0.0, (4900.0, 5000.0)),
        pop_up_spikes(5450.0, 0.0, 4, (5350.0, 5420.0)),
    ];
    level.checkpoints = vec![checkpoint(3100.0, 0.0), checkpoint(4300.0, 0.0)];
    level.finish = finish_at(5650.0, 0.0);
    framed(level, 700.0)
}

/// Gorges of growing width, two kangaroos, a landing plate that gives way,
/// a trampoline over a canyon, and a rock out of the empty sky.
fn gorges() -> Level {
    let mut level = blank("australien-schluchten", "Schluchten", Country::Australia);
    level.ground = Profile::start(-100.0, 400.0)
        .to(-100.0, 0.0)
        .pit(600.0, 1000.0, -400.0)
        .pit(2900.0, 3300.0, -400.0)
        .pit(3420.0, 3700.0, -400.0)
        .pit(4800.0, 5300.0, -400.0)
        .to(7300.0, 0.0)
        .to(7300.0, 400.0)
        .build();
    level.effects = vec![EffectDef {
        zone: Rect::new(4600.0, 0.0, 4680.0, 8.0),
        kind: EffectKind::Trampoline {
            velocity: Vec2::new(760.0, 700.0),
        },
    }];
    level.traps = vec![
        trap(kangaroo(2200.0, 0.0), (1100.0, 1200.0)),
        trap(kangaroo(2700.0, 0.0), (1100.0, 1200.0)),
        TrapDef {
            kind: TrapKind::DropSlab {
                slab: Rect::new(3420.0, -20.0, 3700.0, 0.0),
            },
            trigger: Rect::new(3420.0, 0.0, 3700.0, 140.0),
        },
        trap(
            TrapKind::FallingPillar {
                pillar: Rect::new(6000.0, 700.0, 6060.0, 900.0),
                floor: 0.0,
            },
            (5650.0, 5750.0),
        ),
        trap(
            TrapKind::Raven {
                from: Vec2::new(6580.0, 330.0),
                strike: Vec2::new(6480.0, 50.0),
            },
            (6150.0, 6200.0),
        ),
        pop_up_spikes(6600.0, 0.0, 4, (6520.0, 6580.0)),
    ];
    level.checkpoints = vec![checkpoint(4200.0, 0.0)];
    level.finish = finish_at(6800.0, 0.0);
    framed(level, 1000.0)
}

/// Up the red rock, a kangaroo and spikes on top, a jet flight over a canyon
/// past a raven, and a finish that runs away twice.
fn red_rock() -> Level {
    let mut level = blank(
        "australien-roter-felsen",
        "Am roten Felsen",
        Country::Australia,
    );
    level.ground = Profile::start(-100.0, 400.0)
        .to(-100.0, 0.0)
        .to(800.0, 0.0)
        .to(1300.0, 350.0)
        .to(3300.0, 350.0)
        .to(3300.0, -400.0)
        .to(4400.0, -400.0)
        .to(4400.0, 200.0)
        .to(5900.0, 200.0)
        .to(5900.0, 700.0)
        .build();
    level.effects = vec![EffectDef {
        zone: Rect::new(3050.0, 350.0, 3100.0, 425.0),
        kind: EffectKind::Jet,
    }];
    level.traps = vec![
        rising_block(500.0, 0.0, (250.0, 300.0)),
        trap(kangaroo(2400.0, 350.0), (1300.0, 1400.0)),
        pop_up_spikes(2300.0, 350.0, 4, (2200.0, 2260.0)),
        trap(
            TrapKind::Raven {
                from: Vec2::new(3900.0, 750.0),
                strike: Vec2::new(3800.0, 560.0),
            },
            (2800.0, 2850.0),
        ),
        trap(
            TrapKind::JumpingFinish {
                to: finish_at(5650.0, 200.0),
            },
            (5000.0, 5100.0),
        ),
        trap(
            TrapKind::JumpingFinish {
                to: finish_at(4700.0, 200.0),
            },
            (5450.0, 5550.0),
        ),
    ];
    level.checkpoints = vec![checkpoint(2650.0, 350.0)];
    level.finish = finish_at(5250.0, 200.0);
    level.fall_y = -150.0;
    framed(level, 1200.0)
}
