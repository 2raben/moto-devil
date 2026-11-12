# Architecture

This document describes the current implementation and conventions for extending
it. [AGENTS.md](../../AGENTS.md) is the agent working brief;
[CONTRIBUTING.md](../../CONTRIBUTING.md) covers shaping, review, and validation.
[ADRs](decisions/README.md) record significant architectural choices and rationale.

## Current structure and runtime

The project is one Rust binary crate implementing the
[Moto Devil pitch](../pitches/moto-devil/pitch--moto-devil.md): a stunt-bike game
with a player selection, three levels with traps, saved progress, and gravestones.
The third level implements the [Fallen & Power-ups pitch](../pitches/fallen-powerups/pitch--fallen-powerups.md).
[Cargo.toml](../../Cargo.toml) uses Rust edition 2024, Bevy 0.20 with default
features disabled and the `2d`, `audio`, and `ui` feature sets, and `bevy_rapier2d`
0.37 with only `dim2` ([ADR 0004](decisions/0004-simulate-the-bike-with-rapier.md)).
Dependencies have a higher development optimization level than the application.
[Cargo.lock](../../Cargo.lock) pins dependency resolution;
[.cargo/config.toml](../../.cargo/config.toml) configures Clang and LLD for x86_64
Linux, and [.mise/](../../.mise/config.toml) pins the toolchain and defines tasks
([ADR 0003](decisions/0003-run-project-tasks-with-mise.md)). The UI font, DejaVu
Sans Bold, is compiled in from [assets/fonts/](../../assets/fonts/LICENSE-DejaVu.txt)
because Bevy's built-in font has no umlauts.

[src/main.rs](../../src/main.rs) composes the app from plugins in two layers
([ADR 0002](decisions/0002-keep-the-simulation-headless-behind-a-controls-resource.md),
amended by [ADR 0006](decisions/0006-pass-menu-input-through-a-menuinput-resource.md)).

**Simulation**, which runs without a window and is used by the tests:

- [simulation.rs](../../src/simulation.rs): `SimulationPlugin` adds physics, the
  `Screen` state, the resources, the flow systems, and the two chained fixed-step
  sets.
- [physics.rs](../../src/physics.rs): Rapier in `FixedUpdate`, collision groups, the
  `GameSystems::Drive` and `GameSystems::React` sets around Rapier's `PhysicsSet`,
  and `Pose`/`PreviousPose`, which hold every simulated position and angle.
- [flow.rs](../../src/flow.rs): the `Screen` state (`SelectPlayer`, `Playing`), the
  `Levels` list and `CurrentLevel`, `MenuInput`/`MenuAction`, the `Selection`, and
  the systems that pick a player, load levels, and move between them.
- [level.rs](../../src/level.rs) and [levels.rs](../../src/levels.rs): the `Level`
  data type (ground profile, solids, loops, spikes, traps, checkpoints, start,
  finish, bounds, fall line, contact effects) and the three built-in levels.
- [course.rs](../../src/course.rs): spawns a level's colliders, traps, finish, and
  bike, all marked `LevelEntity`.
- [bike.rs](../../src/bike.rs): `Controls`, `BikeTuning`, the bike's body and wheels,
  `drive_bike`, and `sense_ground` (contacts and flips).
- [traps.rs](../../src/traps.rs): `Trap`, `Hazard`, `Finish`, and loop state, with
  the systems that re-arm, move, trigger, and switch them.
- [effects.rs](../../src/effects.rs): contact definitions and `CourseEffect` /
  `RideEffects` state for cactus, mud, trampoline, and jet pickups.
- [run.rs](../../src/run.rs): the `Run` resource (time, deaths, flips, finish, crash
  delay, respawn point, attempt) and the rules for restart, respawn, checkpoints,
  crashes, the finish, and the clock.
- [progress.rs](../../src/progress.rs): `Profiles` and `ActivePlayer`, name rules,
  gravestone rules, and the save format.

**Presentation and I/O**, which need a window or a file system:

- [input.rs](../../src/input.rs): keyboard and gamepad into `Controls` and
  `MenuInput`. The only module that reads devices.
- [save.rs](../../src/save.rs): loads and writes the progress file
  ([ADR 0005](decisions/0005-save-progress-in-a-versioned-text-file.md)). The only
  module that touches files.
- [visuals.rs](../../src/visuals.rs): the palette; ground, loop, spike, trap, flag,
  bike, wheel, and gravestone drawings; interpolation from `Pose`.
- [camera.rs](../../src/camera.rs): a `Camera2d` with a minimum 960 × 540 view that
  follows the bike and stays inside the level.
- [hud.rs](../../src/hud.rs): the progress panel, controls reminder, cues
  ("Autsch!", "Flip!", "Checkpoint!"), and finish panel; installs the font.
- [menu.rs](../../src/menu.rs): the "Wer fährt?" screen.

### Frame and fixed-step flow

1. `PreUpdate`, after Bevy's `InputSystems`: while riding, `sample_controls` writes
   held values (including jet intent `jump_held`) and latches presses into `Controls`; `collect_menu_actions` appends
   menu keys and typed characters to `MenuInput`.
2. `StateTransition`: entering `Playing` loads `CurrentLevel` (resources `Level`,
   `Run`, `Controls`, and the course entities); leaving it despawns the course.
3. `FixedUpdate` at 64 Hz while `Playing`:
   - `GameSystems::Drive`, chained: `remember_poses`, `restart_or_respawn`,
     `rearm_course`, `reset_effects`, `drive_bike`, `update_traps`.
   - Rapier's `SyncBackend`, `StepSimulation` (four substeps), and `Writeback`.
   - `GameSystems::React`, chained: `read_body_poses`, `sense_ground`,
     `trigger_traps`, `switch_loops`, `reach_checkpoint`, `detect_crash`,
     `reach_finish`, `sense_effects`, `tick_run`.
4. `Update`: `select_player` or `playing_menu` drain `MenuInput`; visuals draw newly
   spawned scenery, traps, bike, and wheels, and redraw gravestones when `Profiles`
   changes; the HUD and menu refresh.
5. `PostUpdate`, before transform propagation: drawings follow interpolated poses,
   the bike is mirrored to its facing, the helmet turns red during a crash, TNT
   shows its fuse and explosion, spent pickups and departed hazards hide, jet
   flames show active thrust, and then the camera follows the bike.
6. `Last`: `save_profiles` writes the progress file if `Profiles` changed.

### Bike model

The bike is a dynamic 52 × 14 box with two dynamic wheels of radius 16 on revolute
joints. Its parts collide only with level geometry. Rapier owns their transforms;
`read_body_poses` copies them to `Pose` after each step, and only respawns write
them back. `drive_bike` steers by changing velocities:

- Gas pushes all three bodies along the ground tangent (900 units/s² up to
  720 units/s) while the rear wheel touches the ground; the brake slows them
  (1400 units/s²) while any wheel does.
- Tilt sets the body's spin and gives the wheels matching speed: 3.5 rad/s around
  the grounded wheel on the ground, 8 rad/s around the middle in the air. Without a
  tilt key in the air the bike rights itself; on the ground it never leans more than
  45° from the ground normal.
- A jump adds 520 units/s upwards when a wheel touches the ground, with the
  starter's coyote time (0.1 s) and jump buffer (0.12 s).
- Turning around brakes, then mirrors which wheel is the rear one and where the head
  sits, within 0.45 s.

`sense_ground` reads wheel contacts from Rapier for the ground normal and collects
rotation in the air; landing after almost a full turn counts a flip. A crash is the
head touching level geometry (a Rapier shape query), any part touching an active
hazard, or the body falling below the level's fall line. After 0.45 s the bike
respawns at the last checkpoint, the attempt counter goes up, and the traps re-arm.

### Levels, traps, and loops

A level's ground is a polyline profile from left to right; vertical steps make walls
and pits. Traps are a trigger rectangle plus a kind: a slab that drops, spikes that
pop up, a pillar that falls and rises, TNT that explodes after a fuse, and a finish
that jumps elsewhere. Traps fire when the bike's body enters the trigger, behave the
same every time, and re-arm on each new attempt. A loop is two polyline halves; the
right half is solid until the bike passes the top, then the left half, so the bike
enters and leaves at the bottom.

### Contact effects and flight

[ADR 0007](decisions/0007-compose-contact-effects-and-scripted-hazards.md) records
how effects compose. `sense_effects` checks every region against the bike after
crash and finish checks. `drive_bike` applies the result on the next fixed step.

- A cactus catches a wheel, stops all bike velocities and disables gravity until
  a jump releases it. The release jumps even without ground contact and gives a
  260 units/s push in the facing direction. The same cactus cannot catch it again
  until both wheels have left the region.
- Mud reduces acceleration and top speed to 35% while a wheel touches it. Above
  that cap the bike brakes progressively. Normal tuning returns on leaving it.
- A trampoline sets the launch velocity on contact from above, once per landing.
  It can send the bike straight into a cactus; a cactus hold wins over a pending
  launch. A trampoline does not spend jet fuel.
- A jet pickup fills a two-second reservoir without stacking. Holding Space or
  controller South supplies upward acceleration, capped at 420 units/s rise;
  releasing saves the remainder. It replaces the normal jump while fuel remains.
  A cactus escape still takes priority. The HUD shows fuel and a hold hint, plus
  cactus/mud cues. No power-up prevents a hazard crash.

`reset_effects` clears course contact/pickup state on a new attempt and bike
state on a new attempt, crash or finish. The third level puts both pickups after
its last checkpoint. Its three sections teach ground effects, timed hazards,
and flight, including trampoline into cactus and jet flight past a raven.

Ravens use a fixed warning, dive and exit. Balls roll or repeatedly bounce along
flat authored paths. Their poses and circular hazards share the same trajectory;
travel/time limits remove their danger and drawing until the next attempt. The
rising block mirrors the falling pillar's timing. These scripted hazards do not
push the bike or react to other geometry.

### Progress and gravestones

`Profiles` lists the players; `ActivePlayer` is the one riding. A crash farther right
than any earlier one moves that player's gravestone in the level; finishing marks the
level done and moves the stone to the finish. The player selection starts each
player at their first unfinished level.

## Code conventions and boundaries

Use Bevy entities and components for game objects, resources for shared state, and
systems for behavior. Keep application composition easy to locate in `main.rs`.
Keep simulation plugins free of rendering, device input, and file access so they
keep running headless; presentation plugins react to simulation components and
resources.

Follow rustfmt and nearby Rust naming and import conventions. Prefer small,
purposeful systems with explicit data access. Make scheduling dependencies explicit
when behavior depends on order. Read device input only in `input.rs` and pass it to
the simulation through `Controls` and `MenuInput`. Simulated positions live in
`Pose`; Rapier owns physics bodies' transforms, and presentation draws separate
entities linked with `VisualOf`. Keep pure logic, such as level geometry, name
rules, the save format, and control laws, in plain functions that unit tests can call
directly.

New dependencies, physics integration, simulation timing, and persisted formats can
create lasting constraints; explain such choices in an ADR. Grow modules into
folders or new plugins only when a feature needs them.

## Validation

The [contribution guide](../../CONTRIBUTING.md#setup-and-validation) lists the
formatting, Clippy, build, test, and manual-run checks. There is no CI workflow.

- **Unit tests** beside each module cover level geometry and validation, the
  built-in levels, name rules, gravestone rules, the save format and damaged files,
  file writing and backups, flip counting, the tilt assists, trap timing, hazards,
  loop halves, poses, camera bounds, and the HUD's number format.
- **Simulation tests** in [src/tests.rs](../../src/tests.rs) step the real plugins and
  Rapier headless, one fixed step per update. They cover resting, driving and
  braking, jumping, flips, self-righting, the ground lean limit, turning around, head
  crashes and respawn, falling out, pop-up spikes, the finish and progress,
  checkpoints, restart, the player selection and level flow, and scripted rides that
  finish all three built-in levels, including the loop.
- [tests/effects.rs](../../src/tests/effects.rs) covers contact effects, the
  trampoline/cactus combination, jet hold/release/exhaustion and reset, overlapping
  pickups with a lethal hazard, repeatable movers and the complete third level.
- [tests/rendered.rs](../../src/tests/rendered.rs) provides opt-in Linux GPU
  captures of the trampoline/cactus encounter and jet flight, using the actual
  presentation plugins. Run separately as documented in the contribution guide.
- The input regression test verifies held jet intent and latched jump presses
  have distinct release semantics.

These tests guard behavior, not feel. Ride the levels after any tuning change, and
check visuals and input by hand.

## Documentation boundaries

- [Game idea](../idea.md): direction and longer-term possibilities.
- [Pitches](../pitches/README.md): intended work, rough solutions, risks, and
  exclusions.
- Architecture documentation: current structure, behavior, and coding conventions.
- [ADRs](decisions/README.md): significant choices, alternatives, consequences, and
  checks that expose drift.

Update conventions with the code. Keep ideas clearly separate from implemented
behavior. Preserve decision history by amending or superseding an ADR when its
choice changes.
