//! USA: desert, canyons, and a gold mine. Tumbleweeds roll and bounce
//! towards the bike, and saguaros catch it like the training cactus.

use bevy::prelude::*;

use super::{
    ball, blank, checkpoint, finish_at, framed, pop_up_spikes, rising_block, runaway_finish,
    sky_block, strip, trap,
};
use crate::effects::{EffectDef, EffectKind};
use crate::level::{Country, Level, Loop, Profile, TrapDef, TrapKind};

pub fn levels() -> Vec<Level> {
    vec![desert(), canyon(), gold_mine()]
}

fn saguaro(x: f32) -> EffectDef {
    EffectDef {
        zone: Rect::new(x, 0.0, x + 40.0, 80.0),
        kind: EffectKind::Cactus,
    }
}

/// Saguaros, tumbleweeds, a gorge with a saguaro behind the landing, TNT,
/// spikes, and a bouncing tumbleweed.
fn desert() -> Level {
    let mut level = blank("usa-wueste", "Wüste", Country::Usa);
    level.ground = Profile::start(-100.0, 400.0)
        .to(-100.0, 0.0)
        .pit(2300.0, 2700.0, -400.0)
        .to(6300.0, 0.0)
        .to(6300.0, 400.0)
        .build();
    level.effects = vec![saguaro(700.0), saguaro(2950.0)];
    level.traps = vec![
        trap(ball(2000.0, 0.0, 30.0, 260.0, 0.0, 1300.0), (900.0, 1000.0)),
        trap(
            TrapKind::Tnt {
                crate_box: Rect::new(4100.0, 0.0, 4156.0, 56.0),
                blast_radius: 110.0,
            },
            (3700.0, 3800.0),
        ),
        pop_up_spikes(4500.0, 0.0, 4, (4420.0, 4480.0)),
        trap(
            ball(5600.0, 0.0, 30.0, 230.0, 220.0, 1100.0),
            (4900.0, 5000.0),
        ),
        runaway_finish(5500.0, 0.0, (5850.0, 5950.0)),
    ];
    level.checkpoints = vec![checkpoint(3300.0, 0.0), checkpoint(4750.0, 0.0)];
    level.finish = finish_at(6000.0, 0.0);
    framed(level, 700.0)
}

/// Ramps over canyons, a block that rises where you land, a tumbleweed in
/// the mud, a landing plate that gives way, and a rock out of the sky.
fn canyon() -> Level {
    let mut level = blank("usa-canyon", "Canyon", Country::Usa);
    level.ground = Profile::start(-100.0, 400.0)
        .to(-100.0, 0.0)
        .to(600.0, 0.0)
        .to(800.0, 100.0)
        .to(800.0, -400.0)
        .to(1500.0, -400.0)
        .to(1500.0, 0.0)
        .to(3600.0, 0.0)
        .to(3800.0, 100.0)
        .to(3800.0, -400.0)
        .to(4500.0, -400.0)
        .to(4500.0, 0.0)
        .pit(4500.0, 4650.0, -400.0)
        .to(6400.0, 0.0)
        .to(6400.0, 400.0)
        .build();
    level.effects = vec![strip(2700.0, 3000.0, 0.0, EffectKind::Mud)];
    level.traps = vec![
        sky_block(400.0, 0.0, (150.0, 200.0)),
        runaway_finish(5800.0, 0.0, (6000.0, 6100.0)),
        trap(
            TrapKind::RisingPillar {
                pillar: Rect::new(2300.0, -210.0, 2360.0, -10.0),
                rise: 210.0,
            },
            (1950.0, 2050.0),
        ),
        trap(
            ball(3500.0, 0.0, 30.0, 260.0, 0.0, 1000.0),
            (2600.0, 2700.0),
        ),
        TrapDef {
            kind: TrapKind::DropSlab {
                slab: Rect::new(4500.0, -20.0, 4650.0, 0.0),
            },
            trigger: Rect::new(4500.0, 0.0, 4650.0, 140.0),
        },
        trap(
            TrapKind::FallingPillar {
                pillar: Rect::new(5450.0, 650.0, 5510.0, 850.0),
                floor: 0.0,
            },
            (5100.0, 5200.0),
        ),
    ];
    level.checkpoints = vec![checkpoint(3300.0, 0.0)];
    level.finish = finish_at(6100.0, 0.0);
    framed(level, 1000.0)
}

/// A mine tunnel with TNT and a rising block where a jump hits the ceiling,
/// a loop with spikes behind it, two tumbleweeds, a gorge, and a finish that
/// jumps back.
fn gold_mine() -> Level {
    let mut level = blank("usa-goldmine", "Goldmine", Country::Usa);
    level.ground = Profile::start(-100.0, 400.0)
        .to(-100.0, 0.0)
        .pit(5800.0, 6300.0, -400.0)
        .to(7300.0, 0.0)
        .to(7300.0, 400.0)
        .build();
    level.solids = vec![Rect::new(600.0, 170.0, 2200.0, 520.0)];
    level.loops = vec![Loop {
        center: Vec2::new(2800.0, 160.0),
        radius: 160.0,
    }];
    level.traps = vec![
        rising_block(450.0, 0.0, (150.0, 250.0)),
        trap(
            TrapKind::Tnt {
                crate_box: Rect::new(1300.0, 0.0, 1356.0, 56.0),
                blast_radius: 110.0,
            },
            (900.0, 1000.0),
        ),
        trap(
            TrapKind::RisingPillar {
                pillar: Rect::new(1850.0, -210.0, 1910.0, -10.0),
                rise: 160.0,
            },
            (1500.0, 1600.0),
        ),
        pop_up_spikes(3250.0, 0.0, 4, (3120.0, 3340.0)),
        trap(ball(4400.0, 0.0, 30.0, 260.0, 0.0, 900.0), (3700.0, 3800.0)),
        trap(ball(5300.0, 0.0, 30.0, 260.0, 0.0, 900.0), (4550.0, 4650.0)),
        trap(
            TrapKind::JumpingFinish {
                to: finish_at(6450.0, 0.0),
            },
            (6650.0, 6750.0),
        ),
    ];
    level.checkpoints = vec![checkpoint(3600.0, 0.0)];
    level.finish = finish_at(6800.0, 0.0);
    framed(level, 800.0)
}
