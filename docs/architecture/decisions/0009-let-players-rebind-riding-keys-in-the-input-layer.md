---
tags: [adr]
status: accepted
date: 2026-11-13
amends: [0006-pass-menu-input-through-a-menuinput-resource.md]
sources:
  - src/bindings.rs
  - src/input.rs
  - src/settings.rs
  - src/save.rs
  - src/flow.rs
  - src/hud.rs
---

# 0009 — Let players rebind riding keys in the input layer and a keys file

## Context

The riding keys were fixed in `input.rs` (W/A/S/D and the arrows, Space, E, R).
Players asked to choose their own keys in a settings view. That needs a binding
table that `sample_controls` reads, an editor that captures the next key pressed,
a place to keep the choice between sessions, and HUD hints that name the bound
keys. [ADR 0006](0006-pass-menu-input-through-a-menuinput-resource.md) keeps
device types (`KeyCode`, `KeyboardInput`) out of the headless simulation, and
[ADR 0005](0005-save-progress-in-a-versioned-text-file.md) stores progress as a
versioned text file.

Alternatives considered:

- **Bindings in the simulation, keyed by a device-neutral key id.** Keeps the editor
  in `flow.rs` next to the other screens, but needs a second key enum that mirrors
  `KeyCode` one to one.
- **Bindings per player in `progress.txt`** (format version 2). Fits "progress is
  saved per player", but the settings view would need a chosen player first, and a
  shared keyboard usually keeps one layout.
- **An input-action crate.** More features (chords, gamepad rebinding) than seven
  keyboard actions need, and another dependency on Bevy's release cadence.
- **A binding table and editor in the input layer, one shared file.**

## Decision

- [bindings.rs](../../../src/bindings.rs) joins `input.rs` in the input layer. It
  holds the riding `Action`s (gas, brake, tilt left/right, jump/jet, turn, restart),
  the `KeyBindings` resource with two key slots per action, the `BindingEditor`
  cursor, and the pure `encode`/`decode` of the keys file. The defaults are the
  former fixed keys.
- Bindings are **physical keys**, as before. Keys that type one character take the
  character the layout typed at bind time as their label, so a German keyboard
  shows "Z" for the key that types Z.
- A key belongs to **one slot only**; binding it elsewhere takes it from its old
  action, and the view says so. An action may be left without keys; the view lists
  such actions. Escape and Enter cannot be bound: they always cancel/go back and
  confirm, so every menu stays reachable whatever is bound.
- A new `Screen::Settings` is opened with Tab (`MenuAction::Settings`) from the
  player selection. On that screen `edit_bindings` reads `KeyboardInput` directly:
  arrows move, Enter waits for a key (or restores the defaults on the last row),
  Backspace/Delete clear a slot, Escape cancels waiting or pushes
  `MenuAction::Back`, which `flow::settings_menu` turns into the way back.
  `collect_menu_actions` ignores the keyboard there; a gamepad's East or Start goes
  back. [settings.rs](../../../src/settings.rs) draws the view from
  `BindingEditor` and `KeyBindings`.
- [save.rs](../../../src/save.rs) keeps the bindings for everyone on this computer
  in `moto-devil/keys.txt` next to `progress.txt`, with the same rules as ADR 0005:
  a size limit (4 KiB), a header with a version, line-by-line checks (unknown
  actions and keys, keys bound twice, more than two keys per action, and labels
  the font cannot show are dropped), a backup of a broken file, and atomic writes in
  `Last` when the encoded text changes.

  ```text
  moto-devil-keys 1
  key<TAB>gas<TAB>KeyW<TAB>W
  key<TAB>jump<TAB>Space<TAB>Leertaste
  ```

- The HUD's controls reminder, the jet hint, and the finish panel name the bound
  keys. Gamepad controls and menu keys stay fixed.

## Consequences

- The simulation and its tests are unchanged: `Controls` stays the only riding
  input. The editor and the format are tested as pure functions and with written
  `KeyboardInput` messages.
- The device-reading boundary of ADR 0006 now covers two files, `input.rs` and
  `bindings.rs`.
- Key names in the file are a table in `bindings.rs`, not `KeyCode`'s debug output,
  so a Bevy rename cannot change the format silently. Adding a bindable key is one
  table line; renaming one needs a new format version or a migration.
- Bindings are not per player and cannot be changed while riding; leaving a level
  for the settings would unload it.
- Gamepad buttons cannot be rebound yet. A label typed at bind time is only as good
  as the layout reported it; changing the keyboard layout later leaves old labels.

## The tell

- `rg -n "ButtonInput|Gamepad|KeyCode|KeyboardInput" src --files-with-matches`
  lists only `src/input.rs` and `src/bindings.rs`.
- `cargo test --locked bindings::` covers defaults, moving keys between actions,
  reserved keys, layout labels, the editor, the format, and damaged lines;
  `cargo test --locked save::key_bindings` the file round trip and backup;
  `cargo test --locked input::rebound_keys` that rebound keys drive `Controls`.
- Review any change to the `KEYS` table or `KEYS_VERSION` together with `decode`.
