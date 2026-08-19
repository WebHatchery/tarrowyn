# The Years of Tarrowyn — Phase 0

This crate is the first client foundation for The Years of Tarrowyn, a cozy,
slow-burn 2D MMORPG planned around a separate authoritative Rust server and
client. Phase 0 is intentionally a small local “first evening” rather than an
MMO slice: it establishes the visual language, input loop, data loading,
collision, world clock, local persistence, and seams where server authority will
later replace local simulation.

## What is playable

- Walk around a tiny settlement, shared fields, tavern, and nearby forest.
- Use visible touch-safe controls or a mouse; keyboard movement is optional.
- Plant, tend, and harvest three crop types.
- Listen for a rumour at the Hearth tavern.
- Watch the accelerated day/night clock change the atmosphere.
- Save, load, start fresh, or delete a local Phase 0 session.

The game has no network connection, account system, combat, or real server yet.
Those belong to the next milestone and are deliberately called out in
`docs/PHASE_0.md`.

## Run and verify

```powershell
cargo run
cargo test
.\publish.ps1
```

The project uses `macroquad-toolkit` for embedded data parsing, asset manifests,
virtual-resolution UI, notifications, event dispatch, camera state, and
persistence. Static JSON is kept under `assets/data/` and loaded through the
toolkit data-loader boundary.

## Source of truth

The design direction is in `The_Years_of_Tarrowyn_GDD.md`. The complete
milestone roadmap is in [`docs/README.md`](docs/README.md), beginning with the
Phase 0 scope in [`docs/PHASE_0.md`](docs/PHASE_0.md) and continuing through the
first three multiplayer milestones.
