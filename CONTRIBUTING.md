# Contributing

Keep changes focused and explain the problem, resulting behavior, and validation
in the pull request. Start with the [architecture documentation](docs/architecture/README.md)
and [architectural decision records](docs/architecture/decisions/README.md).
AI coding agents also follow [AGENTS.md](AGENTS.md).

## Shaping work

Shape substantial features before implementation: describe the problem, rough
solution, risks, and excluded work. Small fixes and documentation corrections do
not need a pitch. Bound each pitch by a useful playable experiment; add an appetite
or deadline only if your project wants one.

Save pitches as `docs/pitches/<topic>/pitch--<slug>.md` using the
[pitch template](docs/pitches/README.md#template), and add them to the pitch index.
Pitches are written in German, so the sections are Problem, Lösung (Solution),
Sicherheit (Security), Rabbit Holes, and No-Gos; the
[Moto Devil pitch](docs/pitches/moto-devil/pitch--moto-devil.md) is a worked
example. Explain relevant security concerns or why none apply. Keep Excalidraw
breadboards and rough sketches beside the pitch and link them from Lösung when
they help explain interactions or layout.

Pitches describe intent. Architecture documentation describes implemented behavior
and conventions; ADRs preserve significant choices and tradeoffs. Update the
relevant documents with implementation changes. Do not treat the broader
[game idea](docs/idea.md) as a commitment to implement every direction it lists.

## Branches and commits

Use short-lived branches and reviewed pull requests. The default branch is `main`;
use the intended integration branch as the PR base. Prefer `feature/<slug>` or
`bugfix/<slug>`, including an issue ID when useful. Link the relevant issue or
pitch and use a draft PR for early feedback.

Use Conventional Commits: `<type>[optional scope][!]: <description>`. Common types
are `feat`, `fix`, `docs`, `refactor`, `test`, `build`, `ci`, and `chore`. Use a concise
description and separate optional bodies and footers with blank lines. Mark
breaking changes with `!` or a `BREAKING CHANGE:` footer and explain migration
requirements. Squash commit messages follow the same convention.

```text
feat(player): add a wall jump
fix(run): keep collected coins after falling out
docs: describe the level map format
```

## Setup and validation

Follow the [README requirements](README.md#requirements) for Rust and native
dependencies. `Cargo.toml` declares the minimum Rust version and Bevy features;
`Cargo.lock` pins dependency resolution; [.mise/config.toml](.mise/config.toml)
pins the toolchain. On x86_64 Linux, [.cargo/config.toml](.cargo/config.toml)
selects Clang and LLD; on NixOS, run the checks inside `nix-shell`
([shell.nix](shell.nix)). Keep dependency or toolchain changes deliberate, change the
Rust version in `Cargo.toml` and `.mise/config.toml` together, and explain such
changes in the PR.

Run from the repository root for code changes:

```sh
mise run check
```

which runs:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked
cargo test --locked
```

There are no CI workflows yet. Tests include unit tests beside each module and
headless simulation tests in `src/tests.rs`, which step the real plugins at the
fixed timestep. Prefer focused tests of game rules and ECS systems that can run
without a window. Include regression coverage for meaningful bugs; avoid tests that
merely duplicate implementation details. Run a subset with `mise run test <name>`.

For rendering, input, or gameplay changes, also run:

```sh
mise run run
```

Check the affected behavior manually:

- The player selection shows "Wer fährt?"; typing a name (umlauts included) and
  Enter starts riding; ↑/↓ picks an existing player; Escape quits.
- W/↑ accelerates smoothly, S/↓ brakes, A/D and ←/→ tilt; holding a tilt in the air
  flips the bike, and releasing it lets the bike right itself. On the ground the bike
  does not flip over backwards.
- Space jumps only from the ground; E turns the bike around and gas then drives the
  other way.
- Level 1: the small ramp clears the ditch at speed; the spikes before the finish
  pop up; the finish shows the result, and Enter goes on to level 2.
- Level 2: the slab drops, the loop can be ridden at full gas, the spikes after the
  loop pop up, the checkpoint moves the respawn point, the pillar falls and rises,
  TNT explodes, and the finish jumps behind the bike.
- Level 3: cactus holds the bike until a jump, mud slows it, ravens dive, blocks
  move in both directions, balls roll/bounce, and the plate falls away. The first
  trampoline clears a pit; the second lands in a cactus that a jump escapes.
  Jet pickups give two seconds of thrust on held Space/South; releasing saves
  fuel. Check the bar/flames, both wide pits, and reset from both checkpoints.
- A crash shows "Autsch!", the bike respawns quickly, and the player's gravestone
  appears where they crashed farthest. Gravestones of other players show with their
  names; after quitting and restarting the game, they are still there.
- R restarts the level; Escape returns to the player selection.
- Tab on the player selection opens "Tasten". Rebind gas to another key (on a
  German layout try the key that types Z, which must show "Z"), bind a key that
  another action uses (the view says it moved), clear a slot with Backspace, and
  check that Escape and Enter cannot be bound. Escape cancels waiting, then goes
  back. Riding uses the new keys and not the old ones, the HUD reminder, jet hint,
  and finish panel name them, and the keys survive a restart of the game. The last
  row restores the defaults.
- A controller rides with the triggers and stick, jumps with South, turns with
  West, restarts with Select, and returns with Start.

Record what was checked; if no graphical environment is available, report that
limitation.

Optional Linux GPU captures exercise the actual simulation, visuals and HUD with
scripted controls. They need a working display/GPU, use a hidden window and no
save file, and must each run in a separate process (one winit event loop per
process). Inside `nix-shell`, use:

```sh
TMPDIR=/tmp cargo test --locked tests::rendered::capture_trampoline_cactus -- --ignored --exact
TMPDIR=/tmp cargo test --locked tests::rendered::capture_jet -- --ignored --exact
MOTO_SHOT=schweiz-alpweg,1350,235,0 TMPDIR=/tmp cargo test --locked tests::rendered::capture_country -- --ignored --exact
TMPDIR=/tmp cargo test --locked tests::rendered::capture_key_settings -- --ignored --exact
```

Inspect `/tmp/moto-devil-trampoline-cactus.png`, `/tmp/moto-devil-jet.png`,
`/tmp/moto-devil-<level id>-<start x>.png`, and `/tmp/moto-devil-keys.png` (the key
settings waiting for a second gas key, with the jump left without a key). `MOTO_SHOT` names the level, the start
x, the frame to capture, and a constant throttle.
These are capture smoke tests, not pixel baselines or a physical controller test.

For documentation-only changes, verify relative links and claims against source,
then run `git diff --check`; a full Rust build is unnecessary. Report checks
actually run, failures, and skipped coverage in the PR. Avoid unrelated formatting
and changes to generated `target/` contents.

## Architecture decisions

Use the [ADR workflow and index](docs/architecture/decisions/README.md) for lasting
choices such as physics integration, input handling, persisted formats, or
simulation scheduling. Copy the [template](docs/architecture/decisions/_adr-template.md),
explain the alternatives and consequences, and identify how reviewers can detect
drift. Routine implementation details do not need an ADR.
