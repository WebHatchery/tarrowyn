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
- Generate new production access and refresh credentials from the operating
  system's secure random source; retain only bounded legacy session records
  during their configured expiry window.
- Add rate limits, request validation, replay protection, abuse controls,
  moderation tools, and audit records for chat, trades, claims, governance,
  and account actions.
- Define privacy and data-retention behaviour for account, chat, payment-free
  gameplay, and settlement-history data before public access.

The unauthenticated development-guest entry point now has a bounded HTTP
admission window of 32 attempts per source over 60 seconds, preventing an
anonymous burst of new client keys from growing the durable fixture without
blocking the supported 24-client local target. Production deployment should
still apply its own proxy-level limits and identity controls.

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
transactional snapshot/index migration are now present. A recorded earlier
release-candidate run of the configured local preview passed
`scripts/verify_mysql.ps1`, including migration/readiness, overlapping
duplicate-request replay, restart persistence, and native dump/restore into a
temporary database. Later reruns first found the preview file using a stale
username key; after correcting it to the documented `DB_USERNAME` contract, a
password-safe connection succeeded but the existing database failed closed at
readiness because legacy production refresh replay results lacked the current
ownership mirror. The reload migration now reconstructs that mirror from
persisted sessions and discards only orphaned replay cache entries, so that
legacy failure is repaired locally. No new MySQL persistence claim is made
from those reruns.
The checklist remains open for a clean current preview snapshot,
target-environment migration, multi-worker concurrency, failover, and rollback
drills; the JSON backend remains the deterministic default for local fixtures.

The operator readiness projection now validates the full durable release
candidate before admitting traffic: world clock, players, crops, trades,
event history, frontier records, Phase 4 civic state, Phase 5 regional state,
and Phase 6 identity, session, audit, moderation, replay, deletion, and backup
metadata. Broken cross-references report degraded readiness rather than being
served as authoritative state. These checks are covered by focused repository
regressions and the recorded cross-subsystem gate; they do not close the
target-environment MySQL, topology, identity-gateway, TLS, scale, or rollback
requirements.

### Deployment and operations

- Select and document the hosting architecture, browser delivery path, TLS,
  configuration and secret management, process supervision, and environment
  separation.
- Provide health/readiness checks, structured logs, metrics, error reporting,
  latency and rejection dashboards, and alerts for persistence, tick drift,
  event backlog, economy anomalies, and client connection failures.
- The local release pipeline now runs formatting, tests, clippy, content and
  asset validation, `publish.ps1`, and archive identity generation; its clean
  Windows/WebGL/server candidates can be preserved and rollback-rehearsed. The
  server package defaults to the current host and accepts an explicit installed
  target; the gate also launches it with isolated JSON state and checks both
  health endpoints. The production OS or container contract remains open.
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

Current measurement note: the isolated harness passes the supported 24-client
regional target with a 250 ms tick, backup, travel arrival, metrics, and restart
recovery. Exploratory 50- and 100-client single-round runs complete their mixed
requests but raise the configured `market_backlog` alert after open orders pass
32; when that one alert is explicitly allowed, both runs complete backup,
arrival, metrics, restart, and recovery checks. The several-hundred-player zone
goal therefore remains an explicit scale and topology gate, not an implied
property of the current one-worker preview. A 250-client boundary run also
completed those checks with the warning allowlisted, but required 82,299.85 ms
for 2,500 requests; that result is capacity evidence against promoting the
current one-worker snapshot bridge to the several-hundred-player direction.

The browser client also caps each movement, chat, farming, trade, and
cross-phase command buffer at 32 pending entries. A saturated buffer leaves
the current action state retryable instead of advertising a request that was
never queued.

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

Chronicle policy is now explicit: the server keeps the newest 64 entries in the
normal regional view and moves older entries into an append-only durable archive.
The normal view receives a bounded summary of archived ticks, entry count, event
kinds, and recent highlights. The authenticated search endpoint scans both the
archive and recent window, so an old achievement remains retrievable even after
it leaves the normal view. Account deletion anonymises matching names in recent,
archived, and event-stream chronicle records; the archive is intentionally not
used as a client-side unbounded display buffer.

The shared and regional event streams each retain at most 2,048 records. Regional
event state records carry their own retention floor, and both the server projection
and client cache reject or trim beyond that boundary so a missed regional cursor
reconnects through the authoritative reload path instead of receiving an incomplete
history.

- Add economy and population monitoring for inflation, item scarcity, NPC
  replacement, abandoned claims, settlement decline, and newcomer access.
- Make an explicit decision on optional legacy or generational play. If it is
  selected, implement it as a bounded, opt-in continuity system; it must not
  become the normal consequence of combat defeat.

### Player-facing readiness

- Provide a clear first-session path using visible touch targets, including
  account creation/linking, reconnect, recovery, moderation/reporting, and
  safe logout.
- The client now polls readiness alongside its normal state refresh and shows
  the configured maintenance message from connection startup onward, or a
  tap-to-reconnect fallback when the server reports degraded readiness.
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
| `GET /v1/ops/health` / `GET /v1/ops/metrics` | Expose a safe public readiness projection for client maintenance handling; keep detailed operational metrics behind authenticated operator access. |
| `GET /v1/chronicle/search` | Search or summarise long-lived regional history with access controls. |

The client must handle session expiry, maintenance, deployment mismatch,
rate-limiting, moderation responses, restore- or retention-era cursor invalidation, and
regional handoff. It must never present a locally cached success as an
authoritative reward. Shared response decoding rejects a protocol-version
mismatch before any endpoint projection can be applied; the client surfaces
that failure through its visible recovery state and `Reconnect` control.
The shared toolkit also preserves structured API error codes across native and
browser transport paths. When `/v1/events` reports `cursor_ahead` after a
restore or `cursor_stale` after the retained window is crossed, the client keeps
the connection open, clears cursor-derived players,
chat, feed, chronicle, frontier, and reward projections, cancels stale state
and chronicle requests, and immediately reloads `/v1/state` plus history from
cursor zero. This prevents a restored server from being presented with cached
history or a stale cursor. The embedded regional client follows the same
boundary for `/v1/events/region`: it advances from the returned cursor, merges
changed event stages by stable ID, and clears/restarts its regional cache when
the server reports `cursor_ahead` or `cursor_stale`.

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

The current target-environment gates and deliberate product deferrals are
tracked with evidence and exit conditions in
[`PHASE_6_FOLLOW_UP_REGISTER.md`](PHASE_6_FOLLOW_UP_REGISTER.md).
