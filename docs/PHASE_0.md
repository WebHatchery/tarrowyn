# Phase 0 — The First Evening

## Purpose

Phase 0 gives The Years of Tarrowyn a small, attractive 2D client foundation
that can be extended into the separate client/server architecture described in
the GDD. It proves the local game loop and the shape of a gentle first session
without claiming that the MMO systems already exist.

## Included

- Rust + Macroquad client using the shared `macroquad-toolkit`.
- Data-driven game configuration, actions, and three crop definitions.
- A deterministic top-down map with a settlement path, Hearth tavern, shared
  fields, forest edge, and water collision.
- Player movement through visible on-screen controls, map taps, and optional
  keyboard input.
- An accelerated 180-second day, simple crop stages, a tavern rumour, and
  lightweight local progression (gold, skill, reputation, and satchel contents).
- Toolkit save slots with versioned save data and migration structure.
- Deterministic screenshot capture for visual verification.

## Deliberately deferred

Phase 0 does not implement networking, a server process, accounts, character
identity, authoritative world state, database persistence, chat transport,
trading, combat, NPC simulation, or deployment. The local `GameSession` owns
simulation only as a temporary client prototype; the first multiplayer phase
must move shared state and validation behind a server boundary.

## Phase 1 handoff

The next milestone should introduce a workspace-level server crate and a small
protocol for login, player presence, movement acknowledgement, chat, and a
server-owned clock. The existing `GameSession` fields provide the first client
projection to map onto that protocol, while the UI actions already return
intent-like events rather than mutating state directly.

The GDD’s smallest multiplayer slice remains the product target: multiple
clients in one tiny settlement, movement, collision, social presence, chat,
accelerated time, a tavern, shared plots, three crops, inventory, direct trade,
one wilderness threat, one repeatable contract, skill progress, and restart-safe
server persistence.
