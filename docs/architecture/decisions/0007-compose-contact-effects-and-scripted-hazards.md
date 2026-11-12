---
tags: [adr]
status: accepted
date: 2026-11-12
amended-by: [0008-group-levels-into-countries-with-presentation-themes.md]
sources:
  - src/effects.rs
  - src/bike.rs
  - src/traps.rs
  - src/simulation.rs
  - ../README.md
---

# 0007 — Compose contact effects and scripted hazards in the fixed step

> Amended by [ADR 0008](0008-group-levels-into-countries-with-presentation-themes.md):
> quicksand joins the contact effects, and the avalanche is a scripted hazard
> that follows the course surface, including gallery roofs. Ravens and balls
> still do not follow terrain.

## Context

The [Fallen & Power-ups pitch](../../pitches/fallen-powerups/pitch--fallen-powerups.md)
adds contact effects, temporary thrust, and moving hazards. They must combine: a
trampoline can throw the bike into a cactus, and a pickup cannot cancel a crash.
The bike already uses Rapier; timed traps already use scripted poses and hazard
regions. A new attempt must reproduce an encounter without saved transient state.

## Decision

Keep scripted traps in `TrapKind`. Ravens follow a fixed dive and exit; balls
follow a leftward trajectory with an optional repeating ballistic bounce above
their configured baseline. These are hazardous regions, not pushable Rapier
bodies. Their visual and hazard position come from the same fixed-step pose.
Their configured duration or travel ends the encounter. They do not navigate
terrain or interact with one another.

Add `CourseEffect` entities for cactus, mud, trampoline and jet pickups, and a
`RideEffects` component on the bike. Inspect every overlapping effect after
physics, crash detection and finishing; consume its result before the next
physics step. A cactus holds all three bike bodies by zeroing velocities and
disabling gravity until a jump releases them. A release also pushes in the facing
direction and suppresses recapture by that cactus until the wheels leave it.
A cactus hold takes priority over a pending trampoline launch; jet thrust is
additive to a launch and consumes its own capped reservoir.

`Controls::jump_held` carries continuous intent alongside the latched jump press.
All velocity changes still act on the body and both wheels; Rapier owns their
transforms. Respawn resets the course effects before driving. Crashes and finishing
clear active bike effects before presentation; none of these states are saved.

## Consequences

Encounters repeat and remain testable without a window. Multiple overlapping
regions retain their effects, and rendering cannot decide game outcomes. Scripted
balls must be placed on a suitable flat course segment; arbitrary terrain-aware
rolling, dynamic pushing, or bouncing off other objects would require another
design. Contact effects use the existing circle/rectangle overlap rules rather
than duplicating collision math. The one fixed-step delay from sensing to applying
an effect is at most 1/64 second.

## The tell

Run `cargo test --locked tests::effects` inside the development environment.
The tests cover the trampoline/cactus chain, release, mud, jet consumption and
reset, repeatable movers, and a complete ride through the third level. Inspect
`src/simulation.rs` to confirm crash detection precedes pickup sensing, and run
the game to assess timing and drawings. Headless success does not prove readable
graphics or comfortable controls.
