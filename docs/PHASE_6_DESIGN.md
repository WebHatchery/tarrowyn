# Phase 6 decision record — the lasting realm

## Identity and privacy

Production identity is delegated to the WebHatchery Identity Gateway through
the `webhatchery-identity-oidc` provider contract. The server stores a trimmed,
bounded provider subject mapping with no control characters, never a provider
credential. A guest fixture may be
linked to that subject once; the character ID remains the durable character
boundary. Production access tokens are opaque, short-lived, generated from the
operating system's secure random source, rotated on refresh, and revocable.
Refresh tokens are stored only as server-side session records apart from the
authenticated link/refresh response that hands the current credential to the
client.
The unauthenticated development-guest endpoint admits at most 32 attempts per
source in a 60-second HTTP window, with a bounded source table; deployment
proxies must add their own rate and identity controls for public traffic.
Linking a guest migrates its persisted account references across trades, claims,
orders, frontier parties, market records, retained history, audits, and
account- or identity-keyed replay payloads before the production session is
issued, so pre-link play remains attached to the same character boundary
without breaking request idempotency.
Logout revokes one or all sessions. Credential recovery and MFA remain the
identity gateway's responsibility; the game server receives only a verified
subject.

Account identity is retained until deletion. Chat reports are retained for 90
days, while settlement history is retained as public world history with
account identifiers minimised. Payment-free gameplay has no payment data. An
authenticated deletion request is durably queued and executed on the next
authoritative tick; it removes the provider mapping, sessions, character-private
state, and private support records while preserving anonymised public history
where required for world continuity. The online client exposes this boundary
through a visible Delete control; it only arms and sends the bounded request
after the account projection identifies a linked production account. The first
tap arms the action and the second tap sends it. A scheduled deletion clears
the client session and returns the player to the visible Reconnect path. The
deletion worker also removes persisted Phase 4 account-keyed and Phase 5
identity-keyed replay payloads, so private command responses do not survive
the private-state boundary.

All mutation endpoints validate bounded request IDs and 64 KiB JSON request
bodies, use server authorization, and retain idempotent results where retries
can happen. Durable identity and player-provided labels such as account-deletion
account IDs, linked display names, expedition outpost names, and public proposal
targets are trimmed,
bounded, and rejected when they contain control characters; chat text and
channel names follow the same stored-text boundary. Canonical contract,
expedition, claim, knowledge, profession, governance, trade, and lesson
selectors, regional route, travel, market, and event selectors, event
intervention text, plus moderation target IDs and notes,
support account and repair selector IDs, and operator notes, use the same
bounded control-free audit boundary. Refresh tokens use a separate bounded
secret boundary and are never copied into support/player projections or audit
records; the link and refresh responses are the deliberate authenticated
credential handoff. Chat
also has length and per-tick limits, and chronicle search accepts only bounded
control-free queries while preserving an empty query for browsing;
chat metadata, trades, claims, governance, moderation, and support repairs are
recorded in the audit stream without copying private chat text into audit
notes. API responses carry `Cache-Control: no-store` so access and refresh
responses are not retained by intermediaries. The no-PvP law boundary is still
active.

## Persistence, backups, and repair

Storage version 20 adds persisted chat, movement, auth-link, auth-refresh,
auth-revoke, moderation-report replay results, and the account moderation cooldown
alongside the persisted Bellweather animal condition and daily care state,
the reposition opening and wind spark,
field-tool condition, and real-time lease timestamps
and public tax receipts,
the regional state,
production session/audit records, and queued account-deletion work, and
the per-character skill ledger while retaining defaults for Phase 1–6 files.
Local combat additionally persists its next server-tick action window, with
serde defaults keeping older combat records available after upgrade.
Storm Magic discovery uses the persisted root-practice and qualifying-event
ledger: severe-weather three-element workings are limited to one spell per
encounter, require mastered Wind, Water, and Electricity Magic, and reveal the
server-owned merger after 25 successful interactions. Discovery and usability
remain separate in the skill projection: a learner who receives Storm Magic
through a lesson must still satisfy the personal prerequisite and interaction
requirements before the touch Spell control presents the Storm technique.
Ready discovered advanced arts are also exposed as selectable subjects in the
touch School chooser without revealing hidden recipe details.
Account deletion work is a live queue rather than replay history: repeated
requests for one account coalesce onto its pending operation, pending records
are never evicted by replay-cache maintenance, and admission is capped at 128
accounts until the next authoritative processing tick.
Production access records are retained only while their access token is active
or their refresh token remains valid; revoked and fully expired sessions are
removed during session maintenance without interrupting a valid refresh.
After a successful guest link rotates the guest access token, a bounded replay
tombstone keeps that old token able to recover the exact cached link response
but cannot authorize a new link request. The client retries the same durable
request a limited number of times after a transport timeout or failure, so a
response lost after server commit does not strand the character between guest
and production identity. Automatic production-session refreshes use the same
bounded exact-request retry boundary after transient transport failures, while
an explicit expiry or revocation response still clears the session and returns
the player to visible sign-in recovery. Mutation dispatch and authenticated
projection reads wait for an in-flight refresh to finish across movement,
chat, farming, trade, regional, profession, and frontier surfaces so a token
rotation cannot invalidate a newly sent request or leave a same-frame
projection using the old bearer token.
Those command queues preserve their exact request ID and retry a transport failure
within the same bounded limit, allowing the server's durable replay result to
confirm a committed action.
Refresh replay results retain their account ownership separately from the live
session table so deletion also removes rotated responses after their access
session has expired.
Moderation cooldowns are identity-lifecycle state rather than replay history:
they remain for every extant identity and are removed only when that identity
leaves the world, so report rate limits are not defeated by cache eviction.
Every mutation replay cache is trimmed to a 512-entry per-scope bound on the
authoritative world tick, including identity, regional, support, authentication,
moderation, and earlier-phase command results.
The production audit stream likewise keeps its newest 512 records, dropping
only the oldest entries when its bounded operational window is full.
The land registry keeps at most 128 claim records; reclaimed history makes
room for new requests, while live claim rows are never evicted or displaced.
Moderation reports retain their creation time, expire after the documented 90
real days, and are additionally capped at the newest 512 records for bounded
recovery state.
The shared event stream is also bounded at 2,048 records. Requests whose cursor
predates the first retained record fail with structured `cursor_stale` rather
than returning a successful but incomplete stream; the client clears its
cursor-derived projections and reloads authoritative state and history.
The repository now has two selectable backends: JSON with atomic temporary-file
replacement for deterministic fixtures. A configured JSON snapshot that cannot
be read or parsed fails closed without being replaced by a fresh world. MySQL
uses the checked-in
`0001_initial_world.sql` migration. The MySQL bridge stores the versioned
authoritative snapshot and a transactional account/character index; on load it
also compares the denormalized storage version, world tick, and event cursor
with the JSON document and compares every indexed account/character pair with
the snapshot identities, failing closed on any mismatch. It keeps the existing
protocol and repository rules intact while the schema is being proven against a
live environment. The bridge uses a bounded driver pool, reserving one
connection for the process-lifetime world-authority lock and defaulting the
pool maximum to four; `TARROWYN_MYSQL_POOL_MAX_CONNECTIONS` allows a measured
2–32 deployment override. Pool checkout also has a five-second bound so a
depleted or disconnected backend returns a persistence error rather than
stalling an authoritative operation indefinitely. Startup also rejects a
migration table written by a
newer server binary rather than attempting to run an older schema against it.
The server writes a scheduled backup to the configured backup path and reports
the last successful tick through `/v1/ops/health`. A failed authoritative write
or scheduled backup degrades operator readiness and adds a safe persistence or
backup alert; raw storage errors remain in server logs rather than crossing the
API boundary. A failed mutation is rejected with `503 persistence_unavailable`
and the repository restores its last successfully persisted state, so a client
cannot receive an accepted response for a mutation that exists only in memory.
The client treats this specific structured failure as retryable and resubmits
the same request ID within its bounded command retry window.
The tick persists its authoritative state before writing a scheduled backup and
persists the successful backup marker afterward. A later successful backup
clears the backup failure state. Restore drills validate the backup as JSON
before serving it as a named state path; a restore is never an in-place
destructive command. Both JSON and MySQL refuse a snapshot from a newer server
version, preventing an older rollback binary from overwriting unknown durable
fields.

The remaining persistence gate is deliberately explicit: run the MySQL
migration and restart/duplicate-request/partial-write tests against the target
database, then prove database backup and restore rather than relying only on
the JSON snapshot companion. MySQL schema startup uses a bounded advisory lock
so concurrent workers cannot race on the migration record. The current
in-process repository still targets one authoritative worker. A MySQL worker now
holds a separate process-lifetime world-authority advisory lock and a second
worker sharing that database fails startup after a bounded wait, rather than
silently overwriting a newer snapshot. Multi-worker locking with relational
decomposition remains follow-up work.

Repair ownership is explicit. The world authority owns travel, inventory,
market orders, claims, households, and moderation state. The support surface
fails closed unless the authenticated account appears in the deployment's
`TARROWYN_SUPPORT_OPERATOR_ACCOUNTS` allowlist, then accepts repeatable, audited operations for stuck travel, inventory
normalisation, trade reconciliation, active-claim access restoration, duplicate
regional-household merging, and moderation resolution. Claim restoration never
extends a lease or changes ownership. Household merging retains distinct
history entries and rejects conflicting identity or same-tick status records.
Trade reconciliation is owned by the market authority: it accepts only open or
failed orders, restores the original origin escrow to the owner or regional
stock, then closes the order so a second request cannot refund it twice.
Stuck-travel repair is also owned by the regional authority: it accepts only a
recorded active or interrupted journey, returns the character to that journey's
recorded origin, preserves cargo and rewards, and records the regional repair.
Inventory normalisation clamps every persisted item counter, including combat
bandages, to the documented support ceiling without creating new goods.
Unsupported repairs return a clear reason rather than guessing at state. Every
repair carries an operator note and audit ID.
The allowlisted `GET /v1/support/account?account_id=...` view exposes the
target's account and character projection, claims, trades, retained chronicle,
and current event cursor without returning access tokens, refresh tokens,
provider subjects, or other secrets.

Chronicle retention has two server-owned windows. The latest 64 entries remain
in the ordinary settlement view; entries leaving that window move to a durable
append-only archive. Settlement chronicle responses include a bounded archive
summary, while authenticated chronicle search scans the recent window and the
archive. This keeps the client display small without deleting old achievements.
The account-deletion worker anonymises chronicle text in both windows and in
the event stream before private identity state is removed. Before that removal,
any open or failed regional market order owned by the account is cancelled.
Real unsettled escrow is returned to origin stock; a travelling fallback order
has no player escrow and is closed without a refund. The anonymised order
remains as public settlement history without an ownerless shipment.
When a provider leaves while holding an accepted service order, the surviving
requester's typed material and tool escrow is returned before that order is
cancelled; an order owned by the departing requester is instead removed with
the requester's private state and is not credited to another player.

The checked-in content contract is validated twice at release boundaries: the
PowerShell gate requires the canonical manifest set, exact schema declaration,
record IDs, required text fields, and required arrays to be present and valid
JSON, while server startup parses typed action, crop, item, event, settlement,
region, household, infrastructure, NPC-household, recipe, calendar, and game-
config records. Startup rejects duplicate IDs, incomplete records, unknown
route or settlement locations, missing launch IDs, incompatible launch route
and settlement links, invalid event stages, incomplete multi-location event
interventions, and a day-length mismatch. Skill content also rejects
prerequisite cycles and requires every merger prerequisite to have a lower
declared depth, preserving the layered discovery model as new content is added.
It also requires the GDD launch root catalogue and the Weapon Fighting and Storm
Magic discoveries to remain present in every release manifest. The validator
protects future content additions; the authenticated skills read also rechecks
stored prerequisite practice and qualifying history, persisting a newly
eligible discovery after a content release without requiring repeated play.

The crop manifest used by server planting follows that validated order.
Regional event seeding consumes the validated event narrative and affected
location scope by manifest order, while settlement projections consume their
manifest condition, opportunity, and supply profiles. Market base prices also
consume the item manifest, which now covers every traded commodity. Regional
season labels follow the calendar sequence and season length; the configured
real-time day length still belongs to the server deployment boundary. Route
transport, endpoint topology, location roles, resources, names, services,
positions, route tuning, and shared farm-plot positions follow the validated
region manifest, while fresh regional location, route, settlement, and stock
collections consume the same validated catalog IDs. The regional snapshot also
emits its identity from that region manifest rather than a duplicate endpoint
constant.

The launch field-tool repair order consumes a typed recipe manifest for its
material, tool, reward, and benefit values. The repeatable Brambleback contract
consumes a typed contract manifest for its narrative, target, required
progress, and reward curve. The launch wilderness threat consumes a typed
threat manifest for its identity, monster, position, health, risk modifier,
resource demand, and rumour. The Maren opportunity and regional household
projections share a typed household manifest for members, movement, service,
reasons, and history. Infrastructure projections consume typed infrastructure
content for public works, positions, maintenance, quality, and recovery notes.
The fixed Bellweather household consumes typed NPC-household content; its
condition-reactive service lifecycle remains authoritative code, while general
NPC family simulation remains deferred. Each live simulation consumer has a
fixture and compatibility test; future content additions must preserve the
same boundary.

## Hosting and observability

The release candidate runs one region-authoritative worker with a bounded HTTP
request pool behind a TLS reverse proxy, with the browser bundle delivered by
the existing publishing path. The native worker binds only to its configured
private service address; TLS,
provider secrets, and support credentials are environment or deployment-secret
inputs, never checked into the repository. Development guest identities use a
separate fixture state path and must not share production data.
Guest-session admission is limited to 32 attempts per source by default and is
configurable through `TARROWYN_GUEST_SESSION_BURST_LIMIT`; capacity probes may
raise that bound explicitly for their controlled client bootstrap, but the
setting does not change the one-worker ownership boundary. HTTP worker count is
automatic when `TARROWYN_HTTP_REQUEST_WORKERS=0` and remains clamped to 4–32
when overridden; `TARROWYN_HTTP_QUEUE_CAPACITY` defaults to 128 and remains
clamped to 16–4096. These deployment controls are visible in operator metrics.

Authentication also fails closed when a restored session points at a missing
character record: the dangling session is evicted and the request receives the
normal unauthorized response instead of bringing down the world worker.

The world ticker schedules against monotonic deadlines rather than sleeping a
full interval after each update. Normal persistence and request work therefore
does not stretch the GDD's 80-minute day; an overrun advances to the next
recoverable deadline without running an unbounded burst of catch-up ticks.
Authoritative world ticks and calendar days saturate at their numeric ceilings,
and restored clock seconds are normalized into the configured day window before
the worker resumes ticking.

`/health` remains a simple process check. `/v1/ops/health` is a readiness and
integrity check. Authenticated `/v1/ops/metrics` reports sessions, accounts,
regional visibility, event backlog, open orders, travel recovery load, command
rejections, and measured tick latency through an exponentially weighted average,
the latest duration, and a drift count. It also reports the bounded HTTP worker
count, queue capacity, active requests, current and peak queue depth, and
queue-full events so target capacity checks can observe transport pressure.
For a MySQL worker it also reports the configured maximum database pool size;
JSON fixture workers report zero for that field.
Alerts are raised for persistence write
failures, integrity failures, market backlog, interrupted-travel backlog, tick
drift, regional event backlog, and economy invariants that no longer reconcile.
The same operator projection reports average regional price pressure, distinct
scarce goods, active NPC fallback households, abandoned or expired claims,
declining settlements, open market fallback orders, and whether a new player
still has an open access path.
Client connection failures remain a deployment/client telemetry concern because a
disconnected client cannot report through this worker. Slow clients receive
bounded, cursorable projections and do not own the world tick.

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
data-driven fields. The accelerated long-session fixture crosses all four
development seasons, keeps real-time leases independent from world-day rollover,
and checks household, tax, market, chronicle, and newcomer continuity. Essential
services stay open during seasonal pressure.

Optional generational legacy play is deferred and not selected. Combat defeat
never causes permanent character death or an automatic legacy transition.
