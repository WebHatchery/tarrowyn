# Phase 6 decision record — the lasting realm

## Identity and privacy

Production identity is delegated to the WebHatchery Identity Gateway through
the `webhatchery-identity-oidc` provider contract. The server stores the
provider subject mapping, never a provider credential. A guest fixture may be
linked to that subject once; the character ID remains the durable character
boundary. Production access tokens are opaque, short-lived, rotated on refresh,
and revocable. Refresh tokens are stored only as server-side session records.
Logout revokes one or all sessions. Credential recovery and MFA remain the
identity gateway's responsibility; the game server receives only a verified
subject.

Account identity is retained until deletion. Chat reports are retained for 90
days, while settlement history is retained as public world history with
account identifiers minimised. Payment-free gameplay has no payment data. A
future deletion job must remove provider mapping and private support data while
preserving an anonymised chronicle entry where required for world continuity.

All mutation endpoints validate bounded request IDs, use server authorization,
and retain idempotent results where retries can happen. Chat has length and
per-tick limits; trades, claims, governance, moderation, and support repairs
are recorded in the audit stream. The no-PvP law boundary is still active.

## Persistence, backups, and repair

Storage version 15 adds persisted auth-link and auth-refresh replay results
alongside the persisted Bellweather animal condition and daily care state,
the reposition opening and wind spark,
field-tool condition, and real-time lease timestamps
and public tax receipts,
the regional state,
production session/audit records, and
the per-character skill ledger while retaining defaults for Phase 1–6 files.
The repository now has two selectable backends: JSON with atomic temporary-file
replacement for deterministic fixtures, and MySQL with the checked-in
`0001_initial_world.sql` migration. The MySQL bridge stores the versioned
authoritative snapshot and a transactional account/character index; it keeps
the existing protocol and repository rules intact while the schema is being
proven against a live environment.
The server writes a scheduled backup to the configured backup path and reports
the last successful tick through `/v1/ops/health`. Restore drills validate the
backup as JSON before serving it as a named state path; a restore is never an
in-place destructive command.

The remaining persistence gate is deliberately explicit: run the MySQL
migration and restart/duplicate-request/partial-write tests against the target
database, then prove database backup and restore rather than relying only on
the JSON snapshot companion. The current in-process repository still targets
one authoritative worker; multi-worker locking and relational decomposition
remain follow-up work.

Repair ownership is explicit. The world authority owns travel, inventory,
market orders, claims, households, and moderation state. The support surface
fails closed unless the authenticated account appears in the deployment's
`TARROWYN_SUPPORT_OPERATOR_ACCOUNTS` allowlist, then accepts repeatable, audited operations for stuck travel, inventory
normalisation, trade reconciliation, claim handoff, household history, and
moderation resolution. Unsupported repairs return a clear reason rather than
guessing at state. Every repair carries an operator note and audit ID.

## Hosting and observability

The release candidate runs one region-authoritative worker behind a TLS reverse
proxy, with the browser bundle delivered by the existing publishing path. The
native worker binds only to its configured private service address; TLS,
provider secrets, and support credentials are environment or deployment-secret
inputs, never checked into the repository. Development guest identities use a
separate fixture state path and must not share production data.

`/health` remains a simple process check. `/v1/ops/health` is a readiness and
integrity check. Authenticated `/v1/ops/metrics` reports sessions, accounts,
regional visibility, event backlog, open orders, travel recovery load, command
rejections, and tick latency. Alerts are raised for integrity failures, market
backlog, and interrupted-travel backlog. Slow clients receive bounded,
cursorable projections and do not own the world tick.

## Scale, calendar, and legacy

The measured target for this release candidate is 24 concurrent development
clients, 50 regional orders, and a 250 ms tick without blocking on a slow
client. This is a regional target, not a promise of unlimited concurrency.
The topology boundary is the region worker; a later deployment may split
workers only after ownership and cursor handoff are tested.

The calendar uses the GDD's locked 80 real minutes per day. Seasons and years
remain deferred for pacing validation; the current 14-day season and 56-day
year values are development fixtures only. The calendar changes crop growth,
leases, household decisions, migration, prices, and history labels through
data-driven fields. Essential services stay open during seasonal pressure.

Optional generational legacy play is deferred and not selected. Combat defeat
never causes permanent character death or an automatic legacy transition.
