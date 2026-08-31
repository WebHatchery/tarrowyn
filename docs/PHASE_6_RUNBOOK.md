# Phase 6 release and operator runbook

## Release gate

Run `scripts/validate_content.ps1`, then
`scripts/run_release_gate.ps1`. The gate checks formatting, workspace tests,
clippy, content IDs, and `publish.ps1`. The output directory is a reproducible
release artifact; do not copy a live state file into the browser bundle.

## Scoped change checks

For a change confined to one package or subsystem, run the narrowest matching
`cargo test` filter, then the affected package's clippy and formatting checks,
`git diff --check`, and the Rust file-size scan. Keep unrelated tests filtered
out and record the exact filter and result in `docs/PHASE_6_TEST_REPORT.md`.

Reserve `scripts/run_release_gate.ps1` and `publish.ps1` for a major milestone,
a cross-package or cross-subsystem change, content/schema changes that affect
the release artifact, or the scheduled release-readiness check. This keeps
ordinary maintenance feedback proportional while retaining a complete gate at
the points where integration risk changes.

Configuration is environment-specific. Set `TARROWYN_STATE_PATH`,
`TARROWYN_BACKUP_PATH`, `TARROWYN_PRODUCTION_SESSION_TTL_SECONDS`,
`TARROWYN_REFRESH_TTL_SECONDS`, and
`TARROWYN_GUEST_SESSION_BURST_LIMIT` when the deployment needs a different
per-source guest-session burst than the secure default of 32. Set the latter
only to the smallest value needed for the expected client bootstrap rate; the
Phase 6 load harness sets it explicitly for controlled local capacity probes.
Keep the identity-gateway deployment secret in the secret manager. Set
`TARROWYN_SUPPORT_OPERATOR_ACCOUNTS` to a comma-separated
allowlist of operator account IDs; an empty value fails closed. Keep development
guests on a separate state path.

New production access and refresh credentials are generated from the operating
system's secure random source and are never derived from the session counter.
Existing persisted counter-shaped credentials are legacy sessions; let their
configured expiry elapse and revoke them if compromise is suspected.

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

The checker first resolves `mysql.exe` and `mysqldump.exe`, runs a password-safe
`SELECT 1`, and probes creation and removal of one uniquely named disposable
restore database. Missing tools, rejected credentials, or insufficient
`CREATE/DROP DATABASE` permission fail before the preview server starts, so a
readiness timeout does not conceal a local database dependency problem.

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
After the mixed load, it temporarily advances a 1 ms worker past the 2,048
record retention window and requires structured HTTP 409 `cursor_stale` responses
from both endpoints before restoring the normal 250 ms cadence and restart check.
It also checks price pressure, scarce goods, NPC fallback, open market fallback,
abandoned claims, declining settlements, newcomer access, and alert fields in
operator metrics,
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
   `backup_error`; the `integrity_ok` field also covers durable
   cross-references across the world, regional, and production-identity
   ledgers. When it is false, use the fixed, non-sensitive `integrity_failures`
   codes to identify the affected boundary before choosing restore or audited
   support repair. The public response intentionally omits the configured
   backup filesystem path; use the deployment operator's configured path and
   server logs when locating a backup. Do not admit traffic while readiness is
   degraded, and do not edit a live snapshot in place.
2. Check authenticated `/v1/ops/metrics` for the measured average and latest
   tick durations, `tick_drift_count`, regional event backlog, and
   `alert_flags`. Also watch `http_request_workers`,
   `http_request_queue_capacity`, `http_active_requests`, `http_queue_depth`,
   `http_queue_peak`, and `http_queue_full_events` for transport pressure.
   Watch average price index, scarce goods, NPC fallback households, open
   market fallback orders, abandoned claims, declining settlements, and
   `newcomer_access` as well.
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
`Former resident` rather than the deleted display name. Also verify that any
open or failed market order owned by the deleted account is cancelled and its
real escrow has returned to the order's origin stock. A travelling fallback
order is cancelled without a refund because it never held player escrow.

History endpoints reject malformed `since` values with a structured
`invalid_cursor` response; they do not silently treat a broken cursor as zero.
Chronicle search also rejects malformed form encoding in `q` instead of
silently widening the search to an empty query.

Rollback never replays rewards locally. A deployment mismatch is reported as a
maintenance/reconnect state and the client waits for an authoritative response.

## Restore and repair

Run `scripts/phase6_failure_drill.ps1` with the active JSON state and its backup
present. The drill fingerprints the active state, parses the backup, checks
storage version, starts a server on a temporary port, reads `/v1/ops/health`,
and verifies that the active fingerprint is unchanged; it does not overwrite the
active world. A cold `cargo run` is allowed a longer startup window, and a
startup failure includes the temporary server's captured stdout and stderr.
Older JSON snapshots are upgraded in memory before readiness is checked: missing
regional fallback-window metadata is restored from the saved clock, bounded
household opportunity scores are clamped, and orphaned public-history or audit
actors are anonymised as former residents. The MySQL backend currently exposes
the same versioned snapshot as a transactional bridge. `scripts/verify_mysql.ps1`
covers the local migration,
restart, duplicate-request, backup, and native dump/restore portion of that
gate; target-environment failover, concurrency, and rollback drills remain
explicit. For a stuck journey use `ClearStuckTravel`, for a duplicate or invalid
inventory use `NormalizeInventory`, and for an open or failed shipment use
`ReconcileTrade`; the operation restores real origin escrow before closing the
order. For a travelling fallback order, reconciliation closes it without
inventing an escrow refund. `ClearStuckTravel` returns the character to the journey's recorded origin
and rejects an already arrived or missing journey. For a lost access flag on an
active, non-expired claim, use
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
HTTP authentication accepts the standard case-insensitive Bearer scheme and
rejects missing or control-character credentials before they reach the
repository boundary.
The allowlisted `GET /v1/support/account?account_id=...` view shows account ID,
character ID, settlement history, claims, trades, and cursors, but never access
tokens, refresh tokens, provider secrets, or raw credentials. Ordinary player
tokens must receive the forbidden response.

## Player incident path

The client displays connection, maintenance, migration, rate-limit, and
moderation responses in the visible notice area. The player should tap
Reconnect, Recover, Account, Logout, or Report as named by the message. For
an explicit Logout or an expired production session, Reconnect starts a fresh
guest fixture in the local release candidate; a configured production
identity gateway can replace that fixture with provider sign-in at deployment.
After a transport failure or worker restart, a still-valid linked production
session is recovered through its refresh credential before the client requests
authoritative state; only explicit logout, failed refresh, or the absence of a
refresh session uses the fresh-guest path.
For privacy deletion, use the visible Delete control after Account shows a linked
production account. Account opens the link path for a guest fixture and a
read-only identity and character view for an already-linked character; the
latter cannot submit a second link request. The Delete control is disabled for
a guest until linking succeeds; then tap the relabelled Tap again control to
submit `/v1/account/delete`. The response is scheduled for the next
authoritative tick; the client clears the session and exposes Reconnect so the
player can return as a fresh guest. A development guest must link first.
During a restore-era or retention-era cursor mismatch, the client recognizes
the `cursor_ahead` or `cursor_stale` API code, clears stale cursor-derived
projections, cancels the old history requests, reloads authoritative state from
cursor zero, and does not present cached history as a current reward.
