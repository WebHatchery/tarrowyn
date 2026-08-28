# The Years of Tarrowyn — Phase 6 release candidate

The shared region is a persistent authoritative client/server build. The native
server owns guest and linked production identity, characters, the accelerated
clock, farming plots, inventory, trades, presence, tavern history, frontier
threats, contracts, households, claims, settlements, travel, routes, markets,
regional events, backups, audits, and support repair. The Macroquad client
renders accepted projections and keeps the Phase 0 first-evening fixture as an
explicitly labelled offline mode.

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
state, event, movement, chat, farming, trade, frontier, travel, market, event,
and account requests are retained as toolkit `Pending<T>` values and polled
once per frame. A timeout moves the client to a readable degraded/offline
state; visible `Reconnect`, `Recover`, `Account`, `Logout`, and `Report`
controls remain available. Inventory, gold, crop growth, travel arrival, and
completed trades only change after an accepted server projection.

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
concurrent 20-client polling acceptance pass with:

```powershell
.\scripts\verify_phase3.ps1
```

## Architecture decisions

The server uses `tiny_http` for the native process and a mutex-protected
`WorldRepository` behind the HTTP handlers. `DB_DRIVER=json` (the default)
writes a versioned JSON state document to `TARROWYN_STATE_PATH` (default
`dist/tarrowyn-server-state.json`) after authoritative mutations and clock
ticks. `DB_DRIVER=mysql` opens the selected MySQL backend, applies the checked-
in migration in `server/migrations/`, and stores the same authoritative
snapshot plus an account/character index in one transaction. Sessions and
bearer tokens are intentionally ephemeral; the guest client key resolves the
durable account and character again after a restart. The storage version is
part of both representations so the protocol boundary remains unchanged.

## Preview database decision

MySQL is the selected durable database for shared preview and production
worlds. The ignored `.env.preview` file carries the local preview contract:

```dotenv
DB_DRIVER=mysql
DB_HOST=localhost
DB_PORT=3306
DB_DATABASE=tarrowyn
DB_USERNAME=
DB_PASSWORD=
```

Credentials remain local and must not be committed or included in browser or
Windows packages. The server reads these variables from its process
environment; load the ignored file through the deployment or preview launcher
before starting the server. MySQL startup fails before the HTTP listener if
the pool or migration cannot be established. Public deployment remains
blocked until a live MySQL acceptance run, concurrent-write tests, database
backup/restore drill, and rollback path pass on the target environment.

The shared `protocol/` crate is versioned at protocol `6`. Every successful or
error response carries protocol version and server tick metadata; cursor-based
event responses additionally carry the event cursor. The development server
uses versioned JSON with atomic replacement and scheduled backups; production
sessions are expiring, refreshable, revocable, and separate from guest
fixtures. See
[`docs/PHASE_1_RUNBOOK.md`](docs/PHASE_1_RUNBOOK.md) for reset, configuration,
fixture, and capture details, [`docs/PHASE_2_RUNBOOK.md`](docs/PHASE_2_RUNBOOK.md)
for persistence and acceptance details, and [`docs/PHASE_3_RUNBOOK.md`](docs/PHASE_3_RUNBOOK.md)
for the frontier acceptance. Phase 3 adds `/v1/contracts`,
`/v1/combat/actions`, `/v1/recovery`, `/v1/settlement/chronicle`,
`/v1/settlement/opportunities`, `/v1/claims`, and `/v1/expeditions`. Phase 4
adds authoritative governance, infrastructure, claim lifecycle, profession
orders, knowledge, household, and local-combat endpoints. Phase 5 adds
`/v1/region`, `/v1/travel`, `/v1/routes`, `/v1/market/orders`,
`/v1/events/region`, `/v1/households/region`, and `/v1/law`. Phase 6 adds
`/v1/auth/link`, `/v1/auth/refresh`, `/v1/auth/revoke`, `/v1/account`,
`/v1/account/delete`,
`/v1/support/repair`, `/v1/moderation/report`, `/v1/ops/health`,
`/v1/ops/metrics`, `/v1/chronicle/search`, and `/v1/skills`; the latter exposes
the server-owned root catalogue, vague merger hints, authoritative
per-character mastery, a touchable first-practice action for every root, a
school lesson action for qualified nearby players, and varied adventurer
credentials in player projections. See the Phase 5 and Phase 6 runbooks.

## Phase 5 and 6 release validation

```powershell
cargo fmt --package years_of_tarrowyn -- --check
cargo fmt --manifest-path protocol/Cargo.toml -- --check
cargo fmt --manifest-path server/Cargo.toml -- --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
.\scripts\validate_content.ps1
.\scripts\phase5_region_soak.ps1
.\scripts\phase6_failure_drill.ps1 -StatePath <state> -BackupPath <backup>
.\scripts\verify_mysql.ps1
.\publish.ps1
```

The Phase 5 and Phase 6 design decisions, playthrough, test report, operator
runbook, and production-readiness review are recorded in `docs/`. The current
release candidate retains JSON as the deterministic default and keeps the
MySQL backend as a single-worker snapshot bridge; the readiness review records
the live database, multi-worker, identity-gateway, and operational drills that
must precede public access.

`verify_mysql.ps1` is an explicit local-preview check rather than part of the
default release gate because it requires the ignored `.env.preview` credentials
and writes one uniquely named guest identity to that configured database.
