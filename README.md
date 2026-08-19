# The Years of Tarrowyn — Phase 2

The shared settlement is now a small persistent authoritative client/server
build. The native development server owns guest identity, characters, the
accelerated clock, farming plots, inventory, trades, presence, tavern history,
frontier threats, contracts, household opportunities, claims, and a pioneer
outpost. The Macroquad client renders accepted projections and retains the
Phase 0 first-evening fixture as an explicitly labelled offline mode.

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
state, event, movement, chat, farming, trade, and frontier requests are retained as
toolkit `Pending<T>` values and polled once per frame. A timeout moves the
client to a readable degraded/offline state; the visible `Reconnect` action is
subject to an application-owned cooldown and retry limit. Inventory, gold,
crop growth, and completed trades only change after an accepted server
projection.

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

Run the Phase 2 farming, trading, tavern, and restart acceptance pass with:

```powershell
.\scripts\verify_phase2.ps1
```

Run the Phase 3 threat, contract, recovery, chronicle, claim, expedition, and
10-client polling acceptance pass with:

```powershell
.\scripts\verify_phase3.ps1
```

## Architecture decisions

The server uses `tiny_http` for the native process and a mutex-protected
`WorldRepository` behind the HTTP handlers. The repository writes a versioned
JSON state document to `TARROWYN_STATE_PATH` (default
`dist/tarrowyn-server-state.json`) after authoritative mutations and clock
ticks. Sessions and bearer tokens are intentionally ephemeral; the guest
client key resolves the durable account and character again after a restart.
The storage version is part of the document so future migrations can be added
without changing the protocol boundary.

The shared `protocol/` crate is versioned at protocol `3`. Every successful or
error response carries protocol version and server tick metadata; cursor-based
event responses additionally carry the event cursor. See
[`docs/PHASE_1_RUNBOOK.md`](docs/PHASE_1_RUNBOOK.md) for reset, configuration,
fixture, and capture details, [`docs/PHASE_2_RUNBOOK.md`](docs/PHASE_2_RUNBOOK.md)
for persistence and acceptance details, and [`docs/PHASE_3_RUNBOOK.md`](docs/PHASE_3_RUNBOOK.md)
for the frontier acceptance. Phase 3 adds `/v1/contracts`,
`/v1/combat/actions`, `/v1/recovery`, `/v1/settlement/chronicle`,
`/v1/settlement/opportunities`, `/v1/claims`, and `/v1/expeditions`.

## Release validation

```powershell
cargo fmt --package years_of_tarrowyn -- --check
cargo fmt --manifest-path protocol/Cargo.toml -- --check
cargo fmt --manifest-path server/Cargo.toml -- --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
.\publish.ps1
```
