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

The native client honors `TARROWYN_SERVER_URL`, but a static browser WASM
artifact cannot read the server process environment at runtime. Before a WebGL
release, set the embedded `gateway_url` in
`assets/data/game_config.json` to the deployment's explicit HTTPS API origin or
same-origin reverse-proxy route. An empty value keeps
`http://127.0.0.1:8787` as the local development fallback and is not a
production endpoint.

For the shared gateway deployment, the provisioned client value is expected to
be `https://webhatchery.au/local_gateway/api/p/tarrowyn` (or the equivalent
HTTPS tunnel URL); do not put that value in the checked-in config until the
service policy and target registration are live.

When using the shared `local_gateway` reverse proxy, first add `tarrowyn` to its
service policy, then run the authority server with a reachable bind address and
keep the registration heartbeat alive:

```powershell
$env:TARROWYN_SERVER_ADDR = "0.0.0.0:8787"
cargo run -p tarrowyn-server

$env:GATEWAY_ADMIN_TOKEN = "<gateway token from the secret manager>"
.\register-server.ps1
```

For an HTTPS tunnel, pass its base URL with `-Target` instead of forwarding the
port. The helper never stores or commits the gateway token.

`.\publish.ps1 -Production` fails closed when `gateway_url` is blank or is not
an HTTPS origin or same-origin path, so a production browser artifact cannot be
published while it would still call the native loopback fallback.

After the gateway service and registration are live, verify both hops with the
read-only check below:

```powershell
.\scripts\verify_gateway.ps1
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

At a major milestone, the complete deterministic repository suite runs with:

```powershell
cargo test --workspace
```

For this focused HTTP acceptance pass, use the harness below. It starts one
server, creates three guest identities, checks shared presence and server
collision rejection, exchanges ordered chat, and confirms the world clock
advances independently of request count:

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
The acceptance script also creates and removes a uniquely named disposable
restore database before starting the server, so its database account needs
`CREATE/DROP DATABASE` permission in addition to access to `DB_DATABASE`.
The HTTP worker keeps the guest-session burst limit at 32 attempts per source
by default; set `TARROWYN_GUEST_SESSION_BURST_LIMIT` only when the deployment's
known client bootstrap rate requires a different bounded value.
The HTTP pool selects host parallelism automatically and clamps it to 4–32
workers; deployments may set `TARROWYN_HTTP_REQUEST_WORKERS` to `0` for that
automatic mode or a measured value in the same range. Queue capacity defaults
to 128 and is clamped to 16–4096 through `TARROWYN_HTTP_QUEUE_CAPACITY`.
The MySQL pool reserves one connection for the world-authority lock and limits
the total backend pool to a maximum of 4 connections by default; set
`TARROWYN_MYSQL_POOL_MAX_CONNECTIONS` only to a measured value from 2–32.

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
`/v1/support/account`, `/v1/support/repair`, `/v1/moderation/report`, `/v1/ops/health`,
`/v1/ops/metrics`, `/v1/chronicle/search`, and `/v1/skills`; the latter exposes
the server-owned root catalogue, vague merger hints, authoritative
per-character mastery, a touchable chooser for every available root practice, a
school lesson action for qualified nearby players, and varied adventurer
credentials in player projections. See the Phase 5 and Phase 6 runbooks.
The operator metrics response also exposes the bounded HTTP worker count,
queue capacity, active requests, current and peak queue depth, and queue-full
events for target-environment pressure checks.

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
.\scripts\package_server_release.ps1
.\scripts\write_release_manifest.ps1
```

The command list above is the full release gate for a major milestone or a
change that crosses subsystem boundaries. For focused maintenance, run only
the smallest changed-subsystem test and matching package clippy, plus
formatting, diff, size, and publisher checks when runtime files changed.

Run `.\scripts\verify_concept_prototype.ps1` for the focused GDD role-loop
fixture. It starts a fresh JSON worker and places a farmer, adventurer, and
wayfarer in the same world to verify shared farming, exchange, tavern
coordination, repeatable contract play, and regional travel. This automated
fixture strengthens the concept evidence but does not replace a human
multi-session playthrough.

The Phase 5 and Phase 6 design decisions, playthrough, test report, operator
runbook, and production-readiness review are recorded in `docs/`. The current
release candidate retains JSON as the deterministic default and keeps the
MySQL backend as a single-worker snapshot bridge; the readiness review records
the live database, multi-worker, identity-gateway, and operational drills that
must precede public access.

`verify_mysql.ps1` is an explicit local-preview check rather than part of the
default release gate because it requires the ignored `.env.preview` credentials
and writes one uniquely named guest identity to that configured database.

The major-milestone gate also builds the host-targeted authoritative server
package, writes `dist/tarrowyn_release_manifest.json`, and emits one `.sha256`
sidecar per Windows/WebGL/server archive. Preserve a clean candidate for
rollback with `scripts/preserve_release_candidate.ps1`; after a later candidate
exists, rehearse the isolated patch, rollback, and patch-restoration sequence
with `scripts/rehearse_release_rollback.ps1 -PreservedDir <directory>`. These
records contain archive hashes and source identity only; the ignored server
state and backup files are never release inputs. The server package is still
host-targeted by default until the production OS or container contract is
selected. An installed Rust target can be exercised explicitly with
`scripts/package_server_release.ps1 -Target <rust-target>`; this does not by
itself select or approve a production platform.
The complete gate accepts the same target through
`scripts/run_release_gate.ps1 -ServerTarget <rust-target>`.
