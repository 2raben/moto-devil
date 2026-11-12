---
tags: [adr]
status: accepted
date: 2026-11-12
amends: [0002-keep-the-simulation-headless-behind-a-controls-resource.md]
sources:
  - src/input.rs
  - src/flow.rs
  - src/bike.rs
  - src/tests.rs
---

# 0006 — Pass menu keys and typed text through a `MenuInput` resource

## Context

[ADR 0002](0002-keep-the-simulation-headless-behind-a-controls-resource.md) keeps
the simulation headless: only `input.rs` reads devices, and fixed-step systems get
input through `Controls`. Moto Devil adds a player selection with a typed name,
moving to the next level after the finish, and going back to the selection. These
are not fixed-step riding inputs, and typing needs the text a key produces, not
only its physical position.

Alternatives considered:

- **Read `KeyboardInput` in the flow or UI code.** Simple, but the selection and
  level flow could then only be tested with fake devices.
- **Use Bevy's editable text widgets.** More than one name field needs, and it ties
  the flow to UI entities.
- **A second plain resource** of device-independent menu actions, written by the
  input layer like `Controls`.

## Decision

`input.rs` stays the only module that reads devices. It writes two resources:

- `Controls` for riding, now with `throttle`, `brake`, `tilt` and the latched presses
  `jump_pressed`, `turn_pressed`, and `restart` (replacing `move_x` and the jump
  fields of ADR 0002). Riding keys are physical positions and are only sampled
  while riding.
- `MenuInput`, a list of `MenuAction`s (`Up`, `Down`, `Confirm`, `Back`,
  `Type(char)`, `Erase`) in arrival order. On the selection screen, every key that
  produces text becomes `Type`, so letters such as `W` or `E` and the space bar type
  instead of riding.

The flow systems in [flow.rs](../../../src/flow.rs) drain `MenuInput` in `Update`:
`select_player` on the selection screen, `playing_menu` while riding. Name rules
live in `progress.rs`. The presentation reads `Selection` and `Profiles` to draw
the screen.

## Consequences

- Tests choose a player, ride, go on to the next level, and go back by pushing
  `MenuAction`s, without a window or keyboard.
- Gamepads take part through the same actions (D-pad to choose, South to confirm,
  Start to go back while riding); a gamepad cannot type a name.
- Menu actions are drained every frame by whichever screen is active; an action that
  arrives during a screen change is handled by the new screen.

## The tell

- `rg -n "ButtonInput|Gamepad|KeyCode|KeyboardInput" src` matches only `src/input.rs`.
- `cargo test --locked players_choose_a_name` drives the selection and level flow
  through `MenuInput` alone.
