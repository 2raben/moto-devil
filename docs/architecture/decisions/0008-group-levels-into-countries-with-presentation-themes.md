---
tags: [adr]
status: accepted
date: 2026-11-12
amends: [0007-compose-contact-effects-and-scripted-hazards.md]
sources:
  - src/level.rs
  - src/levels/mod.rs
  - src/flow.rs
  - src/effects.rs
  - src/traps.rs
  - src/backdrop.rs
  - src/visuals.rs
  - src/tests/countries.rs
  - ../../pitches/laender/pitch--laender.md
---

# 0008 — Group levels into countries with presentation themes

## Context

The [Länder pitch](../../pitches/laender/pitch--laender.md) groups levels into
countries: Übungsgelände (the three existing levels), Ägypten, Australien, USA,
Paris, and Schweiz, three new levels each. Each country needs its own sky,
ground, and backdrop, and some need their own traps: quicksand in Egypt and
avalanches in Switzerland. Progress is saved per level id
([ADR 0005](0005-save-progress-in-a-versioned-text-file.md)), and the simulation
must stay headless ([ADR 0002](0002-keep-the-simulation-headless-behind-a-controls-resource.md)).

Alternatives considered: a separate country list holding level ids, which can
drift from the level list; a country plugin or data file per country, which adds
a framework the six countries do not need; and themes stored in the level data,
which would put colours into the simulation.

## Decision

Every `Level` names its `Country`, a plain enum with a display name. `Levels`
keeps one list in play order, and a country's levels follow each other; a test
checks the order and that each country has three levels. `Levels::place` and
`Levels::next_country` derive the HUD text, the finish panel, and the player
selection's country from that list. Level ids and the save format do not change;
the three training levels keep their ids and order.

Presentation maps a country to a `Theme` (sky, ground, surface, snow line, rock,
windows, quicksand colour) in [backdrop.rs](../../../src/backdrop.rs). Two
`Parallax` layers of generated shapes follow the camera at fixed factors. They
are drawn behind the course, have no collision, and use repeatable noise, so the
same level always looks the same. Existing traps keep their behaviour and only
change their look per country: kangaroos, tumbleweeds, barrels, snowballs, and
pigeons.

Quicksand is a contact effect. While a wheel touches it, `RideEffects::sink`
rises, slower at speed, gas and top speed drop, and drag slows the bike. Out of
the sand the bike comes up again; fully sunk is a crash.

An avalanche is a scripted `TrapKind` like the raven and ball: a warning, a drop
from its release point to the course, then rolling at a fixed speed to its end
point. This amends ADR 0007, where scripted hazards do not follow terrain: the
avalanche's height follows `Level::surface_height`, which includes roofs of
solids, through a rolling-ball envelope. It can therefore roll over a gallery roof
while the bike waits below. It still does not push the bike or react to other
objects.

## Consequences

Adding a level to a country is one function in its country module plus a
scripted ride. Saves from before countries load unchanged. Themes cannot affect
outcomes because the simulation never reads them. The backdrop costs a few
meshes per level and is rebuilt on each level load.

Avalanche paths must be placed where the rolling envelope makes sense; the
surface helper only knows the ground and the tops of solids. The sink meter is
a simulated quantity; the sand drawn over the bike shows it, while the bike's
physics body stays on the ground below the sand.

## The tell

Run `cargo test --locked levels::tests` for the play order and
`cargo test --locked tests::countries` for crash-free scripted rides that also
meet every trap of each country level. Capture a level with
`MOTO_SHOT=<level id>,<x>,<frames>,<throttle> cargo test --locked capture_country -- --ignored`
to check a backdrop. A search for `Country::` outside `level.rs`, `levels/`,
`flow.rs`, `hud.rs`, `menu.rs`, `backdrop.rs`, and `visuals.rs` should find
nothing in the simulation that branches on a country. Headless rides do not
prove that a level is fun or readable.
