# Architecture

This document describes the current implementation and conventions for extending
it. [AGENTS.md](../../AGENTS.md) is the agent working brief;
[CONTRIBUTING.md](../../CONTRIBUTING.md) covers shaping, review, and validation.
[ADRs](decisions/README.md) record significant architectural choices and rationale.

## Current structure and runtime

The project is one Rust binary crate implementing the
[Moto Devil pitch](../pitches/moto-devil/pitch--moto-devil.md): a stunt-bike game
with a player selection, levels with traps grouped into countries, saved progress,
and gravestones. The third level implements the
[Fallen & Power-ups pitch](../pitches/fallen-powerups/pitch--fallen-powerups.md); the
[Länder pitch](../pitches/laender/pitch--laender.md) adds five countries with three
levels each, and the [Browserspiel pitch](../pitches/browser-game/pitch--browser-game.md)
builds it for the browser.
[Cargo.toml](../../Cargo.toml) uses Rust edition 2024, Bevy 0.20 with default
features disabled and the `2d`, `audio`, and `ui` feature sets, and `bevy_rapier2d`
0.37 with only `dim2` ([ADR 0004](decisions/0004-simulate-the-bike-with-rapier.md)).
For `wasm32` it adds Bevy's `web` feature and `web-sys` for local storage; WebGL2
comes with the `2d` feature set.
Dependencies have a higher development optimization level than the application;
the `web` profile optimizes the browser build for size.
[Cargo.lock](../../Cargo.lock) pins dependency resolution;
[.cargo/config.toml](../../.cargo/config.toml) configures Clang and LLD for x86_64
Linux, and [.mise/](../../.mise/config.toml) pins the toolchain and defines tasks
([ADR 0003](decisions/0003-run-project-tasks-with-mise.md)). The UI font, DejaVu
Sans Bold, is compiled in from [assets/fonts/](../../assets/fonts/LICENSE-DejaVu.txt)
because Bevy's built-in font has no umlauts.

[src/main.rs](../../src/main.rs) chooses the window (a 960 × 540 desktop window, or
the page's `#moto-devil` canvas in the browser) and the save store for the target,
and composes the app from plugins in two layers
([ADR 0002](decisions/0002-keep-the-simulation-headless-behind-a-controls-resource.md),
amended by [ADR 0006](decisions/0006-pass-menu-input-through-a-menuinput-resource.md)
and [ADR 0009](decisions/0009-let-players-rebind-riding-keys-in-the-input-layer.md)).

**Simulation**, which runs without a window and is used by the tests:

- [simulation.rs](../../src/simulation.rs): `SimulationPlugin` adds physics, the
  `Screen` state, the resources, the flow systems, and the two chained fixed-step
  sets.
- [physics.rs](../../src/physics.rs): Rapier in `FixedUpdate`, collision groups, the
  `GameSystems::Drive` and `GameSystems::React` sets around Rapier's `PhysicsSet`,
  and `Pose`/`PreviousPose`, which hold every simulated position and angle.
- [flow.rs](../../src/flow.rs): the `Screen` state (`SelectPlayer`, `Playing`,
  `Settings`), the `Levels` list and `CurrentLevel`, `MenuInput`/`MenuAction`, the
  `Selection`, and the systems that pick a player, load levels, move between them,
  and leave the key settings.
- [level.rs](../../src/level.rs) and [levels/](../../src/levels/mod.rs): the `Level`
  data type (country, ground profile, solids, loops, spikes, traps, checkpoints,
  start, finish, bounds, fall line, contact effects), the `Country` enum, and the
  built-in levels with one module per country.
- [course.rs](../../src/course.rs): spawns a level's colliders, traps, finish, and
  bike, all marked `LevelEntity`.
- [bike.rs](../../src/bike.rs): `Controls`, `BikeTuning`, the bike's body and wheels,
  `drive_bike`, and `sense_ground` (contacts and flips).
- [traps.rs](../../src/traps.rs): `Trap`, `Hazard`, `Finish`, and loop state, with
  the systems that re-arm, move, trigger, and switch them.
- [effects.rs](../../src/effects.rs): contact definitions and `CourseEffect` /
  `RideEffects` state for cactus, mud, quicksand, trampoline, and jet pickups.
- [run.rs](../../src/run.rs): the `Run` resource (time, deaths, flips, finish, crash
  delay, respawn point, attempt) and the rules for restart, respawn, checkpoints,
  crashes, the finish, and the clock.
- [progress.rs](../../src/progress.rs): `Profiles` and `ActivePlayer`, name rules,
  gravestone rules, and the save format.

**Presentation and I/O**, which need a window or a file system:

- [input.rs](../../src/input.rs): keyboard and gamepad into `Controls` and
  `MenuInput`, with riding keys from `KeyBindings`.
- [bindings.rs](../../src/bindings.rs): the riding `Action`s, `KeyBindings`, the
  key settings editor, and the keys file format
  ([ADR 0009](decisions/0009-let-players-rebind-riding-keys-in-the-input-layer.md)).
  With `input.rs`, the only module that reads devices.
- [save.rs](../../src/save.rs): loads and writes progress
  ([ADR 0005](decisions/0005-save-progress-in-a-versioned-text-file.md)) and keys
  through a `Store` in the `SaveStore` resource
  ([ADR 0010](decisions/0010-save-through-a-store-with-local-storage-in-the-browser.md)):
  [save/file.rs](../../src/save/file.rs) natively, the only module that touches
  files, and [save/browser.rs](../../src/save/browser.rs) with the page's local
  storage in the browser.
- [visuals.rs](../../src/visuals.rs): the palette; ground, loop, spike, trap, flag,
  bike, wheel, and gravestone drawings in the country's colours; interpolation from
  `Pose`.
- [backdrop.rs](../../src/backdrop.rs): each country's `Theme` and its two
  parallax backdrop layers, built from simple generated shapes.
- [camera.rs](../../src/camera.rs): a `Camera2d` with a minimum 960 × 540 view that
  follows the bike and stays inside the level.
- [hud.rs](../../src/hud.rs): the progress panel, controls reminder, cues
  ("Autsch!", "Flip!", "Checkpoint!"), and finish panel; installs the font.
- [menu.rs](../../src/menu.rs): the "Wer fährt?" screen.
- [settings.rs](../../src/settings.rs): the "Tasten" screen.

### Frame and fixed-step flow

1. `PreUpdate`, after Bevy's `InputSystems`: while riding, `sample_controls` writes
   held values of the bound keys (including jet intent `jump_held`) and latches
   presses into `Controls`; `collect_menu_actions` appends menu keys and typed
   characters to `MenuInput`; on the key settings `edit_bindings` edits
   `KeyBindings` instead.
2. `StateTransition`: entering `Playing` loads `CurrentLevel` (resources `Level`,
   `Run`, `Controls`, and the course entities); leaving it despawns the course.
3. `FixedUpdate` at 64 Hz while `Playing`:
   - `GameSystems::Drive`, chained: `remember_poses`, `restart_or_respawn`,
     `rearm_course`, `reset_effects`, `drive_bike`, `update_traps`.
   - Rapier's `SyncBackend`, `StepSimulation` (four substeps), and `Writeback`.
   - `GameSystems::React`, chained: `read_body_poses`, `sense_ground`,
     `trigger_traps`, `switch_loops`, `reach_checkpoint`, `detect_crash`,
     `reach_finish`, `sense_effects`, `tick_run`.
4. `Update`: `select_player`, `playing_menu`, or `settings_menu` drain `MenuInput`; visuals draw newly
   spawned scenery, traps, bike, and wheels, and redraw gravestones when `Profiles`
   changes; the HUD and menu refresh.
5. `PostUpdate`, before transform propagation: drawings follow interpolated poses,
   the bike is mirrored to its facing, the helmet turns red during a crash, TNT
   shows its fuse and explosion, spent pickups and departed hazards hide, jet
   flames show active thrust, and then the camera follows the bike.
6. `Last`: `save_profiles` writes the progress entry if `Profiles` changed, and
   `save_keys` the keys entry if `KeyBindings` changed, both through the store.

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

### Countries, quicksand, and avalanches

[ADR 0008](decisions/0008-group-levels-into-countries-with-presentation-themes.md)
records the design. Every level names its `Country`; the play order is Übungsgelände,
Ägypten, Australien, USA, Paris, and Schweiz, with each country's levels in a row.
`Levels::place` gives a level's country and number for the HUD and the player
selection, and the finish panel names the next country after a country's last
level. Each level announces its country and name for its first two seconds.

Each country level strings tricks together: blocks that rise from the ground or
fall out of the empty sky, hatches and bridges that give way, spikes, finishes that
run away (sometimes twice), and the country's own trap. Presentation draws the same
trap kinds in each country's style: kangaroos (hopping balls) in Australia,
tumbleweeds in the USA, barrels and pigeons on the Paris roofs, snowballs in
Switzerland.

Quicksand raises a sink meter while a wheel touches it, slower at speed, cuts gas
and top speed, and adds drag; out of the sand the meter falls again, and a full
meter is a crash. Sand drawn over the bike shows how far it has sunk.

An avalanche hangs as a cracked snow slab at its release point, trembles for
`AVALANCHE_WARNING`, drops to the course, and rolls at a fixed speed to its end,
where it dissolves. Its height follows `Level::surface_height` with a rolling-ball
envelope, so it rolls over a gallery roof (a solid above the track) and the bike
can wait under the roof. One from behind must be outrun, or sheltered from when it
is too fast.

### Key bindings

Each riding action (gas, brake, tilt left and right, jump/jet, turn, restart) has
two key slots; the defaults are W/A/S/D and the arrows, Space, E, and R. Tab on the
player selection opens "Tasten": arrows choose an action and slot, Enter waits for
the next key, Backspace/Delete clear the slot, the last row restores the defaults,
and Escape cancels waiting or goes back. Bindings are physical keys; letter and
punctuation keys show the character the layout typed when they were bound. A key
does one thing only: binding it again takes it from its old action, and the view
says so and lists actions left without a key. Escape and Enter cannot be bound.
Gamepad controls and menu keys are fixed. The bindings are shared by all players
on the computer (or browser) and saved in `moto-devil/keys.txt` or the local storage
item `moto-devil/keys`; the HUD names the bound keys.

### Progress and gravestones

`Profiles` lists the players; `ActivePlayer` is the one riding. A crash farther right
than any earlier one moves that player's gravestone in the level; finishing marks the
level done and moves the stone to the finish. The player selection starts each
player at their first unfinished level.

### Browser build and delivery

The same crate builds for `wasm32-unknown-unknown`. `mise run web:build` compiles
with the `web` profile, generates the JavaScript glue with the `wasm-bindgen` CLI
pinned to the crate version in `Cargo.lock`, shrinks the module with `wasm-opt`,
and copies [web/index.html](../../web/index.html) into `dist/web`. The page holds a
16:9 field with the `#moto-devil` canvas, which Bevy fills; it shows „Lädt …“
until the module has started, a notice when the browser has no WebGL2, and then
focuses the canvas. Bevy keeps the browser's default key handling from the canvas,
so the arrows and Space do not scroll and Tab stays in the game. The font is
compiled in, so the page loads no other files and works under any path. Escape on
the player selection does not quit in the browser (`flow::CAN_QUIT`), and its hint
leaves out "Esc beenden". The browser runs single-threaded; GitHub Pages cannot
send the headers that threads would need.

[.github/workflows/ci.yml](../../.github/workflows/ci.yml) runs `mise run check`,
Clippy for `wasm32`, and `mise run web:build` on pull requests and pushes to
`main`. [.github/workflows/pages.yml](../../.github/workflows/pages.yml) builds the
page on every push to `main` (or on demand) and publishes `dist/web` to GitHub
Pages; only its deploy job may write to Pages.

## Code conventions and boundaries

Use Bevy entities and components for game objects, resources for shared state, and
systems for behavior. Keep application composition easy to locate in `main.rs`.
Keep simulation plugins free of rendering, device input, and file access so they
keep running headless; presentation plugins react to simulation components and
resources.

Follow rustfmt and nearby Rust naming and import conventions. Prefer small,
purposeful systems with explicit data access. Make scheduling dependencies explicit
when behavior depends on order. Read device input only in `input.rs` and `bindings.rs` and pass it to
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
formatting, Clippy, build, test, and manual-run checks; CI runs the automated ones
and the browser build.

- **Unit tests** beside each module cover level geometry and validation, the
  built-in levels, name rules, gravestone rules, the save format and damaged files,
  the save plugin with a memory store, file writing and backups, flip counting, the tilt assists, trap timing, hazards,
  loop halves, poses, camera bounds, and the HUD's number format.
- **Simulation tests** in [src/tests.rs](../../src/tests.rs) step the real plugins and
  Rapier headless, one fixed step per update. They cover resting, driving and
  braking, jumping, flips, self-righting, the ground lean limit, turning around, head
  crashes and respawn, falling out, pop-up spikes, the finish and progress,
  checkpoints, restart, the player selection and level flow, and scripted rides that
  finish the first two built-in levels, including the loop.
- [tests/effects.rs](../../src/tests/effects.rs) covers contact effects, the
  trampoline/cactus combination, jet hold/release/exhaustion and reset, overlapping
  pickups with a lethal hazard, repeatable movers and the complete third level.
- [tests/countries.rs](../../src/tests/countries.rs) rides every country level with
  a scripted pilot that knows its tricks: it waits out timed traps, shelters from
  avalanches, jumps where its plan says, and turns when the finish runs away. Each
  ride must finish without a crash and meet every trap except jumped hatches.
- [tests/rendered.rs](../../src/tests/rendered.rs) provides opt-in Linux GPU
  captures of the trampoline/cactus encounter, jet flight, any country level
  (`MOTO_SHOT=<level id>,<start x>,<frames>,<throttle>`), and the key settings,
  using the actual presentation plugins. Run separately as documented in the contribution guide.
- The input regression tests verify that held jet intent and latched jump
  presses have distinct release semantics, and that rebound keys drive
  `Controls` while their old keys do not. `bindings.rs` tests cover the defaults,
  moving keys between actions, reserved keys, layout labels, the editor (also fed
  with `KeyboardInput` messages), and the keys file format.

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
