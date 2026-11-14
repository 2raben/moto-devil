---
tags: [adr]
status: accepted
date: 2026-11-14
amends: [0006-pass-menu-input-through-a-menuinput-resource.md]
sources:
  - src/flow.rs
  - src/progress.rs
  - src/run.rs
  - src/input.rs
  - src/map.rs
  - src/hud.rs
  - src/tests.rs
---

# 0011 — Derive unlocked levels from finished ones and choose them on a level map

## Context

Players went from the player selection straight into their first unfinished
level, and Enter after the finish loaded the next level. They could not see how
far they had come across the countries, ride a finished level again, or tell
that finishing a level opens the next one. The level map makes that progress
visible and lets players choose among the levels they have opened.

Two questions needed lasting answers: where "unlocked" lives, and how the map
fits into the screen flow without breaking the headless simulation
([ADR 0002](0002-keep-the-simulation-headless-behind-a-controls-resource.md),
[ADR 0006](0006-pass-menu-input-through-a-menuinput-resource.md)).

Alternatives considered:

- **Store unlocked levels in the save file.** A new line kind and a format
  version bump ([ADR 0005](0005-save-progress-in-a-versioned-text-file.md)); the
  stored set could disagree with the finished levels after edits or level-list
  changes.
- **Unlock a whole country at once.** Fewer steps, but finishing a level would
  not visibly open anything most of the time.
- **Draw the map in UI nodes.** Simple layout, but the road, landscapes, and the
  riding bike fit the world-space meshes the backdrops already use.

## Decision

Unlocks are derived, not stored. `Profiles::unlocked` in
[progress.rs](../../../src/progress.rs) opens the first level, every finished
level, and the level after each finished one. The save format is unchanged; old
saves, whose finished levels form a prefix, open exactly the next level.

`Screen::LevelMap` sits between the player selection and riding in
[flow.rs](../../../src/flow.rs). `CurrentLevel` is the chosen level:

- Choosing a player opens the map at their first unfinished level.
- On the map, `MenuAction::Left`/`Right` (and `Up`/`Down`) move between unlocked
  levels and skip locked ones; Enter rides; Escape goes back to the player
  selection. `level_map` moves a locked choice back to the first level.
- While riding, Escape and Enter after the finish return to the map. After a
  finish the map moves on to the next level. `MapArrival` tells the presentation
  which level was ridden and which one its first finish unlocked, from
  `Run::unlocked_next`, which `reach_finish` sets and a restart keeps.

[input.rs](../../../src/input.rs) adds the new menu actions: the arrow keys on
the map, and the controller's D-pad, South to ride, and East or Start to go back.
[map.rs](../../../src/map.rs) only draws: one region per country in its theme
colours with a landmark, a road through a medallion per level, paved up to the
open levels and a footpath to locked ones, and a bike that rides to the chosen
level. Its `MapLayout` is plain geometry that unit tests check.

## Consequences

- Progress needs no migration, and the rule is one pure function that tests call
  directly. Changing the unlock rule later changes it for every existing save.
- Players can ride any finished level again and only reach new levels in order.
- Going on to the next level now takes two presses of Enter, one on the finish
  panel and one on the map; the map shows the newly unlocked level in between.
- Escape while riding no longer reaches the player selection directly.
- Tests and `Sim::riding` pass through the map with one more `Confirm`.

## The tell

- `cargo test --locked map` and `cargo test --locked unlock` cover the layout, the
  unlock rule, the choice of unlocked levels, and the restart that keeps an unlock.
- `rg -n "unlocked" src/save.rs src/progress.rs` finds no stored unlock in the save
  format; only `Profiles::unlocked` derives it.
- `MOTO_MAP=7,150 TMPDIR=/tmp cargo test --locked tests::rendered::capture_level_map -- --ignored --exact`
  captures the map for a manual look; it cannot judge whether the map reads well.
