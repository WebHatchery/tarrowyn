# Phase 6 — The Lasting Realm

## Purpose

Phase 6 prepares Tarrowyn for real accounts, real operators, and a world that
must remain trustworthy over months and years. Phases 1–5 prove the game’s
authority model and regional society with development identities and a small
deployment shape. This phase turns that prototype into a recoverable service
without treating production infrastructure as a substitute for game design.

The phase has two gates. First, the design and operational contracts must be
locked. Second, the complete regional game must pass failure, migration,
security, scale, and long-session tests. New content is subordinate to those
gates.

## Build scope

### Real identity and account safety

- Choose and document the production authentication provider and account
  lifecycle: sign-in, linking, guest upgrade, logout, credential recovery,
  account deletion, and character/account boundaries.
- Replace development bearer-token assumptions with expiring sessions,
  refresh/revocation behaviour, server-side authorization, and secure secret
  handling.
- Add rate limits, request validation, replay protection, abuse controls,
  moderation tools, and audit records for chat, trades, claims, governance,
  and account actions.
- Define privacy and data-retention behaviour for account, chat, payment-free
  gameplay, and settlement-history data before public access.

### Production persistence and recovery

- Move the repository behind the selected MySQL database while preserving
  explicit migrations and protocol compatibility. Preview configuration uses
  the ignored `.env.preview` contract; production credentials come from the
  deployment secret manager.
- Add scheduled backups, point-in-time or equivalent recovery, restore drills,
  integrity checks, and an operator-visible record of the last successful
  backup.
- Test deploy, rollback, schema migration, partial writes, duplicate
  requests, server restart, clock restart, and event-cursor recovery paths.
- Define data ownership and repair tools for stuck travel, corrupted claims,
  invalid inventory, failed trades, household duplication, and moderation
  actions. Repair operations must be audited and safe to repeat.

Current implementation note: the selectable MySQL backend and initial
transactional snapshot/index migration are now present. The configured local
preview passes `scripts/verify_mysql.ps1`, including migration/readiness,
overlapping duplicate-request replay, restart persistence, and native
dump/restore into a temporary database. The checklist remains open for target
environment migration, multi-worker concurrency, failover, and rollback
drills; the JSON backend remains the deterministic default for local fixtures.

### Deployment and operations

- Select and document the hosting architecture, browser delivery path, TLS,
  configuration and secret management, process supervision, and environment
  separation.
- Provide health/readiness checks, structured logs, metrics, error reporting,
  latency and rejection dashboards, and alerts for persistence, tick drift,
  event backlog, economy anomalies, and client connection failures.
- Add a release pipeline that runs formatting, tests, clippy, protocol checks,
  migration checks, asset validation, `publish.ps1`, and a reproducible
  deployment artifact.
- Maintain operator runbooks for launch, rollback, incident response,
  maintenance windows, restore, moderation, and communication of a service
  interruption.

### Scale and topology

- Make the Phase 5 topology decision real: one process with region routing,
  multiple region workers, shards, instances, or another documented model.
- Load-test movement, chat, events, markets, travel, ticks, and persistence
  independently and together. Set measured targets for concurrent clients,
  event latency, command rejection, memory, and recovery time rather than
  carrying forward an unqualified number.
- Add interest management, backpressure, bounded queues, and graceful
  degradation for hot settlements or slow clients.
- Verify that a node or region failure does not create duplicate rewards,
  split-brain ownership, impossible travel, or unrecoverable cursor gaps.

### Long-term world and content operations

- Lock the real-time lengths of days, seasons, and years, including how those
  choices affect crops, leases, households, migration, prices, and history.
- Add a data-driven content pipeline for crops, items, contracts, threats,
  recipes, households, settlements, and events with schema validation and
  compatibility checks.

Current implementation note: the checked-in pipeline now validates and serves
typed crop, item, contract, threat, recipe, household, settlement, infrastructure,
NPC-household, event, region, calendar, and farm-plot content. The remaining
content work is expansion and pacing validation, not a missing launch manifest.
- Define how chronicle history is retained, summarised, archived, searched,
  and displayed after years of events without making old player achievements
  disappear.
- Add economy and population monitoring for inflation, item scarcity, NPC
  replacement, abandoned claims, settlement decline, and newcomer access.
- Make an explicit decision on optional legacy or generational play. If it is
  selected, implement it as a bounded, opt-in continuity system; it must not
  become the normal consequence of combat defeat.

### Player-facing readiness

- Provide a clear first-session path using visible touch targets, including
  account creation/linking, reconnect, recovery, moderation/reporting, and
  safe logout.
- Add in-client status, maintenance, migration, and incident messaging that
  explains what the player can do next.
- Add support-facing views for account identity, character state, settlement
  history, claims, trades, and event cursors without exposing secrets.
- Establish a content and support cadence that preserves low-population
  fallback services while leaving meaningful work for players.

## Server, protocol, and client work

Every production mutation must be authenticated, authorized, idempotent where
retries are possible, cursorable where it changes shared history, and
observable by operators. Development guest sessions may remain as a fixture,
but they must be isolated from production accounts and data.

Suggested production-facing additions include:

| Endpoint or surface | Purpose |
|---|---|
| `POST /v1/auth/link` / `POST /v1/auth/refresh` | Link a development identity and manage an expiring production session. |
| `POST /v1/auth/revoke` | Revoke sessions or credentials after logout, compromise, or support action. |
| `GET /v1/account` | Read the authenticated account and character boundary. |
| `POST /v1/account/delete` | Queue authenticated account deletion and anonymise retained public history. |
| `GET /v1/support/account` | Let an allowlisted operator inspect account state without secrets. |
| `POST /v1/support/repair` | Audited operator repair for explicitly supported stuck-state cases. |
| `GET /v1/ops/health` / `GET /v1/ops/metrics` | Expose readiness and operational signals to the deployment, not to ordinary players. |
| `GET /v1/chronicle/search` | Search or summarise long-lived regional history with access controls. |

The client must handle session expiry, maintenance, deployment mismatch,
rate-limiting, moderation responses, restore-era cursor invalidation, and
regional handoff. It must never present a locally cached success as an
authoritative reward.

## Acceptance test

The phase succeeds only when a release candidate can demonstrate:

1. a new player creates or links a real account, reconnects, logs out, and
   returns without losing the intended character boundary;
2. authorization, revocation, rate limits, replay protection, moderation, and
   support-audit checks pass against representative abuse cases;
3. a database migration upgrades a backup, the upgraded world serves clients,
   and a restore drill returns to a known consistent point;
4. deployment, rollback, node restart, clock restart, and region failure
   preserve or clearly reconcile all durable rewards, claims, trades, travel,
   and chronicle cursors;
5. measured load targets are met for the selected topology while slow or
   disconnected clients degrade without blocking the world tick;
6. a long-session calendar and economy test covers seasons, leases, NPC
   movement, settlement history, sinks, and newcomer access; and
7. browser touch play, publishing, observability, moderation, and operator
   recovery are all exercised from written runbooks.

## Explicitly deferred

Phase 6 does not promise unlimited concurrency, a fully simulated planet,
permanent character death, unrestricted PvP, or every possible profession and
settlement type. Those are product decisions and content expansions that can
follow a stable release. The phase is complete when the chosen world can be
operated honestly and recovered safely.

## Exit artifacts

- Production architecture, authentication, privacy, topology, calendar,
  legacy, and moderation decision records.
- Security, migration, backup/restore, load, failure-injection, and long-run
  test reports.
- A release and rollback pipeline with deployment artifacts and environment
  configuration documentation.
- Operator, support, moderation, and player-facing incident runbooks.
- A final production-readiness review that records remaining risks, measured
  limits, and the content roadmap after launch.
