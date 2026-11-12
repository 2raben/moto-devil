---
tags: [adr]
status: accepted
date: 2026-11-12
supersedes: [0001-move-the-player-with-hand-written-kinematics.md]
sources:
  - Cargo.toml
  - src/physics.rs
  - src/bike.rs
  - src/course.rs
  - src/traps.rs
  - src/run.rs
  - src/visuals.rs
  - src/tests.rs
  - ../../pitches/moto-devil/pitch--moto-devil.md
---

# 0004 — Simulate the bike with Rapier inside the fixed step

## Context

The [Moto Devil pitch](../../pitches/moto-devil/pitch--moto-devil.md) replaces the
starter's box with a stunt bike: a body and two wheels that rotate, follow slopes,
jump ramps, flip in the air, and ride through a loop.
[ADR 0001](0001-move-the-player-with-hand-written-kinematics.md) kept movement
hand-written and limited to axis-aligned boxes, and named exactly this case as the
reason to adopt an engine.

Alternatives considered:

- **Extend the hand-written kinematics.** Wheel contact on arbitrary polylines,
  rotation, joints, and loops would be a small physics engine of our own, and the
  pitch names it as the main rabbit hole.
- **`avian2d` 0.8.** ECS-native and a good fit, but the project's physics skill and
  examples target Rapier, and neither engine was needed for anything Avian does
  better here.
- **`bevy_rapier2d` 0.37**, the Bevy 0.20 line of the mature Rapier integration,
  with default features off (`dim2` only), so no render or asset helpers are pulled
  in.

The pitch asked for a short spike first. The spike is the implementation: the
headless tests drive scripted rides through both levels, including the loop.

## Decision

[physics.rs](../../../src/physics.rs) adds `RapierPhysicsPlugin` to `FixedUpdate`
with `TimestepMode::Fixed` at Bevy's fixed rate (64 Hz) and four substeps, a
length unit of 100 world units per metre, and a gravity of 1200 units/s². Game
systems run in two sets around Rapier's chained `PhysicsSet`:
`GameSystems::Drive` before `SyncBackend` (respawns, trap timers, collider
switches, and the bike controls) and `GameSystems::React` after `Writeback`
(contacts, flips, triggers, checkpoints, crashes, finish, clock).

Ownership of `Transform`:

- Rapier owns the `Transform` of every physics body. Only respawns write it, as a
  teleport, and mark the body `Teleported`.
- After each step `read_body_poses` copies body transforms into `Pose` (position and
  angle), keeping the previous value in `PreviousPose`. Game rules, tests, and
  presentation read `Pose`. Moving parts without a body (traps, the finish flag)
  have a `Pose` too, written by their systems.
- Presentation never touches simulation entities. It spawns separate drawings,
  linked with the `VisualOf` relationship so they despawn with what they draw, and
  places them by interpolating `PreviousPose` → `Pose` with the fixed-step overstep.

This replaces the previous rule that simulation positions live in `Position` and
only presentation writes `Transform`.

The bike ([bike.rs](../../../src/bike.rs)) is a dynamic box body with two dynamic
ball wheels on revolute joints. Bike parts share a collision group that only hits
level geometry. Steering is deliberately arcade-like, as the pitch asks for
controls that are not too hard:

- Gas and brake change the velocity of all three bodies along the ground tangent
  (from the wheels' contact normals), so speed builds predictably; gas needs the
  rear wheel on the ground.
- Tilting sets the body's spin and gives the wheels the matching velocity, turning
  the bike as one piece around the grounded wheel or, in the air, its middle.
- Assists: self-righting in the air without a tilt key, and a 45° lean limit
  against the ground normal while a wheel touches it.
- A jump adds upward speed to all three bodies; turning around flips which wheel is
  the rear one.

Crashes are decided by game rules, not contacts: a Rapier shape query for the
rider's head against level geometry, circle-against-shape checks for hazards, and a
fall line. Loops are two polyline halves whose colliders are switched with
`ColliderDisabled` as the bike passes the top, so the bike can enter and leave at
the bottom.

## Consequences

- Slopes, ramps, loops, and flips come from the engine; the levels are polylines
  instead of tile maps.
- Results are deterministic for the same inputs, build, and machine, but not exact
  to the unit. Tests check ranges and outcomes (speed near the maximum, landed
  without a crash, finished the level) rather than exact positions.
- Tests need `TransformPlugin` and `StatesPlugin` besides `MinimalPlugins`.
- One more dependency tied to the Bevy version: upgrading Bevy means waiting for
  the matching `bevy_rapier2d`.
- Feel is tuned in `BikeTuning` and the level data; the scripted rides in
  `src/tests.rs` show whether the levels can still be finished after a change.

## The tell

- `rg -n "bevy_rapier2d" Cargo.toml` shows default features disabled and only `dim2`.
- `rg -n "Transform" src/bike.rs src/run.rs src/traps.rs src/course.rs` shows only
  spawning and teleporting physics bodies; presentation code
  (`src/visuals.rs`, `src/camera.rs`) never queries `Velocity`, `Collider`, or
  `RigidBody`.
- `cargo test --locked level_` passes the scripted rides through both levels. A
  failure after a tuning or level change means a level may no longer be finishable
  as designed.

These checks cannot judge feel; ride the levels after changing tuning.
