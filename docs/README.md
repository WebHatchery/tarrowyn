# The Years of Tarrowyn — Development Roadmap

This folder is the working roadmap for the game. The GDD describes the long
term design direction; these phase documents describe the smallest buildable
steps toward it.

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

Phases 4–6 are planning documents. Their runbooks should be added when each
phase enters implementation, after the open design decisions listed in the
phase have been resolved. The intended dependency is sequential: Phase 4
proves that one settlement can govern and sustain itself, Phase 5 proves that
the settlement model works across a region, and Phase 6 hardens that regional
world for real players and long-term operation.

- [The Years of Tarrowyn GDD](../The_Years_of_Tarrowyn_GDD.md)
