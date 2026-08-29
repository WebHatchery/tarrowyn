# Phase 6 release-candidate test report

## Design and persistence gate

The workspace has a versioned storage document, atomic replacement, scheduled
backup metadata, integrity readiness, operator persistence- and backup-failure readiness,
production session records, audit records, and a support repair API. Storage
version 20 also persists chat, movement, and auth replay results,
alongside field-tool condition,
real-time lease timestamps, and public tax receipts,
the per-character skill ledger, Bellweather animal condition, and daily care
state, persists the account moderation cooldown and queued account-deletion work, loads older documents through serde defaults, and fails closed on a
corrupt or newer-than-server JSON snapshot. A Phase 1–4
document without Phase 5/6 fields loads through serde defaults and receives the
current regional and operations state.
Mutation replay caches are bounded to 512 entries per scope during world ticks,
covering identity and phase command results so long sessions do not grow them
without limit.
The release validator also parses the canonical action, crop, item, event,
settlement, region, household, infrastructure, calendar, and game-config
manifests with required record shapes, exact schema membership, and duplicate
ID checks before the Rust build, including supported action kinds, positive market base prices,
calendar compatibility, and required launch IDs and links across every
runtime-authoritative catalog; typed server
cross-reference checks then reject incompatible records at startup, including
missing infrastructure anchors.
The launch-default regression confirms the server's world dimensions, day
length, starting gold, and starting seeds follow the shared game-config
manifest; the guest identity and offline fixture checks cover the same initial
seed supply at their respective authority boundaries.
The action-content regressions confirm authored action IDs and kinds stay
aligned with the four protocol actions the client can execute, rejecting an
unmapped kind, missing launch ID, or mismatched executable kind before startup.
The event-content regression confirms every authored intervention choice maps
to a server effect, rejecting a visible choice that would otherwise resolve as
a silent generic response.
The event-scope content regression confirms each intervention includes the
location required by its concrete route, supply, or storehouse effect.
The localized-event regression confirms escalation, intervention supply, and
resolution safety/price consequences stay inside the event's affected-location
scope rather than mutating every settlement.
The Phase 5 event-catalogue regression confirms the stable `river-thaw` launch
event remains present for the regional event fixture.
The skill-catalogue regression confirms prerequisite graphs cannot contain a
cycle that would make an advanced discovery permanently unreachable.
The server crop-rotation regression confirms planting follows the validated
crop manifest rather than a separate hard-coded order.
The server event-template regression confirms regional event seeding follows
the validated event manifest, including its narrative, effects, and
intervention options, and affected locations.
The settlement profile regression confirms condition, milestones, vacancies,
demand, prices, and abundant/scarce goods in the authoritative projections
follow the validated settlement manifest.
The settlement-topology regression confirms each settlement owns a unique
regional location, preventing ambiguous projection and activity rollups.
The fresh regional-stock regression confirms each launch settlement seeds its
market ledger from the settlement manifest's validated initial-stock records,
so launch quantities are not duplicated in repository code.
The market price regression confirms every traded commodity's base price
comes from the validated item manifest.
The calendar regression confirms regional season labels follow the validated
region calendar at season and year boundaries.
The region-identity regression confirms the regional snapshot emits its ID
from the validated region manifest rather than a separate endpoint constant.
The route-profile regression confirms authoritative route transport, endpoint
topology, timing, risk, capacity, and status follow the validated region
manifest.
The route-topology regression confirms authored routes connect distinct
locations before travel projections are built.
The location-profile regression confirms authoritative names, kinds, positions,
roles, resources, services, and condition follow the validated region manifest.
The regional-bootstrap regression confirms fresh location, route, settlement,
and initial-stock collections are assembled from validated catalog IDs rather
than a second hard-coded launch list.
The fresh-world farming regression confirms the shared plot projection and
field tiles initialize from the same validated region plot positions rather
than a separate legacy grid.
The authored-animal regression confirms Bellweather's position follows the
validated region farm position and that the retired saved-world position
upgrades without changing animal condition or care history.
The content gate also rejects an authored animal position that leaves the
world, overlaps a plot, or loses its one-tile relationship to the fields.
It also rejects farm plots and named locations outside the configured world
before a release package is built.
Infrastructure positions receive the same bounds check before their map
projections are constructed.
The contract-template regression confirms the repeatable Brambleback watch
uses the validated contract manifest for its target and progression contract.
The threat-template regression confirms the launch wilderness threat follows
the validated threat manifest for its monster, position, health, and risk.
The content gate and focused threat regression also reject an authored threat
position outside the configured world before the wilderness projection loads.
The household-template regression confirms the opportunity and regional
household projections share the validated household manifest for identity,
members, movement, service, and history.
The household-content regression also requires the opportunity and regional
projection identities to remain unique as authored household content expands.
The infrastructure-profile regression confirms public-work projections follow
the validated infrastructure manifest for identity, kind, position, maintenance,
quality, and recovery notes.
The recipe regression and order fixture confirm the field-tool repair service
reads its materials, tool cost, reward, and benefit from the validated recipe
manifest. The order fixture also confirms the server rejects a timing score
outside the visible 0–100 interaction range before completing the order.
The fixed-NPC household regression confirms the Bellweather service household
follows validated NPC-household content without exposing a general family
simulation contract.
The same content boundary requires emitted NPC household projection identities
to remain unique as authored households expand.
The focused operations regression counts active travelling fallback market
orders separately from the general open-order backlog for support monitoring.
The readiness regressions also reject malformed bounded route and settlement
projections instead of reporting an operationally healthy regional state.
They also reject an empty regional collection rather than allowing the
integrity check to succeed vacuously.
Regional topology regressions also reject unknown route endpoints and duplicate
settlement locations before readiness can report a healthy world.
The local-combat regression confirms accepted actions publish a persisted
server-tick action window, reject same-tick bursts, and become available again
after the authoritative tick advances. Older local-combat snapshots default
the new window boundary to tick zero.
The lease presentation regression confirms the client summary exposes a
human-readable real-time countdown and uses hours near expiry rather than
leaking an opaque Unix timestamp.
The knowledge-visibility regression confirms undiscovered methods are redacted
for unrelated players, become readable for a taught learner, and become public
only after the discoverer records them in the guild archive.
The settlement-activity regression confirms activity is scoped to the touched
settlement, decays after the last session expires, and exposes a low-activity
strained condition without deleting the settlement projection.
The settlement-condition regression confirms route safety, public-work
condition, local industry, and governance signals evolve toward their bounded
regional targets as support changes.
The regional-summary regression confirms the touch projection places the
current settlement condition, recovery-open signal, and compact facility counts
beside travel status.
It also confirms the shared-road sidebar keeps the wider comparison visible by
showing each settlement's condition and open-vacancy count alongside the local
line.
The regional telemetry regression also confirms the sidebar projection exposes
authoritative road availability/risk, open market orders, active fallback
shipments, and the protected-law boundary plus the latest regional event stage
instead of leaving those loaded records hidden behind the controls.
The market-expiry regression confirms failed-fulfilment history reaches the
recorded order endpoints rather than being attributed to Hearth by default.
The regional map presentation also uses the loaded server location positions and
route statuses for its online overlay; the offline fixture remains explicitly
local rather than pretending to be a shared map.
The focused client control regression also confirms the visible Repair action
queues the existing authoritative route-repair request.
The regional summary regression also confirms the client projects the current
regional household service status instead of requiring a separate hidden
endpoint inspection.
The calendar presentation regression keeps the server-projected development
season available to the online header beside the calendar day without locking
the deferred season/year pacing decision.
The settlement-facilities regression confirms the regional projection maps
claims, free plots, and public works to their nearest settlement while leaving
the Phase 4 registry and infrastructure records authoritative.
The skill-catalogue regressions confirm direct root guidance, non-empty advanced
discovery requirements, and re-evaluation of stored prerequisite history when a
new advanced merger becomes eligible.
The chronicle regression confirms the newest 64 entries remain in the normal
settlement view while older entries move to a durable archive, contribute a
bounded summary, and remain discoverable through authenticated full-history
search. Authenticated search returns a bounded 128-entry page with a
continuation cursor instead of expanding with the archive. The account-deletion
regression also checks that named chronicle text
is anonymised in recent entries, archived entries, and the event stream.
The Phase 5 fixture verifies that travel, market, event, household, identity,
refresh, and revocation state survive the authoritative repository boundary.
The client Phase 5 tests verify that a linked account's visible deletion
control requires two taps, a development guest cannot arm deletion, and the
deletion response is decoded as its dedicated command rather than the
ambiguous market response.
The client network tests verify that the deployment maintenance message takes
priority in the visible status line and that degraded readiness without a
message still gives the player a tap-to-reconnect instruction. The health
request is started with the guest connection attempt, so maintenance guidance
can appear before the authenticated state request succeeds.
The shared protocol tests also reject a response whose protocol version does
not match the client before endpoint data can be projected.
The selected MySQL bridge now has a checked-in migration, startup pool/migration
failure handling, transactional snapshot/index writes, and driver-selection
tests. The recorded release-candidate run of the configured local preview
MySQL service passed `scripts/verify_mysql.ps1`: storage version 20 readiness
and native restore verification,
authoritative animal state, duplicate chat/movement/auth/moderation replay,
concurrent duplicate chat replay, temporary backup creation, and
identity/state persistence across a server restart all succeeded. It then
restored a native `mysqldump` into a generated temporary database and verified
the current world row and identity index before cleanup. The script uses a
unique guest key and does not reset or delete the configured database. A later
2026-08-29 rerun stopped at the fail-fast prerequisite probe because the
configured `dev@localhost` credentials were rejected (`ERROR 1045`); that run
claimed no persistence coverage, and the follow-up register records the
environment blocker.

The target environment still owns the remaining migration, multi-worker
concurrent-write, database failover, and rollback gates. The local script
exercises the JSON backup companion, native dump/restore, overlapping retries,
and the single-worker MySQL bridge, not production topology or database
failover. The bridge now enforces that single-worker boundary with a bounded
process-lifetime MySQL world-authority advisory lock; a second worker fails
startup instead of being allowed to overwrite an in-memory snapshot.

## Security gate

Representative checks cover unsupported identity providers, bounded provider
subjects and linked display names, bounded guest client keys, proposal targets,
expedition outpost names, account-deletion account IDs, moderation targets and
notes, support account and repair selectors, request-ID and 64 KiB request-body
validation, bounded Phase 3, Phase 4, and Phase 5 selectors and event
interventions, bounded refresh tokens, unknown knowledge and claim selectors,
bounded chronicle queries, idempotent
regional mutations, expired/revoked
access, refresh rotation, lost-response account-link replay, limited client
command and refresh retry across movement, chat, farming, trade, regional,
profession, and frontier paths with command ordering, chat limits, and the
protected no-PvP law response. Chat metadata,
direct trades, claims, governance, moderation reports, and support repairs are
audit-linked without copying chat text into the audit stream. Moderation reports are queued and audit-linked. The
support account view is operator-only, returns the requested character-facing
records and cursor, keeps its latest chronicle window to 128 entries, and
excludes session tokens and provider subjects. Support
repair fixtures now prove an active claim's access flag can be restored without
extending its lease, and duplicate regional household records can be merged
while retaining history; both operations remain replay-safe and audited.
The failed-shipment repair fixture also proves that `ReconcileTrade` restores
the owner's escrow, closes the failed order, records the regional repair, and
returns the same response on a replay without paying the escrow twice.
The stuck-travel repair fixture proves that `ClearStuckTravel` uses the
journey's recorded origin instead of a hard-coded settlement, preserves the
journey's cargo/reward boundary, records the repair, and rejects an already
cleared journey.
The account-deletion fixture proves that open and failed regional market
orders are anonymised, cancelled, and returned to origin stock before the
owner's private identity is removed.
The inventory repair fixture covers the persisted bandage counter alongside the
crop and seed counters, confirming the support ceiling applies to every item
field and remains replay-safe.
The provider secret and TLS termination remain deployment concerns and are not
stored in the repository.

After the Storm Magic milestone release gate, maintenance changes have used
focused checks for only the changed subsystem: lifecycle chronicle transitions,
regional history locality, persisted input boundaries, and numeric boundaries
each run their targeted repository or protocol test plus formatting and diff
checks. The numeric checks cover trade receipt, farming counters, skill-merger
qualification, travel progress, governance upkeep, and settlement scarcity
projection; the regional price-note projection also saturates malformed
manifest values, and oversized unsigned environment values fall back instead of
wrapping. The latest clock-boundary checks also confirm trade expiry, world tick,
calendar day, and restored clock seconds remain safe at their numeric limits.
The shared event cursor now also saturates at its numeric ceiling instead of
wrapping or panicking; a focused event-stream regression covers that boundary.
Chat message and tavern notice identifiers follow the same saturation policy,
with focused boundary regressions for both durable counters.
Guest identity and development-session identifiers now also saturate at their
numeric ceiling, with a focused session-allocation boundary regression.
Direct trade identifiers follow the same saturation policy, with a focused
trade-ledger boundary regression.
Land-lease identifiers now follow the same saturation policy, with a focused
claim-ledger boundary regression.
Public proposal, governance decision, and tax receipt identifiers now follow
the same saturation policy, with focused town-hall boundary regressions.
Professional service-order identifiers now follow the same saturation policy,
with a focused service-board boundary regression.
The online client request-ID allocator also saturates at its numeric ceiling,
with a focused network boundary regression.
The offline recovery fixture also saturates its local progression counters,
with a focused state boundary regression.
The shared inventory projection also saturates malformed total quantities, with
a focused protocol boundary regression.
The offline inventory total follows the same saturation policy when saved crop
counts are already at their ceiling.
The offline clock also catches up a huge finite delta in constant time, with a
focused long-session boundary regression instead of an unbounded day loop.
Regional household movement history also keeps its latest 64 entries on runtime
updates and snapshot load. On 2026-08-30, the long-run boundary reliability
milestone passed the full release gate: 14 protocol tests, 242 server tests, 92
client tests, asset and code-standard checks, workspace clippy, Windows and
WebGL release builds, and Preview deployment. The next full release gate
remains reserved for a new major milestone or a change that crosses subsystem
boundaries.
The Phase 4 readiness milestone now validates governance, infrastructure,
leases, households, service orders, school lessons, knowledge, keyed private
state, animal records, and available-plot identity/reference boundaries before
`/v1/ops/health` reports a healthy world. Five focused readiness regressions
cover malformed Phase 4 governance links, duplicate lease IDs, dangling
account references, dangling identity-keyed state, and out-of-range household
values. Its cross-subsystem full release gate passed on 2026-08-30 with 14
protocol tests, 247 server tests, 93 client tests, asset and code-standard
checks, workspace clippy, Windows and WebGL release builds, and Preview
deployment.

The accumulated maintenance batch published successfully through
`publish.ps1` on 2026-08-29: Windows and WebGL packages built and deployed to
Preview. The publisher reported a non-blocking Project Roost tracking HTTP 500
and the existing `net2` future-incompatibility warning; neither prevented the
deployment artifact from being produced.

The subsequent focused client checks cover the visible recovery-safe Retreat
path for both local and frontier combat, the regional inspection's event cause,
exact touch-selectable intervention choices, and its pending/resolved outcome
state. The selected-root practice check also confirms that the touch chooser
queues the named root rather than silently selecting the first catalogue entry,
while advanced or mastered skills remain out of that direct-practice list.
The focused market regression also confirms that an essential shipment can use
the bounded travelling fallback, respects its delay and daily capacity, and
does not refund goods on cancellation. These maintenance changes also passed `publish.ps1` with Windows and WebGL
Preview deployment; no full release gate was rerun because they stayed within
the established client/UI boundary.

## Load and failure gate

The accepted regional target is 24 connected clients, 50 open orders, and a
250 ms tick. The repository's bounded projections and event cursors avoid
broadcasting every regional entity to every client. The release scripts
exercise concurrent fixture requests, backup parsing, persistence- and
backup-failure readiness, restore-on-a-copy, measured tick telemetry, and
operational alert boundaries for tick drift, regional backlog, and economy
invariants. Operator metrics also expose price pressure, scarce goods, NPC
fallback, abandoned claims, settlement decline, and newcomer access. Node-failure and clock-restart behavior are reconciled by
the durable travel/order/event cursors; a duplicate request returns its cached
result instead of paying twice.

On 2026-08-29, `scripts/phase6_load_test.ps1` passed its isolated regional drill
with 24 clients and three rounds: 624 HTTP requests completed in 5,390.96 ms of
mixed-load wall time, with 110 accepted and 154 rejected command outcomes. The
run exercised state, events, movement, chat, markets, travel, the autonomous
tick, scheduled backup, operator metrics, server-owned arrival, and restart
recovery. The result is evidence for the bounded 24-client regional target, not
for multi-worker production concurrency or several-hundred-player capacity.
Each round also includes a deliberate invalid movement probe so the rejection
metric is deterministic rather than dependent on timing contention. Exploratory
50- and 100-client single-round runs completed the full backup, arrival, metrics,
restart, and recovery path when explicitly allowing the expected `market_backlog`
warning: 500 requests in 8,689.34 ms at 50 clients, and 1,000 requests in
16,715.97 ms at 100 clients. Both crossed the current 32-open-order alert
boundary; they demonstrate functional recovery under a monitored warning, not a
supported several-hundred-player capacity.
An additional 250-client single-round boundary probe also completed the same
checks with the warning allowlisted: 2,500 requests in 82,299.85 ms, with 1,000
accepted and 250 deterministic rejections. The long wall time is evidence that
the current one-worker snapshot bridge has not met the GDD's several-hundred
concurrent-player direction; the supported release target remains 24 clients.
The latest 24-client baseline also recorded 67.06 MB of server working set
after load and 2,837.56 ms from worker stop through restart readiness. These
are repeatable local evidence fields, not production memory or recovery SLOs;
the target deployment must establish those limits on its own hardware.
The standalone `scripts/phase6_failure_drill.ps1` also passed on the same date:
it loaded the generated JSON backup into an isolated temporary server, confirmed
readiness and a fresh backup, ran the regional Phase 5 tests, and left the active
state untouched.
The live Phase 6 load journey also reads the allowlisted support-account view,
checks its character and event-cursor boundary, asserts that access and refresh
credentials are absent, and confirms an ordinary player receives HTTP 403.
Before the load begins, it also verifies that both shared and regional event
endpoints return the structured HTTP 409 `cursor_ahead` boundary for a cursor
ahead of the live world.
The same drill then uses a temporary 1 ms server cadence to cross the 2,048
record retention window and verifies structured HTTP 409 `cursor_stale` responses
from both event endpoints before returning to the normal 250 ms cadence.
It also verifies the economy and population monitoring fields in the operator
metrics response and confirms those metrics remain operator-only. The harness
records server working set and stop/start-to-readiness recovery alongside the
mixed-load wall time; the latest 24-client baseline measured 67.06 MB and
2,837.56 ms.

The shared client recovery tests also cover the restore-era `cursor_ahead` and
retention-era `cursor_stale` boundaries. Structured API errors remain identifiable
through the shared native and browser HTTP paths; the client clears stale cursor-derived projections and
schedules a fresh authoritative state/history load without dropping to the
generic disconnected state. Regional event tests also pin cursor advancement,
stable-ID stage merging, and cache reset after a restore.

## Long-session gate and remaining risks

The fixed 80-minute day is recorded in the GDD and Phase 5/6 decision records;
the 14-day season and 56-day year remain explicitly labelled development
fixtures pending pacing validation. Settlement condition, route maintenance,
market sinks, household movement, decline recovery, and chronicle search are
data-bearing surfaces. The remaining release risk is that the MySQL
implementation is still a single-worker snapshot bridge rather than a
decomposed multi-worker storage service. Production deployment must run the
live migration, database restore, concurrency, and rollback drills before
public access.

The deterministic `long_session_crosses_calendar_and_keeps_world_accessible`
fixture crosses thaw, greenrise, harvest, deepwinter, and the next thaw while
checking a household's regional arrival history, a 90-real-day lease under the
accelerated calendar, market expiry, public tax collection and upkeep, regional
event resolution and chronicle retention, and newcomer seeds, locations, and
vacancies. It proves fixture continuity and recovery boundaries without treating
the deferred season/year pacing as a final product decision.

The readiness integrity check also rejects restored market orders that point to
unknown routes or locations, and restored journeys whose route reference or
endpoints no longer resolve. Focused repository regressions cover both failure
boundaries; this maintenance slice used the targeted server test and did not
rerun the full release gate.

The same check now rejects regional event records with empty or unknown
affected-location IDs and household records with unknown movement endpoints.
Focused regressions cover those two restore boundaries as well.

Regional stock keys are also checked against known location and item IDs, with
a focused regression for a stock entry that names a missing location.

Identity readiness now checks both non-empty account IDs and character IDs for
uniqueness, matching the MySQL account-index key boundary; a focused regression
covers duplicate account IDs before persistence can fail.

Persistent-world readiness now validates the stored clock, player positions,
plots and crops, direct trades, event and history cursors, frontier threat,
contracts, claims, expeditions, credentials, households, chronicle entries, and
outpost state before operational readiness is reported. Five focused regressions
cover malformed player position, saved crop stage, frontier threat health, trade
map identity, and event cursor state. The cross-subsystem release gate passed on
2026-08-30: 14 protocol tests, 252 server tests, and 93 client tests, followed
by asset and code-standard checks, clippy, Windows and WebGL release builds,
packaging, Preview deployment, and catalog synchronization.

Phase 6 persistence readiness now also rejects broken production-account links,
orphaned or unmirrored production sessions, malformed audit outcomes, orphaned
moderation timestamps, invalid replay-cache references, malformed deletion
queue keys, and inconsistent backup metadata. Five focused regressions cover
those Phase 6 boundaries. The scoped validation passed on 2026-08-30 with 26
integrity tests, server-only clippy, and the project publisher's Windows and
WebGL builds, packaging, Preview deployment, and catalog synchronization; the
workspace test gate remains reserved for the next major milestone.

The Phase 4 readiness boundary now also requires the authored civic foundation
collections—offices, infrastructure, households, knowledge, and animals—to
remain present after restore or repair. One focused regression exercises each
missing collection; the dynamic available-plot list remains allowed to be empty
when all recognised plots are occupied.

Phase 4 land-registry integrity now also rejects claim and available-plot
positions outside the configured world bounds, while retaining dynamic plot
layouts for migration compatibility. Two focused regressions cover malformed
claim and free-plot positions.

Regional market integrity now also requires every persisted order owner to be a
current account or the deletion-safe former-resident marker. One focused
regression covers an order whose owner reference has disappeared.

Regional event integrity now validates lifecycle text, affected-location
uniqueness and references, intervention choices, stage/outcome combinations,
tick ordering, and retained cursors. Two focused regressions cover an invalid
intervention choice and a missing event cursor.

Regional household integrity now validates the fixed household registry,
location references, bounded history, supported lifecycle statuses, and
departure/arrival timelines. Two focused regressions cover an unsupported
status and an impossible considering-state timeline.

Regional travel integrity now validates journey identifiers, endpoint
references, non-zero timelines, bounded interruption notes, supported stored
statuses, and interruption recovery metadata. One focused regression covers a
zero-length persisted journey.

Settlement projection integrity now validates bounded identity and narrative
fields, the runtime population and gauge ranges, and retained local chronicle
ordering/cursors. Two focused regressions cover a missing settlement milestone
and an unassigned local chronicle cursor; the scoped server checks remain the
appropriate validation for this subsystem until the next major milestone.

Regional location and route integrity now validates bounded projection text,
known endpoints, in-world location positions, non-empty service data, route
operational ranges, and action ticks that cannot point into the future. Two
focused regressions cover a malformed location access note and a future route
action timestamp.

Market-order integrity now validates bounded order identity text, the supported
one-to-99 quantity range, creation ticks, and status-consistent settlement
timestamps. Two focused regressions cover an oversized quantity and an open
order carrying a settled timestamp.

Phase 5 replay-cache integrity now validates cache size, bounded keys and
request IDs, live identity ownership, and response/key request agreement. Two
focused regressions cover an orphaned identity key and a mismatched cached
request ID.

Phase 5 sequence metadata now validates positive travel, order, and event ID
counters plus a fallback-day marker that cannot point beyond the world clock.
One focused regression covers a zeroed order sequence.

Phase 4 replay-cache integrity now validates cache size, bounded keys and
request IDs, current account ownership across the skill prefixes, and
response/key request agreement. Two focused regressions cover an orphaned
account key and a mismatched cached request ID.

Phase 3 replay-cache integrity now validates cache size, bounded identity/request
keys, current identity ownership, and response/key request agreement across
contracts, combat, recovery, claims, and expeditions. Two focused regressions
cover an orphaned identity key and a mismatched cached request ID.

Core repository sequence integrity now requires positive guest, message, token,
trade, and notice counters before readiness is reported. One focused regression
covers a zeroed notice sequence.

Core character replay integrity now validates the bounds and key/response
agreement of farming, trade, movement, and chat replay maps. One focused
regression covers a mismatched chat response request ID.

Core identity timing integrity now rejects future last-seen ticks and tax days
relative to the authoritative world clock. One focused regression covers a
future identity activity tick.

Phase 3 state integrity now requires a live chronicle sequence counter and
retains the contract-progress identity cross-reference. Two focused
regressions cover a zeroed chronicle sequence and orphaned contract progress.
The same boundary now rejects future claim activity and a zero reclaim window,
with two claim-focused regressions.
Expedition persistence now bounds the outpost and member display text and
rejects control characters, with two expedition-focused regressions.

The cross-layer persistence-readiness milestone passed the full release gate on
2026-08-30 after the core and Phase 3-6 integrity additions: 14 protocol tests,
282 server tests, and 93 client tests, followed by asset/code-standard checks,
clippy, Windows and WebGL release builds, packaging, Preview deployment, and
catalog synchronization. Future slices return to changed-subsystem tests
unless they cross another major release boundary.
