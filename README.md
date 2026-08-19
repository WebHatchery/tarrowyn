# The Years of Tarrowyn — Phase 1

The first multiplayer slice is now a small authoritative client/server build.
The native development server owns guest identity, the shared collision map,
movement validation, presence, chat ordering, session expiry, and the
accelerated world clock. The Macroquad client renders that projection and
retains the Phase 0 first-evening fixture as an explicitly labelled offline
mode.

## Run the shared road locally

From this directory, start the server in one terminal:

```powershell
cargo run -p tarrowyn-server
```

Then start the client in another:

```powershell
$env:TARROWYN_SERVER_URL = "http://127.0.0.1:8787"
cargo run -p years_of_tarrowyn
```

The client never performs a blocking HTTP call on the render thread. Guest,
world, event, movement, and chat requests are retained as toolkit
`Pending<T>` values and polled once per frame. A timeout moves the client to a
readable degraded/offline state; the visible `Reconnect` action is subject to
an application-owned cooldown.

For the local first-evening fixture, use a separate terminal or the visible
`Use offline fixture` action:

```powershell
$env:TARROWYN_OFFLINE = "1"
cargo run -p years_of_tarrowyn
```

Offline save slots are development fixtures only. They are not used as an
online source of truth.

## Verify three clients

The deterministic repository fixtures run with:

```powershell
cargo test --workspace
```

The HTTP acceptance pass starts one server, creates three guest identities,
checks shared presence and server collision rejection, exchanges ordered chat,
and confirms the world clock advances independently of request count:

```powershell
.\scripts\verify_three_clients.ps1
```

## Architecture decisions

Phase 1 uses `tiny_http` for the native server process and an in-memory
`WorldRepository` behind the HTTP handlers. The repository boundary is
intentional: Phase 1 proves the wire and authority model without pretending
that guest development data is durable. Phase 2 can add SQLite persistence
behind the same repository boundary.

The shared `protocol/` crate is versioned at protocol `1`. Every successful or
error response carries protocol version and server tick metadata; cursor-based
event responses additionally carry the event cursor. See
[`docs/PHASE_1_RUNBOOK.md`](docs/PHASE_1_RUNBOOK.md) for reset, configuration,
fixture, and capture details.

## Release validation

```powershell
cargo fmt --package years_of_tarrowyn -- --check
cargo fmt --manifest-path protocol/Cargo.toml -- --check
cargo fmt --manifest-path server/Cargo.toml -- --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
.\publish.ps1
```
