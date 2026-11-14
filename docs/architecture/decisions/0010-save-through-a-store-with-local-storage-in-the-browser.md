---
tags: [adr]
status: accepted
date: 2026-11-14
amends: [0005-save-progress-in-a-versioned-text-file.md]
sources:
  - src/save.rs
  - src/save/file.rs
  - src/save/browser.rs
  - src/main.rs
  - ../../pitches/browser-game/pitch--browser-game.md
---

# 0010 — Save through a store, with local storage in the browser

## Context

The [Browserspiel pitch](../../pitches/browser-game/pitch--browser-game.md) builds
Moto Devil for the browser and publishes it on GitHub Pages. A browser has no data
directory, so `save.rs` as described in
[ADR 0005](0005-save-progress-in-a-versioned-text-file.md) would keep nothing there.
The pitch also asks that a later central store (progress on several devices) need
not touch every place that knows about files.

Alternatives considered:

- **IndexedDB.** Made for game saves and larger data, but asynchronous: loading at
  startup and the player selection would have to wait for a completion, and every
  write would need a pending state. The two texts are at most 64 KiB and 4 KiB.
- **Bevy's `SettingsPlugin`.** Has native and local-storage backends, but it is
  meant for user settings, not a list of players, and ADR 0005 already rejected it.
- **A `cfg` switch inside `save.rs`** between files and local storage. Smallest
  change, but each new place to save would add another branch to the plugin.
- **A store abstraction over named texts**, with one store per platform.

## Decision

[save.rs](../../../src/save.rs) defines a `Store` trait over two entries,
`Entry::Progress` and `Entry::Keys`. A store can read an entry's text (or report
that there is none), write it, move an unusable entry aside, and describe where it
lives for log messages. It refuses texts larger than the entry's limit; formats and
all other checks stay in `progress.rs` and `bindings.rs`. `SavePlugin` loads at
`Startup` and writes in `Last` through the `SaveStore` resource and knows no paths.

- [save/file.rs](../../../src/save/file.rs): `FileStore`, native only, keeps the
  files, atomic writes, and `.broken` backups of ADR 0005 unchanged
  (`moto-devil/progress.txt`, `moto-devil/keys.txt` in the data directory).
- [save/browser.rs](../../../src/save/browser.rs): `BrowserStore`, wasm only, uses
  the page's `localStorage` items `moto-devil/progress` and `moto-devil/keys`; an
  unusable item moves to `moto-devil/progress.broken` or `moto-devil/keys.broken`.
  Local storage is synchronous, so the plugin's flow stays as it was.
- `MemoryStore`, in tests only, lets the plugin itself run headless.

[main.rs](../../../src/main.rs) chooses the store by target and inserts it. Without
one (no data directory, or local storage blocked) the game runs and does not save,
with a warning in the log.

## Consequences

- Native progress and keys files load as before; neither format changes.
- The browser keeps progress per browser profile and origin. On GitHub Pages every
  project page of the account shares that origin and its local storage; the
  `moto-devil/` prefix avoids accidental collisions, and decoding stays defensive.
- Two tabs overwrite each other; the last write wins. A full quota or blocked
  storage only logs a warning.
- A central store would fit behind the same trait, for example by caching in local
  storage and syncing in the background. The trait is synchronous on purpose; a
  store that must wait for the network would need a different contract.
- `SavePlugin` now has tests of its own with the memory store.

## The tell

- `cargo test --locked save::` covers the memory store, the plugin's load and save,
  the run without a store, and the file store.
- `rg -n "std::fs" src` matches only `src/save/file.rs`, and `rg -n "web_sys" src`
  only `src/save/browser.rs`.
- The browser store has no automated test. Check it in the browser: ride, crash,
  reload, and the gravestone is still there; the browser's storage inspector shows
  the `moto-devil/` items.
