# Phase 6 release and operator runbook

## Release gate

Run `scripts/validate_content.ps1`, then
`scripts/run_release_gate.ps1`. The gate checks formatting, workspace tests,
clippy, content IDs, and `publish.ps1`. The output directory is a reproducible
release artifact; do not copy a live state file into the browser bundle.

Configuration is environment-specific. Set `TARROWYN_STATE_PATH`,
`TARROWYN_BACKUP_PATH`, `TARROWYN_PRODUCTION_SESSION_TTL_SECONDS`,
`TARROWYN_REFRESH_TTL_SECONDS`, and the identity-gateway deployment secret in
the secret manager. Keep development guests on a separate state path.

MySQL is the selected shared-world database. Local preview uses the ignored
`.env.preview` file with `DB_DRIVER=mysql`, `DB_HOST`, `DB_PORT`,
`DB_DATABASE`, `DB_USERNAME`, and `DB_PASSWORD`. Never commit that file or copy
it into a release artifact. Production injects the same variables from its
secret manager. The current JSON-backed release candidate does not consume
this database contract yet; do not deploy it as a public multi-worker service
until the MySQL repository, migrations, backup, restore, and rollback checks
are implemented and pass.

## Deploy, rollback, and maintenance

1. Check `/v1/ops/health` and the last backup tick.
2. Announce a maintenance window through the client status message.
3. Stop the worker only after the current persistence write completes.
4. Deploy the immutable release artifact and run the readiness check.
5. If readiness or migration checks fail, stop the new worker and restore the
   previous artifact plus the last known-good state backup to a new named path.
6. Reconcile the event cursor, travel records, orders, claims, and chronicle
   before reopening player access.

Rollback never replays rewards locally. A deployment mismatch is reported as a
maintenance/reconnect state and the client waits for an authoritative response.

## Restore and repair

Run `scripts/phase6_failure_drill.ps1` against a copy of the state and backup.
The drill parses the backup, checks storage version, starts a server on a
temporary port, and reads `/v1/ops/health`; it does not overwrite the active
world. For a stuck journey use `ClearStuckTravel`, for a duplicate or invalid
inventory use `NormalizeInventory`, and for an open failed shipment use
`ReconcileTrade`. Include the player-facing reason and the support note in the
ticket. Repeat the same request ID to confirm safe idempotency.

## Moderation and support

Reports from `/v1/moderation/report` begin in `queued` status and never expose
support credentials to the client. A moderator records the category, target,
evidence reference, decision, and retention deadline. Resolve through the
audited support repair surface. Account views show account ID, character ID,
settlement history, claims, trades, and cursors, but never access tokens,
refresh tokens, provider secrets, or raw credentials.

## Player incident path

The client displays connection, maintenance, migration, rate-limit, and
moderation responses in the visible notice area. The player should tap
Reconnect, Recover, Account, Logout, or Report as named by the message. During
a restore-era cursor mismatch, the server response is reloaded and no cached
success is presented as a reward.
