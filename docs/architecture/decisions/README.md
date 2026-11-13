# Architectural decision records

ADRs explain lasting architectural choices and their tradeoffs. Read them alongside
the [current architecture](../README.md) and [contribution guide](../../../CONTRIBUTING.md).
Routine refactoring and individual implementation details do not need a record.

## Recording a decision

Copy [the template](_adr-template.md) to `NNNN-<slug>.md`, using the next four-digit
number starting at `0001`, and add a link and status to the index below. Include
Context, Decision, Consequences, and The tell.

Use `proposed` for unimplemented choices, `accepted` for implemented choices,
and `superseded` for replaced choices. Use the recording date; for retrospective
records add `recorded: true` and distinguish observed code from reconstructed
tradeoffs. Do not invent historical discussions or imply that a recording date is
the implementation date.

Preserve accepted rationale. For replacements, use `supersedes` / `superseded-by`
on the new and old records and mark the old record superseded. For partial changes,
use `amends` / `amended-by`, keep both records accepted, and explain the amendment
below the old title. Update both sides and the index.

Keep Markdown links and lineage paths relative to the ADR file. In `sources`,
code paths are repository-relative and document paths are ADR-relative. Remove
unused metadata and placeholders before accepting a record.

**The tell** identifies a repository-root search, focused test, or manual review
that can expose drift. State the expected result and limitations; a search alone
does not prove correctness.

## History

Records 0001 to 0003 were written for the starter platformer this project began
from. Moto Devil kept 0002 and 0003, amended 0002 with 0006, and replaced 0001 with
0004 when it adopted a physics engine. Superseded records stay for their rationale.

## Decision index

| Record | Status |
| --- | --- |
| [0001 — Move the player with hand-written kinematics on a fixed timestep](0001-move-the-player-with-hand-written-kinematics.md) | superseded by 0004 |
| [0002 — Keep the simulation headless behind a `Controls` resource](0002-keep-the-simulation-headless-behind-a-controls-resource.md) | accepted, amended by 0006 |
| [0003 — Pin the toolchain and run project tasks with mise](0003-run-project-tasks-with-mise.md) | accepted |
| [0004 — Simulate the bike with Rapier inside the fixed step](0004-simulate-the-bike-with-rapier.md) | accepted |
| [0005 — Save progress in a versioned text file in the user's data directory](0005-save-progress-in-a-versioned-text-file.md) | accepted |
| [0006 — Pass menu keys and typed text through a `MenuInput` resource](0006-pass-menu-input-through-a-menuinput-resource.md) | accepted, amended by 0009 |
| [0007 — Compose contact effects and scripted hazards in the fixed step](0007-compose-contact-effects-and-scripted-hazards.md) | accepted, amended by 0008 |
| [0008 — Group levels into countries with presentation themes](0008-group-levels-into-countries-with-presentation-themes.md) | accepted |
| [0009 — Let players rebind riding keys in the input layer and a keys file](0009-let-players-rebind-riding-keys-in-the-input-layer.md) | accepted |
