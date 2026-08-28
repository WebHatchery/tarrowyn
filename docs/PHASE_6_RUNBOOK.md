# Phase 6 release and operator runbook

## Release gate

Run `scripts/validate_content.ps1`, then
`scripts/run_release_gate.ps1`. The gate checks formatting, workspace tests,
clippy, content IDs, and `publish.ps1`. The output directory is a reproducible
release artifact; do not copy a live state file into the browser bundle.

Configuration is environment-specific. Set `TARROWYN_STATE_PATH`,
`TARROWYN_BACKUP_PATH`, `TARROWYN_PRODUCTION_SESSION_TTL_SECONDS`,
`TARROWYN_REFRESH_TTL_SECONDS`, and the identity-gateway deployment secret in
the secret manager. Set `TARROWYN_SUPPORT_OPERATOR_ACCOUNTS` to a comma-separated
allowlist of operator account IDs; an empty value fails closed. Keep development
guests on a separate state path.

MySQL is the selected shared-world database. Local preview uses the ignored
`.env.preview` file with `DB_DRIVER=mysql`, `DB_HOST`, `DB_PORT`,
`DB_DATABASE`, `DB_USERNAME`, and `DB_PASSWORD`. Never commit that file or copy
it into a release artifact. Production injects the same variables from its
secret manager. The server applies `server/migrations/0001_initial_world.sql`
on startup and refuses to listen if the pool or migration fails. JSON remains
the default for deterministic local fixtures; set `DB_DRIVER=mysql` explicitly
for the shared preview. Do not deploy it as a public multi-worker service
until the live MySQL, backup, restore, and rollback checks below pass.
The snapshot bridge also holds a process-lifetime `tarrowyn-world-authority`
advisory lock. A second worker pointed at the same database must fail startup;
do not interpret that rejection as multi-worker support. Use one worker for the
current bridge until relational ownership and cursor handoff are implemented.

For a configured local MySQL preview, run:

```powershell
.\scripts\verify_mysql.ps1
```

The script uses a separate HTTP port and temporary JSON backup, adds uniquely
named guest and linked production identities to the configured preview world,
checks the migration and animal projection, and replays chat, account-link,
refresh, revoke, and moderation requests. It submits eight overlapping retries
of one chat request and requires a single cached result, then restarts the
server and checks persisted auth-revoke and moderation replay results. Finally,
it uses `mysqldump.exe` and `mysql.exe` to restore into a generated temporary
database. It validates the restored world and identity index before dropping
that generated database and removing its temporary files. It does not reset or
delete the configured database.

For the local regional load and restart drill, run:

```powershell
.\scripts\phase6_load_test.ps1
```

The default run starts an isolated JSON worker on port 8799, creates 24 clients,
and performs three mixed rounds across state, cursorable events, movement, chat,
markets, and travel, including a deliberate invalid movement rejection probe.
It verifies the allowlisted support-account view exposes the expected character
and event cursor without access, refresh, or provider secrets, and that an
ordinary player receives HTTP 403 for the same view.
It also sends deliberately ahead cursors to both event endpoints and requires
the structured HTTP 409 `cursor_ahead` response before starting the mixed load.
It also checks price pressure, scarce goods, NPC fallback, abandoned claims,
declining settlements, newcomer access, and alert fields in operator metrics,
then proves ordinary players cannot read that endpoint.
It measures the mixed-load wall time and server working set, checks scheduled
backup and operator metrics, waits for server-owned travel arrival, restarts the
worker, times restart-to-readiness recovery, and verifies the travelled location.
The script removes its temporary
state, backup, and process environment on completion. Pass `-ClientCount` or
`-Rounds` to extend the fixture; the client target remains at least 24. The
default run rejects every operational alert. For an exploratory boundary run
that intentionally exceeds the 32-open-order warning threshold, pass
`-AllowedAlertFlags market_backlog`; any alert not explicitly allowed still
fails the drill.

## Deploy, rollback, and maintenance

1. Check `/v1/ops/health`, the last backup tick, `persistence_error`, and
   `backup_error`; do not admit traffic while readiness is degraded.
2. Check authenticated `/v1/ops/metrics` for the measured average and latest
   tick durations, `tick_drift_count`, regional event backlog, and
   `alert_flags`. Also watch average price index, scarce goods, NPC fallback
   households, abandoned claims, declining settlements, and `newcomer_access`.
   Route persistence, backup, tick-drift, regional-backlog, and economy-anomaly
   alerts to the deployment on-call. Client connection failures must be reported
   by client/deployment telemetry.
3. Announce a maintenance window through the client status message.
4. Stop the worker only after the current persistence write completes.
5. Deploy the immutable release artifact and run the readiness check. For a
   MySQL deployment, confirm the migration table and world row through the
   database operator view before admitting traffic.
6. If readiness or migration checks fail, stop the new worker and restore the
   previous artifact plus the last known-good state backup to a new named path.
7. Reconcile the event cursor, travel records, orders, claims, and chronicle
   before reopening player access.

Chronicle support is split between the normal settlement view and the
authenticated search path. The view is intentionally bounded to recent
entries plus its archive summary; use `/v1/chronicle/search?q=...&since=...`
when investigating older history. After an account deletion, verify the
recent window, archived records, and retained chronicle event records contain
`Former resident` rather than the deleted display name.

Rollback never replays rewards locally. A deployment mismatch is reported as a
maintenance/reconnect state and the client waits for an authoritative response.

## Restore and repair

Run `scripts/phase6_failure_drill.ps1` with the active JSON state and its backup
present. The drill fingerprints the active state, parses the backup, checks
storage version, starts a server on a temporary port, reads `/v1/ops/health`,
and verifies that the active fingerprint is unchanged; it does not overwrite the
active world. The MySQL backend currently exposes the same versioned snapshot as a
transactional bridge. `scripts/verify_mysql.ps1` covers the local migration,
restart, duplicate-request, backup, and native dump/restore portion of that
gate; target-environment failover, concurrency, and rollback drills remain
explicit. For a stuck journey use `ClearStuckTravel`, for a duplicate or invalid
inventory use `NormalizeInventory`, and for an open failed shipment use
`ReconcileTrade`. For a lost access flag on an active, non-expired claim, use
`RestoreClaim` with the target account and claim ID; it never extends the lease
or assigns ownership. For duplicated regional household records, use
`MergeHousehold` with the regional household ID; it retains distinct history
entries and refuses conflicting identity or same-tick status records. Include
the player-facing reason and the support note in the ticket. Repeat the same
request ID to confirm safe idempotency.

## Moderation and support

Reports from `/v1/moderation/report` begin in `queued` status and never expose
support credentials to the client. A moderator records the category, target,
evidence reference, decision, and retention deadline. Resolve through the
audited support repair surface using a token for an account in the configured
operator allowlist. Ordinary player tokens receive a clear forbidden response.
The allowlisted `GET /v1/support/account?account_id=...` view shows account ID,
character ID, settlement history, claims, trades, and cursors, but never access
tokens, refresh tokens, provider secrets, or raw credentials. Ordinary player
tokens must receive the forbidden response.

## Player incident path

The client displays connection, maintenance, migration, rate-limit, and
moderation responses in the visible notice area. The player should tap
Reconnect, Recover, Account, Logout, or Report as named by the message. For
privacy deletion, use the visible Delete control after Account shows a linked
production account: tap Delete once to arm it, then tap the relabelled
Tap again control to submit `/v1/account/delete`. The response is scheduled
for the next authoritative tick; the client clears the session and exposes
Reconnect so the player can return as a fresh guest. A development guest must
link first. During a restore-era cursor mismatch, the client recognizes the
`cursor_ahead` API code, clears stale cursor-derived projections, cancels the
old history requests, reloads authoritative state from cursor zero, and does
not present cached history as a current reward.
