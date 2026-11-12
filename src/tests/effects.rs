use super::*;
use crate::effects::{CourseEffect, EffectDef, EffectKind, JET_SECONDS, RideEffects};
use crate::traps::Hazard;

#[test]
fn flight_school_can_be_finished_with_both_checkpoints_and_combinations() {
    let mut sim = Sim::riding(crate::levels::flight_school());
    let mut catches = 0;
    let mut had_cactus = false;
    let mut checkpoints = Vec::new();
    for _ in 0..(100.0 / STEP) as usize {
        if sim.run().finished {
            break;
        }
        let p = sim.body().translation;
        let grounded = sim.state().grounded();
        let effects = sim.bike::<RideEffects>();
        let vy = sim.bike::<bevy_rapier2d::prelude::Velocity>().linear.y;
        if effects.stuck.is_some() && !had_cactus {
            catches += 1;
        }
        had_cactus = effects.stuck.is_some();
        let mut wait = false;
        let mut ball_near = false;
        let world = sim.app.world_mut();
        for (trap, pose) in world.query::<(&Trap, &Pose)>().iter(world) {
            if let Some(t) = trap.fired {
                let duration = match trap.def.kind {
                    TrapKind::Raven { .. } => 2.1,
                    TrapKind::FallingPillar { .. } | TrapKind::RisingPillar { .. } => 3.0,
                    TrapKind::Tnt { .. } => 1.0,
                    _ => 0.0,
                };
                wait |= duration > 0.0
                    && t < duration
                    && p.x >= trap.def.trigger.min.x
                    && p.x < trap.def.trigger.max.x + 240.0;
            }
            if let TrapKind::Ball {
                start,
                bounce_height: 0.0,
                ..
            } = trap.def.kind
            {
                let distance = start.x + pose.translation.x - p.x;
                ball_near |=
                    trap.fired.is_some() && !trap.departed() && (0.0..300.0).contains(&distance);
            }
        }
        let jump = effects.stuck.is_some()
            || (grounded
                && (ball_near
                    || [
                        (1340.0, 1390.0),
                        (2000.0, 2050.0),
                        (2830.0, 2880.0),
                        (8420.0, 8480.0),
                    ]
                    .iter()
                    .any(|&(a, b)| (a..b).contains(&p.x))));
        *sim.controls() = Controls {
            throttle: if wait { 0.0 } else { 1.0 },
            brake: if wait { 1.0 } else { 0.0 },
            jump_pressed: jump,
            jump_held: !wait
                && effects.jet_left > 0.0
                && p.y < 110.0
                && vy < 150.0
                && ((11920.0..13050.0).contains(&p.x) || (13840.0..14950.0).contains(&p.x)),
            ..default()
        };
        sim.steps(1);
        let respawn = sim.run().respawn.x as i32;
        if respawn > 100 && !checkpoints.contains(&respawn) {
            checkpoints.push(respawn);
        }
        assert_eq!(
            sim.run().deaths,
            0,
            "crashed at {:?}, before step {:?}, jet {}",
            sim.body(),
            p,
            effects.jet_left
        );
    }
    assert!(sim.run().finished, "did not finish, at {:?}", sim.body());
    assert_eq!(
        catches, 2,
        "both the initial cactus and the trampoline combination"
    );
    assert_eq!(checkpoints, vec![3500, 9100]);
}

#[test]
fn cactus_stops_gas_and_tilt_but_a_jump_releases_it() {
    let mut level = flat();
    level.effects.push(EffectDef {
        zone: Rect::new(350.0, 0.0, 400.0, 80.0),
        kind: EffectKind::Cactus,
    });
    let mut sim = Sim::riding(level);
    sim.gas(1.0);
    assert!(sim.until(3.0, |s| s.bike::<RideEffects>().stuck.is_some()));
    sim.steps(3);
    let caught = sim.body();
    sim.controls().tilt = 1.0;
    sim.seconds(0.5);
    assert!(sim.body().translation.distance(caught.translation) < 2.0);
    assert!(sim.speed() < 1.0);
    sim.controls().tilt = 0.0;
    sim.controls().jump_pressed = true;
    sim.steps(1);
    assert!(sim.bike::<RideEffects>().stuck.is_none());
    assert!(sim.until(3.0, |s| s.body().translation.x > 550.0));
    assert_eq!(sim.run().deaths, 0);
    assert!(sim.bike::<RideEffects>().immune_cacti.is_empty());
}

#[test]
fn mud_slows_the_bike_and_normal_acceleration_returns_outside() {
    let mut level = flat();
    level.effects.push(EffectDef {
        zone: Rect::new(200.0, 0.0, 1100.0, 8.0),
        kind: EffectKind::Mud,
    });
    let mut sim = Sim::riding(level);
    sim.gas(1.0);
    sim.seconds(2.0);
    assert!(sim.bike::<RideEffects>().muddy);
    assert!(sim.speed() < 280.0);
    assert!(sim.until(6.0, |s| s.body().translation.x > 1200.0));
    assert!(!sim.bike::<RideEffects>().muddy);
    sim.seconds(1.0);
    assert!(sim.speed() > 500.0);
}

#[test]
fn trampoline_lands_in_cactus_and_the_escape_reaches_safe_ground() {
    let mut level = crate::levels::flight_school();
    level.start = Vec2::new(10600.0, 0.0);
    let mut sim = Sim::riding(level);
    sim.gas(1.0);
    let mut top: f32 = 0.0;
    assert!(
        sim.until(5.0, |s| {
            top = top.max(s.body().translation.y);
            s.bike::<RideEffects>().stuck.is_some()
        }),
        "did not land in cactus, at {:?}",
        sim.body()
    );
    assert!(top > 140.0 && top < 250.0, "one launch, top={top}");
    assert!((11050.0..11270.0).contains(&sim.body().translation.x));
    sim.controls().jump_pressed = true;
    assert!(sim.until(4.0, |s| s.body().translation.x > 11400.0));
    assert_eq!(sim.run().deaths, 0);
    assert!(sim.bike::<RideEffects>().stuck.is_none());
}

#[test]
fn jet_hold_release_exhaustion_and_restart_have_distinct_effects() {
    let mut level = flat();
    level.effects.push(EffectDef {
        zone: Rect::new(350.0, 0.0, 400.0, 75.0),
        kind: EffectKind::Jet,
    });
    let mut sim = Sim::riding(level);
    sim.gas(1.0);
    assert!(sim.until(3.0, |s| s.bike::<RideEffects>().jet_left > 0.0));
    sim.controls().jump_held = true;
    sim.seconds(0.5);
    let used = sim.bike::<RideEffects>();
    assert!(used.jet_active);
    assert!((used.jet_left - 1.5).abs() < 0.04);
    assert!(sim.body().translation.y > 70.0);
    sim.controls().jump_held = false;
    sim.seconds(0.25);
    assert_eq!(sim.bike::<RideEffects>().jet_left, used.jet_left);
    assert!(!sim.bike::<RideEffects>().jet_active);
    sim.controls().jump_held = true;
    sim.seconds(1.75);
    assert_eq!(sim.bike::<RideEffects>().jet_left, 0.0);
    assert!(!sim.bike::<RideEffects>().jet_active);
    sim.controls().jump_held = false;
    sim.controls().restart = true;
    sim.gas(0.0);
    sim.steps(2);
    assert_eq!(sim.bike::<RideEffects>().jet_left, 0.0);
    let world = sim.app.world_mut();
    assert!(world.query::<&CourseEffect>().iter(world).all(|e| !e.spent));
    sim.settle();
    sim.controls().jump_pressed = true;
    sim.seconds(0.2);
    assert!(sim.body().translation.y > 75.0, "normal jump works again");
}

#[test]
fn overlapping_pickups_refill_without_stacking_and_do_not_prevent_crashes() {
    let mut level = flat();
    for _ in 0..2 {
        level.effects.push(EffectDef {
            zone: Rect::new(80.0, 0.0, 140.0, 75.0),
            kind: EffectKind::Jet,
        });
    }
    level.traps.push(TrapDef {
        kind: TrapKind::Tnt {
            crate_box: Rect::new(70.0, 0.0, 126.0, 56.0),
            blast_radius: 120.0,
        },
        trigger: Rect::new(0.0, 0.0, 500.0, 200.0),
    });
    let mut sim = Sim::riding(level);
    assert_eq!(sim.bike::<RideEffects>().jet_left, JET_SECONDS);
    assert!(sim.until(1.0, |s| s.run().crash.is_some()));
    assert_eq!(sim.bike::<RideEffects>().jet_left, 0.0);
    assert!(sim.until(1.0, |s| s.run().attempt == 1));
    assert_eq!(
        sim.bike::<RideEffects>().jet_left,
        JET_SECONDS,
        "pickup reappears at respawn"
    );
}

#[test]
fn new_movers_repeat_after_restart_and_stop_being_hazards() {
    for kind in [
        TrapKind::Raven {
            from: Vec2::new(900.0, 300.0),
            strike: Vec2::new(850.0, 40.0),
        },
        TrapKind::Ball {
            start: Vec2::new(1100.0, 32.0),
            radius: 32.0,
            speed: 200.0,
            bounce_height: 0.0,
            travel: 600.0,
        },
        TrapKind::Ball {
            start: Vec2::new(1100.0, 32.0),
            radius: 32.0,
            speed: 200.0,
            bounce_height: 180.0,
            travel: 600.0,
        },
    ] {
        let mut level = flat();
        level.traps.push(TrapDef {
            kind,
            trigger: Rect::new(0.0, 0.0, 200.0, 200.0),
        });
        let mut sim = Sim::riding(level);
        sim.steps(32);
        let snapshot = |s: &mut Sim| {
            let world = s.app.world_mut();
            let (p, h) = world
                .query_filtered::<(&Pose, &Hazard), With<Trap>>()
                .single(world)
                .unwrap();
            (*p, *h)
        };
        let first = snapshot(&mut sim);
        sim.controls().restart = true;
        sim.steps(1);
        sim.steps(32);
        assert_eq!(snapshot(&mut sim), first);
        sim.seconds(4.0);
        assert_eq!(snapshot(&mut sim).1, Hazard(None));
        assert_eq!(sim.run().deaths, 0);
    }
}

#[test]
fn block_from_below_rises_and_retracts() {
    let mut level = flat();
    level.traps.push(TrapDef {
        kind: TrapKind::RisingPillar {
            pillar: Rect::new(700.0, -210.0, 760.0, -10.0),
            rise: 210.0,
        },
        trigger: Rect::new(0.0, 0.0, 200.0, 200.0),
    });
    let mut sim = Sim::riding(level);
    sim.seconds(0.8);
    let world = sim.app.world_mut();
    let (p, h) = world
        .query_filtered::<(&Pose, &Hazard), With<Trap>>()
        .single(world)
        .unwrap();
    assert!((p.translation.y - 210.0).abs() < 0.01);
    assert!(h.0.is_some_and(|s| s.touches(Vec2::new(730.0, 100.0), 10.0)));
    sim.seconds(3.0);
    let world = sim.app.world_mut();
    assert_eq!(
        world
            .query_filtered::<&Hazard, With<Trap>>()
            .single(world)
            .unwrap()
            .0,
        None
    );
}
