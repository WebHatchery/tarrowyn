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

Service-order creation now checks the recipe's material and tool escrow before
evicting completed board history to make room. A failed insufficient-materials
request therefore leaves the retained service ledger unchanged; the focused
service-order retention suite covers that rejection beside the existing
room-making and full-board paths.

Claim creation now checks for a recognised free plot before evicting reclaimed
history to make room. A failed no-plot request therefore leaves the land
registry unchanged; the focused claim-retention suite covers that rejection
beside reclaimed-history eviction and full-live-ledger blocking.

Crop tending now rejects a second action against the same plot in one server
beat, preserving the tool condition and slow-growth boundary; the next beat
opens the plot again. The focused farming suite covers same-beat rejection and
post-tick acceptance.

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
Chronicle persistence now rejects entries dated beyond the authoritative tick,
with one chronicle-focused regression.
Fixed household persistence now requires a bounded, nonempty member list with
safe member text, with one household-focused regression.
Phase 4 persistence now requires positive record-sequence counters and a
governance cursor no newer than the shared event cursor, with two focused
metadata regressions.
Governance office and proposal timelines now stay within the authoritative
world tick, with one focused proposal-timestamp regression.
Phase 4 land-right persistence now enforces lifecycle access, ownership, and
timestamp ordering, with two focused claim-state regressions.
Phase 4 infrastructure persistence now checks world position, condition/status
agreement, and maintenance timing, with three focused regressions.
Phase 4 household persistence now bounds member records and decision timing,
with two focused household regressions.
Phase 4 service-order persistence now enforces creation/completion timing and
status-compatible completion metadata, with two focused order regressions.
Phase 4 knowledge persistence now bounds item text and rejects duplicate
discoverer entries, with two focused knowledge regressions.
Phase 4 profession profiles now enforce unique professions, valid capability
levels, aligned capability professions, and bounded capability text, with two
focused profile regressions.
Phase 4 local-combat persistence now bounds encounter health, preserves stored
property safety, and enforces status/health agreement, with two focused combat
regressions.
Phase 4 animal persistence now validates world position, safe names, condition
limits, and care timing, with two focused farming regressions.
Phase 4 school-lesson persistence now validates bounded lesson payloads,
distinct live participants, start/expiry ordering, and the configured lesson
room cap, with three focused lesson-state regressions.
Phase 4 governance history now validates bounded decision and tax-receipt
payloads, positive tax rates, and decision/receipt dates that cannot lead the
authoritative world clock, with two focused governance regressions.
Phase 4 governance metadata now bounds office, proposal, and tax-policy text,
keeps occupied office names aligned with their holders, and requires completed
proposals to carry completion records, with three focused governance metadata
regressions.
Phase 4 infrastructure persistence now bounds names and failure notes and
keeps upkeep records nonzero, with two focused infrastructure regressions.
Phase 4 land-claim persistence now bounds claim narratives and identifiers,
keeps owner names aligned with owners, and rejects reversed nonzero real-time
lease bounds, with three focused claim regressions.
Phase 4 service-order persistence now bounds order text, keeps provider IDs and
names paired, and requires completed orders to carry completion ticks, with
three focused order regressions.
Phase 4 profession persistence now bounds reputation and the separate
credential ledger, with two focused profession-state regressions.
Regional event persistence now requires retained events to remain newer than
the regional history floor, with one focused history-retention regression.
Production identity replay caches now bind each cached link response to the
identity encoded by its key, with one focused cross-account cache regression.
Production refresh replay caches now validate request-key shape and retained
session/account alignment, with one focused cross-account cache regression.
Moderation replay caches now bind their keys to live identity and request
records, with one focused moderation-key regression.
Support repair replay caches now bind their keys to an authenticated operator
account and request, with one focused support-key regression.
Production revoke replay caches now bind their keys to the issuing identity
and request, with one focused revoke-key regression.
Production replay caches now enforce the shared bounded entry window, with one
focused valid-entry overflow regression.
Character skill ledgers now validate catalog membership, unique discoveries,
bounded qualifying history, positive counters, entry caps, and qualified
discovery requirements, with four focused skill-ledger regressions.
Regional travel persistence now keeps status, progress, and ETA in agreement,
with one focused travel-timeline regression.

Core world-event persistence now validates payload bounds, nested cursor
agreement, historical clock ordering, account references, and the structural
shape of farming, trade, notice, chronicle, and frontier records. Three focused
event-payload regressions plus the existing event-stream tests cover this
changed subsystem.

Core identity persistence now bounds client, account, character, and display
identifiers and rejects control characters before readiness is reported. Two
focused identity-payload regressions cover this changed subsystem.

Core retained chat and notice history now enforces its queue caps, bounded
display/channel/text metadata, and non-future notice timestamps. Two focused
history-payload regressions cover this changed subsystem.

Core live session persistence now validates bounded token and identity keys,
identity ownership, client/identity agreement, and activity timestamps. Two
focused session-integrity regressions cover this changed subsystem.

The skill manifest boundary now rejects unsafe player guidance, oversized
identifiers, duplicate or oversized prerequisite lists, invalid practice keys,
and malformed qualifying-event names, with two focused catalogue regressions.

The shared content ID boundary now rejects control characters and IDs over 160
characters across every validated manifest, with one focused content-validator
regression.

Phase 4 readiness now enforces the existing caps for claims, service orders,
infrastructure, governance decisions, and tax receipts. It also bounds the
per-identity profession, capability, and credential ledgers before a restored
snapshot can report healthy, with focused Phase 4 capacity regressions covering
those retained collections.

Regional readiness now also rejects a market-order ledger above its existing
128-record retention cap, with the market retention regression exercising the
changed Phase 5 boundary.

Core trade readiness now rejects a direct-trade ledger above its existing
128-record retention cap, with a focused trade-retention regression covering the
same restored-state boundary.

The content runtime now keeps manifest validation in its own child module so
future content additions do not push the catalog access module toward the
workspace's 800-line Rust limit. The 37 content-focused tests and the project
publisher both pass after the organization-only change.

The player-facing Account control now remains linkable only while the current
projection is a guest fixture; linked characters see a disabled Linked state
and retain the Logout and deletion controls. The 13 focused client account-
lifecycle tests, client clippy check, size audit, diff check, and project
publisher pass after this recovery-path fix.

Explicit logout and expired production sessions now clear the remembered linked
client key before the visible Reconnect path, allowing the local release
candidate to return as a fresh guest fixture instead of retrying a protected
production identity. The focused reconnect regression and client checks pass.

The Delete control now reflects the same account boundary: it is enabled only
for a linked production projection, while a guest must use Account first. The
focused account-lifecycle tests cover both states alongside the client checks.

The shared, regional, settlement-chronicle, and chronicle-search HTTP routes
now reject malformed `since` values with `invalid_cursor` instead of silently
resetting history reads to cursor zero. The focused HTTP cursor regression,
server clippy check, size audit, diff check, and project publisher pass.

Chronicle search now also rejects malformed form encoding in its `q` parameter
while preserving an omitted query as an unfiltered search. The focused HTTP
query suite passes 8 tests with server clippy, size, diff, and publisher checks.

The support-account query now preserves malformed form encoding as a structured
`invalid_query` response instead of silently converting it into an empty target
account. The focused HTTP validation regression covers the shared parser.

Recovery now validates a self-recovery seed or healer balance before clearing
knockout state or moving the character home. The focused local-combat recovery
regression confirms a rejected no-seed choice leaves the stranded position
unchanged.

The HTTP authentication boundary now accepts case-insensitive Bearer scheme
names while requiring non-empty credentials without control characters. The
focused HTTP suite passes 9 tests, with server clippy, size, diff, and publisher
checks passing for the same boundary.

Regional route Repair, Escort, and Improve actions now keep a persisted
per-route availability beat, limiting accepted logistics changes to one step
per configured regional decision interval. The focused route-history suite
covers same-beat rejection and post-interval acceptance, while regional
readiness validates cooldown keys against recorded routes.

Direct trade creation now rejects an empty offer and empty request before
allocating a ledger row, while preserving item-only and gold-only exchanges.
The focused trade suites cover the rejection, retention, numeric-boundary, and
completed-exchange paths, with protocol tests, server clippy, size, and diff
checks passing for the changed boundary.

Regional market creation now checks player and fallback supply before evicting
settled history to make room. A failed missing-supply request therefore leaves
the retained order ledger unchanged; the focused market-retention suite covers
that rejection beside the existing room-making and full-ledger paths, with
server clippy, size, and diff checks passing.

Phase 6 audit and session helpers now live in a dedicated child module, keeping
the repository coordinator below the workspace's 800-line Rust limit while
preserving the existing behavior. The 43 Phase 6-focused tests, server-only
clippy check, size audit, and project publisher all pass after this
organization-only change.

The cross-layer persistence-readiness milestone passed the full release gate on
2026-08-30 after the core and Phase 3-6 integrity additions: 14 protocol tests,
282 server tests, and 93 client tests, followed by asset/code-standard checks,
clippy, Windows and WebGL release builds, packaging, Preview deployment, and
catalog synchronization. Future slices return to changed-subsystem tests
unless they cross another major release boundary.

Account deletion now anonymises composite moderation audit targets such as a
deleted account paired with a retained message evidence ID, while preserving
the report audit. The focused account-cleanup suite covers this target
boundary; the next full workspace gate remains reserved for a major milestone.

Terminal pioneer expeditions now reject late supply and relaunch commands,
preserving the succeeded or retreated lifecycle boundary. The focused Phase 3
expedition flow covers both rejected mutations after successful resolution;
server clippy, size, diff, and publisher checks pass for the change.

Engaged local encounters now reject a second Prepare action, preserving the
equipped weapon until the traveller finishes or retreats. The existing focused
combat-action test covers the rejected weapon swap; server clippy, size, diff,
and publisher checks are the validation scope for this small Phase 4 boundary.

Public-work completion now rejects a second commission of the fixed Hearth
workshop record before charging the public treasury. The focused governance
history test covers the stable infrastructure ID and unchanged funds; server
clippy, size, diff, and publisher checks remain the validation scope.

Animal care now measures the player's distance to Bellweather's authoritative
position rather than trusting the requested interaction tile. The focused
farming regression covers a two-tile attempt through an adjacent spoofed tile;
the test, server clippy, size, diff, and publisher checks pass. No new external
or deferred work was opened.

Local combat now waits until an active regional journey has arrived before
accepting preparation. The focused combat regression covers the normal
Whisperwood-to-Saltmere travel overlap; the test, server clippy, size, diff,
and publisher checks pass. No new external or deferred work was opened.

Animal condition decay now accounts for every world day advanced by a single
clock tick, including accelerated multi-day steps, while retaining the normal
one-day behaviour. The focused multi-day and existing animal-care regressions,
server clippy, size, diff, and publisher checks pass. No new external or
deferred work was opened.

Full account-deletion queues now count a blocked request in the operational
rejection metrics and persist that updated counter. The focused deletion-queue
regression covers the full-queue response and rejection count; server clippy,
size, and diff checks are the validation scope for this Phase 6 maintenance
slice. No new external or deferred work was opened.



Expired regional route-action cooldown entries are now pruned at the
authoritative Phase 5 tick, so a route that is ready again cannot leave the
readiness integrity check degraded. The focused route-history regression covers
cooldown expiry and cleanup; server clippy, size, diff, and publisher checks are
the validation scope for this regional maintenance slice. No new external or
deferred work was opened.

Guest-to-production account linking now migrates account-scoped support-repair
replay keys, preserving idempotency when a support operator retries the same
request with the newly issued production session. The focused account-lifecycle
regression covers that replay boundary; server clippy, size, diff, and publisher
checks are the validation scope for this Phase 6 identity-maintenance slice. No
new external or deferred work was opened.

Chronicle search now rejects a cursor ahead of the authoritative world with the
same structured `cursor_ahead` boundary used by the other history endpoints.
The focused chronicle-search regression covers the invalid cursor response;
server clippy, size, diff, formatting, and publisher checks are the validation
scope for this Phase 6 history-maintenance slice. No new external or deferred
work was opened.

Account deletion now removes only the departing identity's complete moderation
replay key, so a client-key prefix collision cannot discard another player's
idempotent report response. The focused account-deletion regression covers that
identity boundary; server clippy, size, diff, formatting, and publisher checks
are the validation scope for this Phase 6 privacy-maintenance slice. No new
external or deferred work was opened.

Development-guest reset now applies the same exact moderation replay-key
cleanup as production account deletion, so an identity-prefix collision cannot
discard another player's idempotent report response. The focused reset
regression covers that privacy boundary; server clippy, size, diff, formatting,
and publisher checks are the validation scope for this Phase 6 maintenance
slice. No new external or deferred work was opened.

Reset and account deletion now use complete cached request IDs when cleaning
identity-scoped Phase 3, Phase 5, moderation, and revoke replay maps. This
prevents a client-key prefix collision from discarding a neighbouring player's
idempotent response and removes the resetting guest's own revoke replay. The
focused replay-cleanup regressions cover the boundary; server clippy, size,
diff, formatting, and publisher checks are the validation scope for this Phase
6 privacy-maintenance slice. No new external or deferred work was opened.

Auth-link token retention and readiness validation now require the complete
identity/request replay key, so another identity sharing a client-key prefix
cannot make a missing link result appear healthy or keep its stale token alive.
The focused replay-integrity regression covers detection and pruning; server
clippy, size, diff, formatting, and publisher checks are the validation scope
for this Phase 6 identity-maintenance slice. No new external or deferred work
was opened.

Account-link migration and account/reset cleanup now compare complete Phase 4
and support-repair replay keys against their cached response request IDs. This
keeps a delimiter-like account boundary from moving or deleting a neighbouring
player's idempotent response. The focused Phase 4/support boundary tests and
the account-link, deletion, and reset regressions pass; server clippy, size,
diff, formatting, and publisher checks are the validation scope for this Phase
6 identity-maintenance slice. No new external or deferred work was opened.

Cursor recovery now clears stale Phase 4 projections, local crafting state, and
queued Phase 4/Phase 5 mutations before reloading authoritative state and
history. Regional cursor recovery also cancels stale regional reads and event
commands, while production authentication refresh state remains intact. The
focused global, Phase 4, and regional cursor tests pass; client formatting,
client clippy, size, diff, and publisher checks are the validation scope for
this Phase 6 recovery slice. No new external or deferred work was opened.

Expedition announcements now append one frontier event through the shared
accepted-response path instead of duplicating the same planning event. The
focused event-count regression passes; server clippy, formatting, size, diff,
and publisher checks are the validation scope for this Phase 3 event-stream
maintenance slice. No new external or deferred work was opened.

Expedition actions now bind a supplied selector to the authoritative pioneer
record, so stale Join, Supply, Launch, or Resolve requests cannot mutate the
current expedition; omitted selectors remain compatible with the existing
client. The focused stale-selector regression passes; server clippy, rustfmt,
size, diff, and publisher checks are the validation scope for this Phase 3
frontier maintenance slice. No new external or deferred work was opened.

The pioneer registry now permits a staffed but under-supplied party to attempt
departure, and resolves that attempt as a durable retreat without founding the
outpost or granting expedition credentials. Fully supplied parties retain the
successful outpost path. The focused retreat regression passes; server clippy,
rustfmt, size, diff, and publisher checks are the validation scope for this
Phase 3 frontier-scope slice. No new external or deferred work was opened.

Expedition launch history now distinguishes a staffed attempt from a fully
prepared party, and the client notification names the authoritative outpost
result, including the retreat explanation. The focused Phase 3 retreat and
client notification regressions are the validation scope for this presentation
and wording maintenance slice; no full workspace gate was rerun because the
change stays within the established expedition boundary. No new external or
deferred work was opened.

The client map now consumes the existing successful expedition projection and
draws the founded outpost at its authoritative position, while retreated or
planning expeditions remain absent from the landmark layer. Client clippy,
formatting, size, diff, and publisher checks are the validation scope for this
small Phase 3 presentation slice; no new external or deferred work was opened.

The pioneer outpost now uses the validated Phase 5 region location for the
Phase 3 frontier site, and the map replaces that static location label with
the player-founded outpost name after success. The focused durable expedition
flow, client UI checks, formatting, size, diff, and publisher checks are the
validation scope for this cross-phase landmark correction; no new external or
deferred work was opened.

Support claim repair now removes a stale free-plot entry when restoring an
active land right, and Phase 6 readiness rejects any non-reclaimed claim that
is still listed as available. The focused support repair regression, server
clippy, formatting, size, and diff checks pass; no full workspace gate was
rerun because this is a bounded claim-recovery maintenance slice. No new
external or deferred work was opened.

Session revocation now removes the active legacy guest bearer as well as
production access sessions, so the visible logout boundary cannot leave a guest
token usable until its normal timeout. The focused guest reset/logout regression,
server clippy, formatting, size, and diff checks pass; no full workspace gate
was rerun because this is a bounded session-boundary maintenance slice. No new
external or deferred work was opened.

Guest revocation also keeps a bounded fingerprint tombstone long enough to
identify the same request after the bearer has been removed, preserving replay
idempotency without persisting the token itself. The focused guest revoke replay
and reset cleanup assertions, server clippy, formatting, size, and diff checks
pass; no full workspace gate was rerun because this is a continuation of the
same session-boundary maintenance slice. No new external or deferred work was
opened.

Development guest reset now anonymises retained presence, chat, frontier, and
chronicle records before removing the old identity, preventing stale account
references from degrading readiness or reappearing in the next guest fixture.
The focused reset-history regression also verifies the resulting readiness
projection; server clippy, formatting, size, and diff checks pass. No full
workspace gate was rerun because this is a bounded development-identity cleanup
slice. No new external or deferred work was opened.

The persisted guest-revocation fingerprint cache now receives the same bounded
trim during world loading as it does during tick maintenance. The focused
persistence cache regression verifies both the backup and restart boundaries;
server clippy, formatting, size, diff, and publisher checks pass. No full
workspace gate was rerun because this is a bounded Phase 6 replay-retention
maintenance slice. No new external or deferred work was opened.

Persisted regional route-action cooldowns now discard entries whose availability
beat has already passed before the world reopens. The focused restart-readiness
regression verifies the expired route boundary; server clippy, formatting, size,
diff, and publisher checks pass. No full workspace gate was rerun because this
is a bounded Phase 5 retention maintenance slice. No new external or deferred
work was opened.

Explicit claim reclamation now honours the configured grace interval instead of
opening an expired or abandoned plot immediately to a new claimant. The focused
claim-lifecycle regression verifies the too-early path; server clippy,
formatting, size, diff, and publisher checks pass. No full workspace gate was
rerun because this is a bounded Phase 4 lease-rule maintenance slice. No new
external or deferred work was opened.

The claim grace boundary is now enforced by the explicit lifecycle command as
well as the authoritative tick path, so a reclaim request cannot make an
expired or abandoned plot available before its recorded interval. The focused
Phase 4 lifecycle regression, server clippy, formatting, size, diff, and
publisher checks pass. No full workspace gate was rerun because this remains a
bounded lease-lifecycle maintenance slice. No new external or deferred work
was opened.

Lease summaries now label abandoned and expired claims as “grace pending” until
the registry interval actually opens, matching the protected server state. The
focused client summary regression, client clippy, formatting, size, diff, and
publisher checks pass. No full workspace gate was rerun because this is a
bounded Phase 4 presentation maintenance slice. No new external or deferred
work was opened.

Governance and knowledge inspection are now read-only at the repository
boundary. Repeated same-beat inspection returns the current projection without
replaying a stale response, adding an audit, or inflating command counters. The
focused Phase 4 read-view regressions, server clippy, diff, Rust file-size, and
publisher checks pass. The workspace formatter still reports pre-existing
formatting differences in unrelated committed files; the edited files were
formatted directly. No full workspace gate was rerun because this is a bounded
Phase 4 read-path maintenance slice. No new external or deferred work was
opened.

Profession and local-combat inspection now project their default views without
materializing player records in the authoritative Phase 4 state. The focused
read-view regressions verify default projections and empty mutation maps;
server clippy, direct formatting, size, diff, and publisher checks pass. No
full workspace gate was rerun because this is a continuation of the bounded
Phase 4 read-path maintenance slice. No new external or deferred work was
opened.

Selectorless lease approval now chooses the caller's own requested lease for
ordinary residents while preserving the Settlement Steward's ability to
approve the oldest pending request. The focused Phase 4 claim-lifecycle
regression, server clippy, direct formatting, size, and diff checks pass. The
publisher check also passes for this bounded Phase 4 selector maintenance
slice; no full workspace gate is required. No new external or deferred work
was opened.

Governance audit targets now follow the command being recorded: approvals and
completions retain their selected proposal, while office and tax actions no
longer inherit an unrelated newest proposal. The focused Phase 4 audit-target
regression, server clippy, direct formatting, diff, and Rust file-size checks
pass. The publisher check also passes for this bounded Phase 4 audit
maintenance slice; no full workspace gate is required.
No new external or deferred work was opened.

Phase 4 client ledger polling now keeps the newest response cursor and ignores
older in-flight GET or command projections, preventing a delayed read from
rewinding a just-confirmed command result. The focused client Phase 4 tests
(28 passed), direct formatting, diff, Rust file-size, and client clippy checks
pass. The publisher check remains the runtime validation path for this client
network maintenance slice; no full workspace gate was rerun because the change
is bounded to cursor ordering in the Phase 4 client. No new external or
deferred work was opened.

Phase 5 regional polling now keeps the newest server cursor across map,
settlement, household, market, and event projections, so a delayed read cannot
rewind a route or shipment state after a newer regional command. The cursor
boundary resets on session and regional-history recovery. The focused client
Phase 5 tests (28 passed), direct formatting, diff, Rust file-size, and client
clippy checks pass. The publisher check remains the runtime validation path;
no full workspace gate was rerun because this is a bounded continuation of the
client cursor-ordering maintenance. No new external or deferred work was
opened.

Root client projection polling now orders full state, incremental events,
movement, farming, chat, trade, and frontier responses by the authoritative
server tick and event cursor. Delayed state or event responses can no longer
rewind a newer world projection or confirmed movement. The focused client
network regression, direct formatting, diff, Rust file-size, and client clippy
checks pass. The publisher check remains the runtime validation path; no full
workspace gate was rerun because this is a bounded continuation of client
cursor-ordering maintenance. No new external or deferred work was opened.

Phase 4 and Phase 5 ledger responses, regional history, and frontier read views
now also advance the root client projection version before applying their
bounded caches. A delayed root snapshot can no longer rewind world state after
an auxiliary projection response. The focused client network tests (88 passed),
direct formatting, diff, Rust file-size, and client clippy checks pass. The
publisher check remains the runtime validation path; no full workspace gate was
rerun because this is a bounded continuation of client cursor-ordering
maintenance. No new external or deferred work was opened.

The regional law and authenticated-account reads now use the same monotonic
cursor boundary as the other Phase 5 projections, so a delayed guest response
cannot replace a newly linked production account view. The focused Phase 5
tests (29 passed), direct formatting, diff, Rust file-size, and client clippy
checks pass. The publisher check remains the runtime validation path; no full
workspace gate was rerun because this is a bounded Phase 5 client ordering
maintenance slice. No new external or deferred work was opened.

Frontier command responses now decide whether their player, claim, wilderness,
and expedition fields are still current before mutating the root projection;
late responses still retain their outcome notice without rewinding newer state.
The focused frontier tests (8 passed), direct formatting, diff, Rust file-size,
and client clippy checks pass. The publisher check remains the runtime
validation path; no full workspace gate was rerun because this is a bounded
frontier client ordering maintenance slice. No new external or deferred work
was opened.

Phase 4 command responses now compare both server tick and event cursor against
the root projection before replacing their bounded settlement ledgers. A late
same-cursor command response can still show its outcome notice without
rewinding a newer registry, governance, profession, knowledge, skill, or
combat cache. The focused Phase 4 client tests (29 passed), direct formatting,
diff, Rust file-size, and client clippy checks pass. The publisher check remains
the runtime validation path; no full workspace gate was rerun because this is a
bounded continuation of client cursor-ordering maintenance. No new external or
deferred work was opened.

Auxiliary Phase 4 and Phase 5 ledgers now compare server tick as well as event
cursor before replacing cached governance, claim, profession, knowledge, skill,
combat, regional, market, law, account, household, and event projections. A
late accepted chat response likewise keeps its outcome notice without
appending stale history. The focused Phase 4 tests (29 passed), Phase 5 tests
(29 passed), root projection regression (1 passed), direct formatting, diff,
Rust file-size, and client clippy checks pass. The publisher check remains the
runtime validation path; no full workspace gate was rerun because this is a
bounded continuation of client cursor-ordering maintenance. No new external or
deferred work was opened.

The movement boundary now uses checked coordinate addition before consulting
the map, so a corrupted persisted position cannot panic the server on an
otherwise cardinal step. The focused movement validation tests (2 passed),
server clippy, direct formatting, diff, and Rust file-size checks pass. The
publisher check remains the runtime validation path; no full workspace gate was
rerun because this is a bounded server input-safety maintenance slice. No new
external or deferred work was opened.

Farming now rejects every shared-field action while a player is knocked out,
preserving the recovery boundary already enforced by movement, travel, and
combat. The focused farming tests (10 passed), server clippy, direct formatting,
diff, and Rust file-size checks pass. The publisher check remains the runtime
validation path; no full workspace gate was rerun because this is a bounded
Phase 4 farming authority slice. No new external or deferred work was opened.

Movement now checks signed positions by converting the coordinate itself to an
unsigned bound instead of casting configured world dimensions down to `i32`,
so a wide configured world cannot falsely reject the positive coordinate edge.
The focused movement validation tests (3 passed), server clippy, direct
formatting, diff, and Rust file-size checks pass. The publisher check remains
the runtime validation path; no full workspace gate was rerun because this is a
bounded movement input-safety slice. No new external or deferred work was
opened.

The online sidebar now disables Plant, Tend, Harvest, and Care while the
character is knocked out, matching the authoritative farming recovery boundary
and keeping the visible touch surface from advertising unavailable work. The
focused online UI tests (3 passed), client clippy, direct formatting, diff, and
Rust file-size checks pass. The publisher check remains the runtime validation
path; no full workspace gate was rerun because this is a bounded client
presentation slice. No new external or deferred work was opened.

The online regional repair selector now keeps a Closed route available for
Repair while continuing to exclude Closed routes from Escort and Improve,
matching the server recovery path and the visible non-operational repair
affordance. The focused Phase 5 client tests (30 passed), client clippy, direct
formatting, diff, and Rust file-size checks pass. The publisher check remains
the runtime validation path; no full workspace gate was rerun because this is a
bounded Phase 5 client selector maintenance slice. No new external or deferred
work was opened.

Regional inspection now preserves the sidebar Repair action while its detail
panel is open, and disables Escort and Improve when the player's local routes
are all Closed. This keeps touch controls aligned with the client route
selectors and the server's recovery boundary. The focused online UI tests (4
passed), client clippy, direct formatting, diff, and Rust file-size checks pass.
The publisher check remains the runtime validation path; no full workspace gate
was rerun because this is a bounded regional inspection affordance slice. No
new external or deferred work was opened.

Regional market fulfilment now rejects the order owner at the destination, so
a player cannot award their own shipment both its goods and its sale gold. The
client also ignores the owner's open order when selecting a destination
fulfilment action. Focused market tests (3 server and 31 client tests passed),
server and client clippy, direct formatting, diff, and Rust file-size checks
pass. The publisher check remains the runtime validation path; no full
workspace gate was rerun because this is a bounded regional economy safety
slice. No new external or deferred work was opened.

Regional route action eligibility now agrees across authority and touch
selectors: Repair is for non-operational roads, Escort can recover a Closed
road as promised by the travel notice, and Improve waits until a road is open.
The focused route-history tests (5 server tests), Phase 5 client tests (31
passed), online UI tests (4 passed), server and client clippy, direct formatting
of changed implementation and client-test files, diff, and Rust file-size
checks pass. The known pre-existing formatting drift in the route-history
fixture was audited and remains untouched. The publisher check remains the
runtime validation path; no full workspace gate was rerun because this is a
bounded regional route-action contract slice. No new external or deferred work
was opened.

The near-limit combat and Phase 4 client test files were split along cohesive
responsibilities into named child modules, keeping each Rust test file below
800 lines without changing runtime behavior. The extracted server weapon
experience test (1 passed) and client projection-ordering tests (3 passed),
server and client all-target clippy, direct formatting, diff, and Rust
file-size checks pass. No publisher run was needed because this was a
test-organization-only slice. No new external or deferred work was opened.

The online map tap and directional pad now stop emitting movement while the
character is knocked out, and the map tooltip points to the visible recovery
prompts instead. The focused online UI tests (4 passed), client clippy, direct
formatting, diff, and Rust file-size checks pass. The publisher check passed
the Windows and WebGL release builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was rerun because this is a bounded
client recovery-affordance slice. No new external or deferred work was
opened.

The client regional transport module's command-dispatch responsibility moved
into a named sync module, reducing the main file from 749 to 692 lines while
preserving its behavior. The focused Phase 5 client tests (31 passed), client
clippy, direct formatting, diff, and Rust file-size checks pass. No publisher
run was needed because this was a test-adjacent organization-only slice. No
new external or deferred work was opened.

The Travel control now enables itself only when an open route touches the
player's current location, matching the route chosen by command dispatch. The
focused disconnected-route regression (1 passed), client clippy, direct
formatting, diff, and Rust file-size checks pass. The publisher check passed
the Windows and WebGL release builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was rerun because this is a bounded
regional selector slice. No new external or deferred work was opened.

The pioneer client now selects the first genuinely missing complementary role,
including Scout, so a party announced under another role cannot become trapped
in a Join/Launch cycle. The focused frontier role-selection regression passes;
client clippy, direct formatting, diff, and size checks pass. The publisher
check passed the Windows and WebGL release builds, packaging, Preview
deployment, and catalog sync. No full workspace gate was rerun because this is
a bounded client selector slice. No new external or deferred work was opened.

Expedition launch and resolution now require the authenticated character to be
named on the pioneer party, matching the existing membership boundary for join
and supply actions. The focused frontier authority regression passes; server
clippy, direct formatting, diff, and size checks pass. No publisher run was
needed separately because the combined frontier publisher run above covered the
runtime change. No full workspace gate was rerun because this is a bounded
server authority slice. No new external or deferred work was opened.

The online sidebar now keeps the authoritative pioneer expedition visible while
it is planning, travelling, founded, or retreated, including party size and
bounded supply totals. The focused online UI tests pass; client clippy, direct
formatting, diff, and size checks pass. The publisher check passed the Windows
and WebGL release builds, packaging, Preview deployment, and catalog sync. No
full workspace gate was rerun because this is a bounded presentation slice. No
new external or deferred work was opened.

The online Travel and Recover controls now close while the character is knocked
out, matching the server's journey recovery boundary and leaving the visible
recovery choices as the next touch action. The focused online UI regression (1
passed), client clippy, direct formatting, diff, and Rust file-size checks pass.
The publisher check passed the Windows and WebGL release builds, packaging,
Preview deployment, and catalog sync. No full workspace gate was rerun because
this is a bounded client presentation slice. No new external or deferred work
was opened.

The regional expedition policy now travels with the authoritative world
snapshot. The client uses server-advertised food, tools, materials, and safety
minimums before offering Launch, while older snapshots retain the default
compatibility values. The focused protocol/client/server regressions pass (1
client and 1 server test), full workspace clippy passes, and the publisher check
passed the Windows and WebGL release builds, packaging, Preview deployment, and
catalog sync. The milestone release gate reached content validation but stopped
on known unrelated formatter drift in `route_history.rs`,
`production_integrity.rs`, `reset.rs`, and its reset test. A manual full
workspace test then recorded 387 passing tests and 11 existing stale fixture or
read-path failures outside this contract; the focused regressions remain green.
Direct formatting of changed files, diff, and Rust file-size checks pass. No new
external or deferred work was opened.

The land registry now leaves abandoned and expired leases in their visible grace
state until a later `Reclaim` action opens the plot, while still closing expired
building access during the background tick. This matches the Phase 4 registry
runbook and prevents an automatic tick from consuming the player's reclaim
interaction. The focused claim-lifecycle suite (4 passed), the complete
land-rights lifecycle regression (1 passed), server clippy, direct formatting,
and diff checks pass. No full workspace suite was rerun because the milestone
record still contains the unrelated formatter drift and the remaining stale
fixture/read-path failures documented above. No new external or deferred work
was opened.

The Phase 4 integrity fixtures now materialize profession ledgers through the
mutating inspection action before corrupting them, preserving the intentional
read-only behavior of `GET /professions`. The focused profession-integrity
tests (2 passed), service-order retention tests (3 passed), and the two touched
state-integrity regressions (1 passed each) pass with server clippy and direct
formatting. No full workspace suite or publisher run was needed for this
test-only maintenance slice. No new external or deferred work was opened.

The Phase 5 market fixtures now use a separate carrier identity when settling a
shipment at Saltmere, matching the live rule that an order owner cannot fulfil
their own shipment. The focused fallback and regional-flow regressions (1
passed each), server clippy, direct formatting, and diff checks pass. No full
workspace suite or publisher run was needed for this test-only maintenance
slice. No new external or deferred work was opened.

The Phase 4 combat-integrity fixtures now establish durable combat state through
the mutating `Prepare` action at Whisperwood before injecting invalid health or
status values. This preserves the intentional read-only default projection of
`GET /combat/local`. The two focused combat-readiness regressions (1 passed
each), server clippy, direct formatting, and diff checks pass. No full workspace
suite or publisher run was needed for this test-only maintenance slice. No new
external or deferred work was opened.

The adventurer-history fixture now gives expedition resolution the three real
complementary participants and configured supplies required by the current
frontier authority before asserting the Lantern Rest credential. The focused
expedition-history regression passes (1), along with server clippy, direct
formatting, and diff checks. No full workspace suite or publisher run was
needed for this test-only maintenance slice. No new external or deferred work
was opened.

The complete milestone gate is green after the expedition contract and
validation-fixture repairs: content manifests passed; 14 protocol, 398 server,
and 110 client tests passed; asset and code standards passed; workspace clippy
passed with warnings denied; and Windows/WebGL release builds, packaging,
Preview deployment, and catalog sync completed successfully. The gate required
removing only the exact stale generated Windows archive before packaging. The
future-incompatibility warning for `net2 v0.2.39` remains non-blocking. No new
external or deferred work was opened.

The online pioneer status now pairs each server-advertised supply total with
its required minimum in a compact `F/T/M current/required` line, so a disabled
Launch state remains explainable when deployment thresholds differ from the
development defaults. The focused UI regression passes (1), client clippy,
direct formatting, diff, and Rust file-size checks pass, and the publisher
check completed Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was rerun because this is a bounded client
presentation slice. No new external or deferred work was opened.

The restore path now migrates older regional snapshots that omitted
`fallback_day`, carries invalid historical opportunity scores back into the
bounded `0..=100` range, and anonymizes orphaned public-history and audit
actors as former residents. The live household tick clamps future score
pressure at zero. The focused migration and runtime regressions pass (1 each),
the JSON restore failure drill passes with the active-state hash unchanged, and
server formatting and clippy checks pass. The publisher check also passes its
Windows/WebGL builds, packaging, Preview deployment, and catalog sync; the
existing `net2 v0.2.39` future-incompatibility warning remains non-blocking. No
full workspace suite was repeated because this was a focused
persistence/runtime recovery slice; the complete milestone gate remains green
in the preceding entry. No new external or deferred work was opened.

The operations health response now reports fixed, non-sensitive integrity
failure codes alongside its existing boolean readiness result, allowing an
operator to identify a failed boundary without exposing ledger contents or
secrets. The focused route-boundary regression passes (1), server clippy,
direct formatting, and diff checks pass. The publisher check is required for
this protocol/server runtime slice; no full workspace suite or release gate
was repeated because this is not a major milestone. No new external or
deferred work was opened.

The protocol compatibility fixture now proves that a health payload produced
before integrity detail codes existed still decodes with an empty failure
list. The focused protocol regression passes (1), with direct formatting and
diff checks clean. No server, client, workspace, or publisher validation was
repeated because this follow-up changes only protocol test coverage; the
runtime publisher result is recorded in the preceding entry. No new external
or deferred work was opened.

The restore-on-a-copy failure drill now also requires a healthy restored
server to return an empty `integrity_failures` list, covering the new health
diagnostic contract at the executable recovery boundary. The focused restore
drill passes, with no runtime source changes and no workspace or publisher
validation repeated. No new external or deferred work was opened.

The MySQL verifier now clears inherited database and test variables before
loading `.env.preview` and fails fast when the canonical host, username, or
password keys are absent. A rerun reached that guard because the ignored local
preview file does not define `DB_USERNAME`; it no longer attempts the unrelated
inherited `ODBC` account. The target migration, restore, and persistence gate
remains unclaimed until valid preview credentials are supplied. No new
external or deferred work was opened.

The JSON Phase 6 load and restore drills now force `DB_DRIVER=json` while
preserving and restoring the caller's environment, so a shell that previously
loaded MySQL credentials cannot silently redirect an isolated drill to the
shared database. Both affected scripts pass when launched under an
intentionally wrong ambient MySQL driver; no new external or deferred work was
opened.

The Phase 1, Phase 2, Phase 3, and Phase 4 JSON acceptance harnesses now force
`DB_DRIVER=json` for their temporary workers and restore the caller's value on
exit. Each affected script passes under an intentionally wrong ambient MySQL
driver, preventing a shell-level database setting from redirecting historical
acceptance fixtures. No new external or deferred work was opened.

The Phase 3 cursor acceptance now tolerates legitimate clock events arriving
between the cursor read and its follow-up request, while still rejecting any
event at or before the accepted cursor and any backwards cursor movement. The
affected Phase 3 harness passes under an intentionally wrong ambient MySQL
driver; no new external or deferred work was opened.

The Phase 4 lease acceptance now waits for the authoritative worker to advance
past the configured reclamation grace interval before sending `Reclaim`, so
the fixture exercises the current protected-grace rule instead of relying on a
race between HTTP requests. The affected Phase 4 harness passes under an
intentionally wrong ambient MySQL driver; no new external or deferred work was
opened.

The Phase 4 runbook and acceptance harness now use the server's real-time
`TARROWYN_LEASE_DURATION_SECONDS` setting; the harness no longer carries a
stale tick-based lease variable that the server ignores. The affected live
acceptance passes, with no runtime source changes and no full workspace or
publisher validation repeated. No new external or deferred work was opened.

Guest-session admission now applies a bounded 32-attempt, 60-second per-source
HTTP window before creating another durable development identity. The focused
HTTP limiter regressions pass (3), server formatting and package clippy remain
clean, and the 24-client target remains below the local window. The publisher
check passes its Windows/WebGL builds, packaging, Preview deployment, and
catalog sync; the existing `net2 v0.2.39` future-incompatibility warning is
non-blocking. No full workspace gate was repeated and no new external or
deferred work was opened.

The HTTP dispatcher no longer wraps JSON responses in CORS headers a second
time after `json_response` has applied them. The focused response-boundary
tests pass (2), including the exact-one-origin-header regression, and
server-package clippy plus workspace formatting remain clean. No full
workspace or publisher gate was repeated because this is an isolated HTTP
header fix; no new external or deferred work was opened.

The public readiness projection now omits the configured backup filesystem
path while retaining the last successful backup tick; the client does not
need deployment paths to show maintenance or recovery status. The focused
operations regression passes (1), with server-package clippy, formatting, and
diff checks clean. No full workspace or publisher gate was repeated because
this is an isolated public-health privacy fix; no new external or deferred
work was opened.

The bounded guest-session `429` response now carries the standard
`Retry-After: 60` recovery hint, allowing touch clients and deployment proxies
to wait for the same admission window. The focused HTTP header regression
passes (1), with server-package clippy, formatting, and diff checks clean. No
full workspace or publisher gate was repeated because this is a narrow
rate-limit response improvement; no new external or deferred work was opened.

The affected HTTP/security slices were then validated through `publish.ps1`:
Windows and WebGL release builds, packaging, Preview deployment, catalog sync,
and the existing non-blocking `net2 v0.2.39` future-incompatibility warning
were observed as expected. No full workspace suite was repeated, and no new
external or deferred work was opened.

Production link and refresh sessions now use independent 32-byte operating
system-random credentials instead of predictable session counters; development
guest fixtures remain unchanged. The focused session-integrity tests pass (4
including the existing core session checks), server-package clippy and
formatting pass, and no full workspace or publisher gate was repeated because
this is an isolated credential-issuance hardening change. No new external or
deferred work was opened.

Phase 6 readiness now accepts the documented counter-shaped legacy production
credentials during their expiry window but rejects malformed credentials and
any new credential shape outside the 64-byte hex format. The focused malformed
credential regression passes (1), with server-package clippy, formatting, and
diff checks clean. No full workspace or publisher gate was repeated because
this is a narrow integrity check paired with the preceding credential
hardening; no new external or deferred work was opened.

Auth refresh and revoked-guest replay fingerprints now use stable SHA-256
hexadecimal keys instead of the previous non-cryptographic 64-bit fingerprint;
legacy persisted replay-key shapes remain readable during expiry and retention.
Cached auth responses are held to the same production credential-shape check as
live sessions. The focused production-integrity, replay-cache, persistence,
and session filters pass (16 tests total), with server-package clippy,
formatting, and diff checks clean. No full workspace or publisher gate was
repeated because this is an isolated replay-integrity hardening change; no new
external or deferred work was opened.

Scheduled backups now flush their complete temporary snapshot to the operating
system before the atomic replacement, while the per-command live snapshot
writer keeps its existing latency profile. The existing complete-backup
regression passes, with server-package clippy, formatting, and diff checks
clean. No full workspace or publisher gate was repeated because this is a
narrow backup durability hardening; no new external or deferred work was
opened.

The Phase 6 security and recovery milestone then passed the full release gate:
15 protocol tests, 410 server tests, 110 client tests, asset and Rust
standards tests, doc tests, warning-denied workspace clippy, content validation,
Windows and WebGL release builds, package creation, Preview deployment, and
catalog sync. The existing `net2 v0.2.39` future-incompatibility warning
remains non-blocking. No new external or deferred work was opened.

The Phase 6 design record now distinguishes the deliberate authenticated
link/refresh credential handoff from support/player projections and audit
records, and records that production credentials use the operating system's
secure random source. This documentation-only consistency fix required no
additional runtime tests; the already-passing full milestone gate remains the
authoritative validation for the implementation. No new external or deferred
work was opened.

Scheduled backup metadata is now included in the snapshot being written, while
failed encoding, directory creation, or replacement restores the previous
metadata instead of reporting an uncommitted backup. The focused backup tests
pass (3 matching tests, including the existing public-health path regression),
with server-package clippy, formatting, diff, and Rust file-size checks clean.
No full workspace or publisher gate was repeated because this is an isolated
backup metadata correctness change; no new external or deferred work was
opened.

JSON persistence and scheduled backups now accept valid filename-only relative
paths by skipping directory creation when a path has no parent directory. The
focused relative-path regression passes (1), exercising both state persistence
and backup replacement, with server-package clippy, formatting, diff, and Rust
file-size checks clean. No full workspace gate was repeated because this is an
isolated path-handling fix; no new external or deferred work was opened.

Scheduled backups now run after the authoritative tick appends its clock and
session-expiry events, so the backup cursor matches the completed live state
instead of stopping one mutation early. The focused backup/persistence tests
pass (4 matching tests), with server-package clippy, formatting, diff, and Rust
file-size checks clean. No full workspace gate was repeated because this is an
isolated backup sequencing fix; no new external or deferred work was opened.

MySQL snapshot loading now compares the indexed storage version, world tick,
and event cursor with the values inside the authoritative JSON document, and
fails closed when the denormalized row metadata disagrees. The focused metadata
regression passes (1), with server-package clippy, formatting, diff, and Rust
file-size checks clean. No live MySQL assertion was claimed because the ignored
preview credentials remain unavailable; no full workspace gate was repeated for
this isolated persistence-integrity check.

MySQL startup now also verifies that the transactional identity index contains
exactly the account and character pairs represented by the loaded JSON world,
including the empty-world case, and rejects orphaned or missing index rows.
The focused MySQL filter passes (4 matching tests), with server-package
clippy, formatting, diff, and Rust file-size checks clean. No live database
assertion was claimed because the ignored preview credentials remain
unavailable; no full workspace gate was repeated for this isolated integrity
check.

The ignored local MySQL preview configuration was corrected from the stale
`DB_USER` spelling to the documented `DB_USERNAME` key without changing its
credential values. A password-safe `SELECT 1` succeeded, but the focused
`scripts/verify_mysql.ps1` run stopped at readiness because the configured
database contains legacy production refresh replay results without the current
ownership mirror; the worker reported `production` integrity failure and failed
closed as designed. No live persistence, restart, or restore assertion was
claimed, and the configured database was not deleted or rewritten. A clean
current snapshot or known-good restore remains required before rerunning the
acceptance script; no full workspace gate was repeated for this configuration
and environment-boundary check.

The client now advances the nested regional session refresh before Phase 4
dispatches its protected reads or mutations. A due refresh therefore blocks a
same-frame governance, profession, knowledge, skill, or combat request, while
regional commands and logout/session invalidation retain the same protection.
The focused Phase 4 race regression passes (1 matching test), with
client-package clippy, formatting, diff, and Rust file-size checks clean.
`publish.ps1` also passed the Windows and WebGL release builds, packaging,
Preview deployment, and catalog synchronization. No full workspace gate was
repeated because this was an isolated client dispatch-order fix.

Regional dispatch now treats an in-flight refresh retry as a protected session
boundary even while its retry timer is waiting, so it cannot start reads or
mutations with the old bearer token. The focused retry-window regression passes
(1 matching test), with client-package clippy, formatting, diff, and Rust
file-size checks clean. `publish.ps1` again passed the Windows and WebGL
release builds, packaging, Preview deployment, and catalog synchronization. No
full workspace gate was repeated because this was an isolated refresh-recovery
guard.
