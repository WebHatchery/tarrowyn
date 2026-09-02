# The Years of Tarrowyn — Development Roadmap

This folder is the working roadmap for the game. The GDD describes the long
term design direction; the numbered phases record the technical path to the
current release candidate, while the foundational playability track defines
the player-facing proof still required for a cohesive first settlement.

## Phase map

| Phase | Name | Proof we need |
|---|---|---|
| [0](PHASE_0.md) | The First Evening | A pleasant, touch-capable local 2D client foundation. |
| [1](PHASE_1.md) | The Shared Road | Multiple clients connect to one authoritative server and see the same small world. |
| [2](PHASE_2.md) | The Persistent Settlement | Farming, inventory, trading, chat, and the accelerated clock survive server restarts and create mutual demand. |
| [3](PHASE_3.md) | The Living Frontier | Threats, contracts, NPC opportunity, settlement history, and a first pioneer outpost turn the shared map into a society. |
| [4](PHASE_4.md) | The Enduring Society | One settlement has durable institutions, deeper professions, accountable land, and households that can sustain community life. |
| [5](PHASE_5.md) | The Roads Between | Several settlements are connected by travel, trade, migration, infrastructure, and world events that cross regional boundaries. |
| [6](PHASE_6.md) | The Lasting Realm | The game is production-ready, operationally recoverable, secure for real accounts, and able to support long-term world history. |

The numbered phases are an implementation and architecture record. They must
not be renumbered or reopened merely because a later player-experience review
finds that their systems need a clearer presentation or a stronger human
playtest. The separate `F0`–`F10` track below owns that work.

## Foundational playability track

The foundational playability track turns the existing server-authoritative
systems into the slow, world-first experience described in the Tarrowyn
Foundational Game Design Brief. A system's presence in the client, protocol,
or server does not by itself complete an `F` phase. Each phase ends in a
player-observable acceptance test that exercises the connected experience.

| Phase | Name | Testable completion goal | Existing foundation |
|---|---|---|---|
| [F0](FOUNDATIONAL_PLAYABILITY_AUDIT.md) | Foundation baseline | A reproducible First Beacon fixture exists, and a requirements matrix identifies every foundational feature as usable, missing, conflicting, or deliberately deferred. | Phases 0–6 |
| [F1](FOUNDATIONAL_PLAYABILITY_F1_RUNBOOK.md) | Arriving at the First Beacon | Three touch-controlled clients can arrive, move through the same tent settlement, meet the builder, read the local need, disconnect, and return to the correct shared state. | Phases 0–1 |
| [F2](FOUNDATIONAL_PLAYABILITY_F2_RUNBOOK.md) | Living off the land | A player can explore, gather timber and minerals, use crude tools, store goods, and recover the resource state after restart. Personal tent placement remains separately tracked in the audit. | Phases 2–4 |
| [F3](FOUNDATIONAL_PLAYABILITY_F3_RUNBOOK.md) | The useful short session | A returning player can plant, tend, advance through offline world time, harvest, and replant during a useful 15-minute farming session using visible controls alone. | Phases 2 and 4 |
| [F4](FOUNDATIONAL_PLAYABILITY_F4_RUNBOOK.md) | Connected production | Ore, fuel, and a timber-derived component can become an improved tool at the rough forge, and a fixed comparison proves that the tool saves actions, time, or materials over the crude fallback. | Phases 2 and 4 |
| F5 | Player interdependence | Two players can complete an atomic barter and finish a fixed production goal in fewer actions or less world time by specialising and trading than by self-supplying. | Phases 2 and 5 |
| F6 | Building the first storehouse | Players can see an unfinished storehouse, understand its needs, contribute goods from several activities, and permanently transform it into an operational settlement structure. | Phases 3–5 |
| F7 | The cohesive first hour | A fresh player can try farming, logging, mining, exploration, smithing, trade, and settlement contribution without choosing a class, then leave with a visible future-session goal. | Cross-phase integration |
| F8 | Permanent property | A player can construct a staged home and register a bounded gated enclosure while automated guards preserve beacon commons, public routes, entrances, existing property, and escape paths. | Phase 4 |
| F9 | The second beacon | Players can build and support a frontier beacon, travel with carried inventory but not remote storage, offer it as an arrival point, let it become dormant, and restore it. | Phases 3 and 5 |
| F10 | Abandonment and restoration | Boundary tests prove three months of inactivity protection followed by visible gradual abandonment, reclamation, salvage, and restoration that survive restart. | Phases 4–6 |

`F7` is the foundational release boundary. The project may be technically
Phase 6 complete while foundational playability remains in progress. `F8`–
`F10` are post-foundation proofs of long-term property, expansion, and world
stewardship rather than prerequisites for the first cohesive release.

The F0 audit, baseline fixture contract, current status counts, risks, and
verification record are in
[`FOUNDATIONAL_PLAYABILITY_AUDIT.md`](FOUNDATIONAL_PLAYABILITY_AUDIT.md).
Run its focused automated gate from the project root with:

```powershell
.\scripts\verify_foundation_baseline.ps1
```

F1's connected arrival scenario, touch path, and verification record are in
[`FOUNDATIONAL_PLAYABILITY_F1_RUNBOOK.md`](FOUNDATIONAL_PLAYABILITY_F1_RUNBOOK.md).
Run its repeatable three-client gate with:

```powershell
.\scripts\verify_foundation_arrival.ps1
```

F2's authoritative resource, shared-cache, retry, and restart proof is in
[`FOUNDATIONAL_PLAYABILITY_F2_RUNBOOK.md`](FOUNDATIONAL_PLAYABILITY_F2_RUNBOOK.md).
Run its connected acceptance gate with:

```powershell
.\scripts\verify_foundation_resources.ps1
```

F3's touch-first crop outlook, modeled 15-minute absence, optional
maintenance, replay, and restart proof is in
[`FOUNDATIONAL_PLAYABILITY_F3_RUNBOOK.md`](FOUNDATIONAL_PLAYABILITY_F3_RUNBOOK.md).
Run its connected acceptance gate with:

```powershell
.\scripts\verify_foundation_farming.ps1
```

F4's gathered-input forge chain, typed recipe needs, fixed crude-versus-iron
comparison, replay, and restart proof is in
[`FOUNDATIONAL_PLAYABILITY_F4_RUNBOOK.md`](FOUNDATIONAL_PLAYABILITY_F4_RUNBOOK.md).
Run its connected acceptance gate with:

```powershell
.\scripts\verify_foundation_forge.ps1
```

### Foundational acceptance rules

Every `F` phase must leave a playable, restartable build and meet all of these
rules before it is marked complete:

- `publish.ps1` passes from the project root.
- Relevant deterministic, protocol, and server tests pass.
- Every required interaction has a visible tap or click path; keyboard input
  is optional assistance only.
- Shared resources, property, and world changes remain server-authoritative
  and survive a restart whenever the phase changes persistent state.
- Replayed or retried commands cannot duplicate items, currency, project
  contributions, or rewards.
- The phase's human acceptance scenario is recorded, with current verification
  screenshots stored directly in `docs/verification/`.
- Every Rust source and test file remains at or below 800 physical lines.

Complex soil simulation, advanced animal husbandry, large profession trees,
player government, bulk-freight restrictions, advanced beacon
specialisations, dynamic NPC populations, large regional economies, PvP, and
extensive combat progression remain outside this track until its simple
activities and economic connections are enjoyable.

## Phase boundaries

Each phase must leave a playable, restartable build. A phase is not complete
because its server endpoints exist; it is complete when players can use the
systems together and the client communicates failure clearly.

The game keeps one important authority rule throughout the roadmap:

- The server owns identity, validation, world time, shared state, and durable
  progress once Phase 1 begins.
- The client owns presentation, input intent, local UI state, credentials, and
  a temporary disconnected/error view.
- Local toolkit save slots may hold UI preferences or development fixtures, but
  they must not become a second source of truth for online inventory, crops,
  land, or character progression.

## Toolkit networking boundary

Phase 1 enables the optional toolkit feature in the client crate:

```toml
macroquad-toolkit = { path = "../macroquad-toolkit", features = ["net"] }
```

The client uses `macroquad_toolkit::net::HttpClient` for request construction
and `Pending<T>` for frame-polled responses. A request is retained and polled
from the Macroquad update loop with `poll()` or `poll_timed(dt, timeout)`; no
blocking network call may run on the render thread.

Tarrowyn still owns its serializable protocol types, endpoint paths, auth and
session policy, reconnect cooldown, optimistic/pessimistic UI decisions, and
the server implementation. The toolkit owns the cross-platform HTTP transport,
JSON encoding/decoding, common headers, bearer-header helper, and timeout
failure path.

## Cross-phase quality bar

For a focused maintenance change, run the smallest test that exercises the
changed rule, then run formatting and diff checks. Reserve the workspace-wide
test/clippy release gate for a major milestone or a change that crosses
subsystem boundaries; do not repeat unrelated full-suite coverage for each
small element.

- `cargo fmt`, `cargo test`, and `cargo clippy --all-targets --all-features
  -- -D warnings` remain clean for the client and every new Rust crate.
- Every `.rs` file stays below the workspace’s 800-line limit.
- Browser play retains visible touch targets for every required action.
- Protocol errors, timeouts, reconnects, and rejected commands become readable
  in the client rather than disappearing into logs.
- A deterministic local fixture or test covers each server rule before it is
  exercised by a live client.
- `publish.ps1` remains the release validation path for the client.

## Design references

- [Phase 0 — The First Evening](PHASE_0.md)
- [Phase 1 — The Shared Road](PHASE_1.md)
- [Phase 2 — The Persistent Settlement](PHASE_2.md)
- [Phase 3 — The Living Frontier](PHASE_3.md)
- [Phase 4 — The Enduring Society](PHASE_4.md)
- [Phase 5 — The Roads Between](PHASE_5.md)
- [Phase 6 — The Lasting Realm](PHASE_6.md)

The Phase 3 live acceptance and restart instructions are in
[`PHASE_3_RUNBOOK.md`](PHASE_3_RUNBOOK.md).

Phase 4 implementation decisions, endpoint fixtures, and the settlement
playthrough are recorded in [`PHASE_4_DESIGN.md`](PHASE_4_DESIGN.md),
[`PHASE_4_RUNBOOK.md`](PHASE_4_RUNBOOK.md), and
[`PHASE_4_PLAYTHROUGH.md`](PHASE_4_PLAYTHROUGH.md).

Phases 5 and 6 are implemented release-candidate slices. Phase 4 proves that
one settlement can govern and sustain itself, Phase 5 proves that the settlement
model works across a region, and Phase 6 hardens that regional world for real
players and long-term operation. The implementation records are
[`PHASE_5_DESIGN.md`](PHASE_5_DESIGN.md),
[`PHASE_5_RUNBOOK.md`](PHASE_5_RUNBOOK.md),
[`PHASE_5_PLAYTHROUGH.md`](PHASE_5_PLAYTHROUGH.md),
[`PHASE_6_DESIGN.md`](PHASE_6_DESIGN.md),
[`PHASE_6_RUNBOOK.md`](PHASE_6_RUNBOOK.md),
[`PHASE_6_TEST_REPORT.md`](PHASE_6_TEST_REPORT.md), and
[`PRODUCTION_READINESS_REVIEW.md`](PRODUCTION_READINESS_REVIEW.md).
The open target-environment gates and deliberate product deferrals are kept in
[`PHASE_6_FOLLOW_UP_REGISTER.md`](PHASE_6_FOLLOW_UP_REGISTER.md).

- [The Years of Tarrowyn GDD](../The_Years_of_Tarrowyn_GDD.md)
