---
tags: [adr]
status: accepted
date: 2026-11-12
sources:
  - src/progress.rs
  - src/save.rs
  - src/flow.rs
  - ../../pitches/moto-devil/pitch--moto-devil.md
---

# 0005 — Save progress in a versioned text file in the user's data directory

## Context

The [Moto Devil pitch](../../pitches/moto-devil/pitch--moto-devil.md) lets several
people play on one computer: each picks a name, and the game remembers which levels
they finished and how far they got, shown as gravestones. That needs one small,
durable file. The file can be edited by hand or cut short by a crash, so it is
untrusted input. Persisted formats are lasting choices, so the pitch asked for an
ADR before building it.

Alternatives considered:

- **Serialize the Bevy world** (`DynamicWorldBuilder`). Ties the file to component
  layouts, has no stable identities or migrations, and would read reflected data from
  an untrusted file.
- **`serde` with RON or JSON**, plus a crate for the data directory. Robust and
  familiar, but two or three dependencies for four kinds of facts.
- **Bevy's `SettingsPlugin`**. Meant for user settings, not a list of players, and
  off by default.
- **A hand-written, line-based text format** with its own small data type, and a
  few lines to find the data directory.

## Decision

[progress.rs](../../../src/progress.rs) holds the data (`Profiles`, `Player`,
`LevelProgress`) and pure `encode`/`decode` functions. The format:

```text
moto-devil-save 1
player<TAB>Mia
level<TAB>level-1<TAB>done
level<TAB>level-2<TAB>crash<TAB>1234.5
```

- The first line names the format and its version; other versions are rejected.
- Level lines belong to the player line before them. Level ids are the stable `id`
  of each level, never its position in the list.
- Reading is defensive: files over 64 KiB are refused; names must pass the same
  rules as typed names (1 to 12 Latin letters, digits, or spaces), which also keeps
  tabs and line breaks out; duplicate names, more than 32 players, unknown levels,
  and crash positions that are not finite or lie outside the level are dropped line
  by line, so one damaged line does not lose everyone's progress.

[save.rs](../../../src/save.rs) stores the file as `moto-devil/progress.txt` in the
platform's data directory (`$XDG_DATA_HOME` or `~/.local/share` on Linux,
`~/Library/Application Support` on macOS, `%APPDATA%` on Windows), never next to the
program or at a path built from a name. It loads at startup; a file that cannot be
read or parsed is renamed to `progress.txt.broken` and the game starts with an empty
list. It saves in `Last` whenever `Profiles` changes and the encoded text differs
from what was last written, through a temporary file in the same directory that
then replaces the old one. File access never runs in the fixed step.

## Consequences

- No new dependencies; the format is readable and can be fixed by hand, which the
  pitch relies on for renaming or removing players.
- Changing the format means a new version number and, if old files should keep
  working, a migration in `decode`. Version 1 has none; add one only when the
  format really changes.
- Writes are small and synchronous. That is fine at this size; a much larger save
  would belong on `IoTaskPool`.
- There is no browser storage; a web build would not keep progress.
- The headless tests do not add `SavePlugin`; they use the pure functions, and
  `save.rs` tests write to a temporary directory.

## The tell

- `cargo test --locked progress::` and `cargo test --locked save::` cover the format,
  damaged lines, oversized and foreign files, and the backup of a broken file.
- `rg -n "std::fs" src` matches only `src/save.rs`.
- Review any change to `SAVE_VERSION` or `encode` together with `decode`; the tests
  cannot tell whether an old player's file still loads after a format change unless
  a test with that old file is added.
