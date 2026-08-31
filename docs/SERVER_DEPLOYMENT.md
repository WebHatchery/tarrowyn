# Tarrowyn authoritative server package

This directory is packaged with the target-specific `tarrowyn-server` release
binary and its build identity. The package is an internal deployment artifact,
not a production approval. It contains no database credentials, live state, or
backup data.

## Package contract

The server package defaults to the Rust host target reported by `rustc -vV`.
For an installed deployment target, pass `-Target <rust-target>` to
`scripts/package_server_release.ps1`; the package records that exact target in
`BUILD_INFO.json`. Do not copy a package to a different operating-system or
architecture target. Choose the production server target and process
supervisor before treating this package as a deployment candidate. The binary
embeds the validated content catalogues and the MySQL migration;
`server/migrations/0001_initial_world.sql` is included as an operator-visible
schema record.

Inject environment values through the target secret manager or process
supervisor. At minimum, a shared MySQL world needs `DB_DRIVER=mysql`,
`DB_HOST`, `DB_PORT`, `DB_DATABASE`, `DB_USERNAME`, and `DB_PASSWORD`.
Configure `TARROWYN_SERVER_ADDR` for the reachable bind address and set
`TARROWYN_STATE_PATH` and `TARROWYN_BACKUP_PATH` to external durable paths.
Never put `.env` files, credentials, or live JSON state beside the package.

Before admitting traffic, check `/v1/ops/health`, migration readiness, the
last backup tick, and the integrity projection. Keep one authoritative MySQL
worker until the documented multi-worker handoff design is proven. The
gateway registration and browser API-origin checks remain separate deployment
steps.

## Rollback boundary

Stop the worker after its current persistence write completes, record the
active package manifest and state-backup identity, and deploy the exact
previous server package without rebuilding it. If state must be restored,
restore the backup to a new named path, validate readiness and event cursors,
then reconcile travel, trades, claims, and chronicle records before reopening
traffic. Never replay rewards locally or edit a live snapshot in place.

The project-local `scripts/rehearse_release_rollback.ps1` checks the packaged
Windows client and server archives through patch → rollback → patch restored.
It validates external manifests, checksum sidecars, and each package's
embedded `BUILD_INFO.json` identity without contacting a deployment target.
