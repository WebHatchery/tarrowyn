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

Accepted Town Hall responses now explain the requested public action using the
in-flight request and returned governance state: office ownership, the current
tax rate and treasury, posted proposals, approvals with the visible completion
path, and completed action target plus cost. The focused Phase 4 feedback suite
passes three regressions, client clippy and the Rust file-size check pass, and
`publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was repeated because this was a bounded
client Town Hall-feedback slice, and no new external or deferred work was
opened.

Accepted profession responses for capability learning now name the learned
discipline and confirm that its credential entered the profession ledger;
inspection responses also use a ledger-specific message. The focused
profession-feedback regressions pass, client clippy and the Rust file-size
check pass, and `publish.ps1` passes Windows/WebGL builds, packaging, Preview
deployment, and catalog sync. No full workspace gate was repeated because this
was a bounded client capability-feedback slice, and no new external or
deferred work was opened.

Accepted frontier homestead responses now explain the returned lease state:
the plot and recognised access duration while active, the inactivity window
before abandoned land can be reclaimed, and the visible Claim path after
reclamation. The focused homestead-success-message regression passes, client
clippy and the Rust file-size check pass, and `publish.ps1` passes Windows/WebGL
builds, packaging, Preview deployment, and catalog sync. No full workspace gate
was repeated because this was a bounded client homestead-feedback slice, and
no new external or deferred work was opened.

An expired access token reaching the revoke endpoint now passes through the
same change-aware session sweep after replay lookup, so the rejected logout
attempt records its offline presence before returning unauthorized. Existing
revocation replay behavior remains unchanged. The focused
`repository::session::tests` filter passes four tests; server-package formatting
and clippy, `git diff --check`, and the Rust file-size scan pass. The project
`publish.ps1` Windows/WebGL build, packaging, Preview deployment, and
catalog-sync checks pass. No full workspace gate was repeated because this was
a bounded session-expiry endpoint correction, and no new external or deferred
work was opened.

The online chat draft now remains visible when the shared state is reloading or
the bounded chat queue is full; the client clears it only after the chat request
is actually accepted into the queue. The focused chat enqueue regressions pass,
along with client-package clippy, formatting, `git diff --check`, and the Rust
file-size scan. No full workspace gate is repeated because this is a bounded
input-preservation correction. The project `publish.ps1` Windows/WebGL build,
packaging, Preview deployment, tracker recording, and catalog synchronization
also pass.

The online crafting overlay now appears only while the shared connection is
Online, so a disconnect during timing play no longer hides the visible
Reconnect control behind an action that the request layer cannot submit. The
focused binary test
`game::tests::crafting_overlay_waits_for_the_shared_road_before_hiding_reconnect`
passes; client-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. The project `publish.ps1` Windows/WebGL build, packaging,
Preview deployment, and catalog-sync checks pass. No full workspace gate was
repeated because this was a bounded client recovery-overlay correction, and no
new external or deferred work was opened.

Accepted direct-trade responses now explain the returned exchange: the other
resident and the exact give/receive bundles, while retaining the existing
action fallback when no offer record is present. The focused trade-success
notice regressions pass, client clippy and the Rust file-size check pass, and
`publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was repeated because this was a bounded
client trade-feedback slice, and no new external or deferred work was opened.

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

Queued frontier commands now count at the Phase 4 mutation boundary instead of
being mistaken for an idle frontier because only the in-flight request was
checked. The focused
`queued_frontier_mutation_blocks_phase_four_dispatch_until_its_turn` filter
passes one regression test, alongside the preceding general-queue regression;
client formatting, clippy, standards, and the Rust file-size check pass, and
`publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was repeated because this was a bounded
frontier-queue predicate correction, and no new external or deferred work was
opened.

Accepted Order responses now explain the authoritative service result when an
order is posted, accepted, or completed: the service and gold reward, the
named provider, completion quality, and the returned benefit. The focused
profession-success-message regression passes, client clippy and the Rust
file-size check pass, and `publish.ps1` passes Windows/WebGL builds, packaging,
Preview deployment, and catalog sync. No full workspace gate was repeated
because this was a bounded client Order-feedback slice, and no new external or
deferred work was opened.

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

Test maintenance moved the Phase 4 feedback formatters and the six recent
Phase 5 result-feedback regressions into focused child modules before either
parent file approached the 800-line Rust limit. The combined focused feedback
suite passes 10 tests, client clippy passes, and the Rust file-size check now
reports the largest affected parent at 779 lines (`src/network/tests.rs`). No
publisher run was needed because this was test organization only, and no new
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
`DB_DRIVER=json` for their temporary workers, isolate their state files, and
restore every process environment value they override on exit. Each affected
script passes under intentionally wrong ambient MySQL and timing settings,
preventing shell-level configuration from redirecting or contaminating
historical acceptance fixtures. No new external or deferred work was opened.

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

MySQL startup now fails fast when the canonical `DB_USERNAME` or `DB_PASSWORD`
configuration is empty, before attempting pool creation or migrations. The
focused `mysql_backend_requires_` filter passes (3 matching tests), with
server-package clippy, formatting, diff, and Rust file-size checks clean.
`publish.ps1` passed the Windows and WebGL release builds, packaging, Preview
deployment, and catalog synchronization. No full workspace gate was repeated
because this was an isolated MySQL configuration-boundary check; the existing
preview database was not changed.

The MySQL password preflight now uses the same whitespace-only rejection as
the acceptance script, while preserving non-empty password bytes for the
driver. The focused `mysql_backend_` filter passes (4 matching tests), with
server-package clippy, formatting, diff, and Rust file-size checks clean.
`publish.ps1` passed the Windows and WebGL release builds, packaging, Preview
deployment, and catalog synchronization. No full workspace gate was repeated
because this was a one-condition consistency fix at the existing configuration
boundary.

The client now pauses authenticated projection reads as soon as production
session rotation is pending: core state/events, trades, and frontier contracts,
chronicle, and opportunities cannot be started with the old bearer token in
the same frame. The focused
`authenticated_reads_wait_for_a_same_frame_refresh_boundary` regression passes
(1 matching test), with client-package clippy, formatting, diff, and Rust
file-size checks clean. `publish.ps1` passed the Windows and WebGL release
builds, packaging, Preview deployment, and catalog synchronization. No full
workspace gate was repeated because this was an isolated client session-boundary
fix.

The target MySQL acceptance gate was rechecked without touching the configured
world. The preview credentials still pass password-safe `SELECT 1`;
`SHOW GRANTS` confirms `ALL PRIVILEGES` on the configured `tarrowyn` schema but
no server-level `CREATE` or `DROP` grant. The configured account therefore
cannot create a uniquely named disposable database (MySQL `ERROR 1044`), and
no existing database was reused because the available schemas belong to other
applications. The full migration/restart/replay/backup/restore script
therefore remains unrun against a clean MySQL schema; the deployment gate now
explicitly requires disposable-schema provisioning permission and an operator-
supplied clean snapshot. The verifier now performs that create/drop privilege
probe before launching the server and reports the operator action directly.

The Phase 6 broad integration fixtures now live in a dedicated child test
module, leaving the parent focused on test-module wiring and keeping every Rust
test file below the 800-line project limit. The focused server Phase 6 test
filter passes (55 tests), with formatting and diff checks clean. No full
workspace or publisher gate was repeated because
this was test-organization-only maintenance, and no new external or deferred
work was opened.

The online combat status now names the active weapon beside the server-owned
action window, making the visible Local fight weapon cycle—including the
improvised club—readable after the transient prompt has gone. The focused
online UI tests pass (7), client-package clippy, formatting, diff, and Rust
file-size checks pass, and `publish.ps1` passes its Windows/WebGL builds,
packaging, Preview deployment, and catalog synchronization. No full workspace
gate was repeated because this was a bounded client presentation slice, and no
new external or deferred work was opened.

The touch movement pad now uses font-safe directional symbols (`^`, `<`, `>`,
`v`) instead of keyboard-looking `U/L/R/D` labels. The captured gameplay
verification image renders all four controls correctly; the focused online UI
tests (7), client-package clippy, formatting, diff, and Rust file-size checks
pass, and `publish.ps1` refreshed the Windows/WebGL packages and Preview
artifacts. No full workspace gate was repeated because this was a bounded
touch-presentation slice, and no new external or deferred work was opened.

The visible quick-chat control now says `Meet`, matching the “Meet at the
Hearth” message it sends and the GDD's intended social regrouping cue. The
focused online UI suite (7) and client-package clippy pass; the gameplay
capture confirms the label in the rendered touch layout, and `publish.ps1`
passes Windows/WebGL builds, packaging, Preview deployment, and catalog sync.
No full workspace gate was repeated because this was a bounded social-copy
presentation slice, and no new external or deferred work was opened.

The online footer now exposes the compact authoritative wallet and inventory
ledger beside visible-player count, so the persistent online character state is
not hidden behind transient command notices. The focused footer-formatting
regression passes (1), client-package clippy, formatting, diff, and Rust
file-size checks pass, and the captured 1280x720 gameplay layout remains
readable. `publish.ps1` passes Windows/WebGL builds, packaging, Preview
deployment, and catalog sync. No full workspace gate was repeated because this
was a bounded online-ledger presentation slice, and no new external or
deferred work was opened.

The skill panel now names advanced arts whose server-owned state has reached
resonance or discovery, while continuing to omit unrevealed recipes and direct
practice controls for merged skills. The focused advanced-skill presentation
regression passes (1), client-package clippy, formatting, diff, and Rust
file-size checks pass. `publish.ps1` passes Windows/WebGL builds, packaging,
Preview deployment, and catalog sync. No full workspace gate was repeated
because this was a bounded progression-presentation slice, and no new external
or deferred work was opened.

The chat placeholder now names the visible `Meet` quick-chat control and its
Hearth call, satisfying the touch prompt rule without changing the message
payload. The focused online UI suite passes (8), the captured 1280x720 gameplay
layout remains readable, and `publish.ps1` passes Windows/WebGL builds,
packaging, Preview deployment, and catalog sync. No full workspace gate was
repeated because this was a bounded touch-copy slice, and no new external or
deferred work was opened.

Knockout guidance now names the visible `Self`, `Rescuer`, and `Healer` recovery
controls in authoritative farming, combat, travel, and client ledger prompts.
The focused recovery server filter passes (10), the changed server package
clippy check and client online UI suite (8) pass, and the captured 1280x720
layout remains readable. `publish.ps1` passes Windows/WebGL builds, packaging,
Preview deployment, and catalog sync. No full workspace gate was repeated
because this was a bounded recovery-guidance slice, and no new external or
deferred work was opened.

The combat cooldown prompt now names the visible `Strike`, `Technique`, `Guard`,
`Bandage`, and `Retreat` controls instead of asking the player to choose an
unspecified combat action. The focused server timing regression passes (1),
formatting and diff checks are clean, and `publish.ps1` passes Windows/WebGL
builds, packaging, Preview deployment, and catalog sync. No full workspace gate
was repeated because this was a bounded combat-guidance slice, and no new
external or deferred work was opened.

The remaining Brambleback and local-combat knockout notices now name the same
visible `Self`, `Rescuer`, and `Healer` recovery controls, including the client
fallback when an older response omits its prompt. The focused recovery server
filter passes (10), the client online UI suite passes (8), and `publish.ps1`
passes Windows/WebGL builds, packaging, Preview deployment, and catalog sync.
No full workspace gate was repeated because this was a bounded recovery-copy
consistency slice, and no new external or deferred work was opened.

The PowerShell content gate now validates the skill manifest's supported
families, depth bounds, root practice paths, prerequisite references and
duplicates, advanced qualifying requirements, and practice-key references in
addition to its existing text and ID checks. The focused `validate_content.ps1`
run passes against the current catalogue, with no Rust or workspace-wide test
gate repeated because this was a release-content validation slice.

The same content gate now protects every named launch root from the GDD and
the initial Weapon Fighting and Storm Magic discovery IDs, while still allowing
future skills to be added. The focused `validate_content.ps1` run passes with
the complete current catalogue; no Rust or workspace-wide gate was repeated
because this was a stable-content-ID check.

The skill content gate now also rejects prerequisite cycles using the same
bounded graph rule enforced by server startup, keeping a malformed merger graph
from reaching a release package. The focused `validate_content.ps1` run passes
against the current catalogue; no Rust or workspace-wide gate was repeated
because this was release-script-only validation.

The online footer now surfaces the first pending direct trade's direction,
neighbour, and exact offered/requested goods alongside the wallet, inventory,
and presence ledger; terminal trade history remains out of the attention line.
The two focused footer regressions pass, client clippy and the Rust file-size
check pass, and the captured 1280x720 layout remains readable with no pending
trade. `publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment,
and catalog sync. No full workspace gate was repeated because this was a
bounded trade-inspection presentation slice, and no new external or deferred
work was opened.

The online farming client now chooses the nearest plot whose projected crop
state matches the requested action: empty for planting, growing for tending,
and mature for harvesting. This prevents an adjacent plot in an unrelated
state from consuming the action attempt, while the authoritative server still
rejects stale projections. The focused action-target regression passes for all
three crop actions, client clippy and the Rust file-size check pass, and
`publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was repeated because this was a bounded
client farming-target slice, and no new external or deferred work was opened.

Accepted farming responses now give the player a compact authoritative result:
the crop kind and plot coordinates for planting, tending, or harvesting, or
Bellweather's resulting condition for animal care. The two focused notice
formatting regressions pass, client clippy and the Rust file-size check pass,
and `publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment,
and catalog sync. No full workspace gate was repeated because this was a
bounded client farming-feedback slice, and no new external or deferred work
was opened.

Accepted travel responses now state the authoritative journey outcome: current
destination, progress and risk while travelling, the visible `Recover` path
when interrupted, and the arrived location after completion. The two focused
travel-notice regressions pass, client clippy and the Rust file-size check pass,
and `publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment,
and catalog sync. No full workspace gate was repeated because this was a
bounded client travel-feedback slice, and no new external or deferred work was
opened.

Accepted regional market responses now retain the existing action-specific
message and add exact shipment details when present: quantity, commodity,
origin, destination, and gold total. The focused market-detail regression
passes, client clippy and the Rust file-size check pass, and `publish.ps1`
passes Windows/WebGL builds, packaging, Preview deployment, and catalog sync.
No full workspace gate was repeated because this was a bounded client
market-feedback slice, and no new external or deferred work was opened.

Accepted contract responses now explain the updated Brambleback watch state:
active progress shows its count, while a completed report confirms payment and
the next available beat. The two focused contract-notice regressions pass,
client clippy and the Rust file-size check pass, and `publish.ps1` passes
Windows/WebGL builds, packaging, Preview deployment, and catalog sync. No full
workspace gate was repeated because this was a bounded client contract-feedback
slice, and no new external or deferred work was opened.

Accepted route responses now state the resulting road condition in the visible
notice: its player-facing status, condition percentage, and risk percentage.
The focused route-detail regression passes, client clippy and the Rust
file-size check pass, and `publish.ps1` passes Windows/WebGL builds, packaging,
Preview deployment, and catalog sync. No full workspace gate was repeated
because this was a bounded client route-feedback slice, and no new external or
deferred work was opened.

Accepted regional event responses now name the authoritative lifecycle result:
the signal or escalation state, the selected intervention, or the resolution
outcome. The focused event-detail regression passes, client clippy and the Rust
file-size check pass, and `publish.ps1` passes Windows/WebGL builds, packaging,
Preview deployment, and catalog sync. No full workspace gate was repeated
because this was a bounded client event-feedback slice, and no new external or
deferred work was opened.

Accepted Registry responses now explain the authoritative lease result: the
plot coordinates and building-access duration for an active lease, the
approval handoff for a request, the receiving resident for a transfer or
inheritance, and the visible reclaim path for abandoned, expired, or reclaimed
land. The focused claim-success-message regression passes, client clippy and
the Rust file-size check pass, and `publish.ps1` passes Windows/WebGL builds,
packaging, Preview deployment, and catalog sync. No full workspace gate was
repeated because this was a bounded client Registry-feedback slice, and no
new external or deferred work was opened.

The completed interaction-feedback milestone received its full workspace gate.
The first run exposed an outdated farming backpressure fixture that supplied
fields without a growing crop for a Tend action; adding that valid crop kept
the test aligned with the action-aware selector. The focused regression then
passed, followed by `cargo test --workspace` with 15 protocol, 416 server, and
133 client tests plus asset-registry, code-size, and doc tests all passing;
`cargo clippy --workspace --all-targets --all-features -- -D warnings` also
passed. This is the first full gate in the current feedback sweep; no new
external or deferred work was opened.

Accepted movement responses now name the authoritative destination tile
instead of only saying that the step was accepted. The focused movement-notice
regression passes, client clippy and the Rust file-size check pass, and
`publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was repeated because this was a bounded
client movement-feedback slice, and no new external or deferred work was
opened.

The Town hall Inspect response now names the authoritative public ledger
instead of only confirming that the action was recorded: filled offices,
proposals in progress, treasury, tax rate, and administration quality are
visible in the client notice. The focused governance-inspection regression
passes, client clippy and the Rust file-size check pass, and `publish.ps1`
passes Windows/WebGL builds, packaging, Preview deployment, and catalog sync.
No full workspace gate was repeated because this was a bounded client
governance-feedback slice, and no new external or deferred work was opened.

Accepted Knowledge discovery responses now name the recorded clue before its
effect, so a discovery notice identifies both the new knowledge and its first
use. The focused knowledge-discovery regression passes, client clippy and the
Rust file-size check pass, and `publish.ps1` passes Windows/WebGL builds,
packaging, Preview deployment, and catalog sync. No full workspace gate was
repeated because this was a bounded client knowledge-feedback slice, and no
new external or deferred work was opened.

Interrupted travel notices now preserve the authoritative interruption reason
before naming the visible Recover control, so a failed journey explains what
blocked it instead of only reporting that it stopped. The focused
interrupted-travel regression passes, client clippy and the Rust file-size
check pass, and `publish.ps1` passes Windows/WebGL builds, packaging, Preview
deployment, and catalog sync. No full workspace gate was repeated because
this was a bounded client travel-recovery-feedback slice, and no new external
or deferred work was opened.

Resumed travel notices now preserve the authoritative recovery note, such as
the route crew's safe continuation, alongside journey progress and risk. The
focused resumed-travel regression passes, client clippy and the Rust file-size
check pass, and `publish.ps1` passes Windows/WebGL builds, packaging, Preview
deployment, and catalog sync. No full workspace gate was repeated because
this was a bounded client travel-recovery-feedback slice, and no new external
or deferred work was opened.

Accepted moderation report responses now expose the authoritative report
reference and queued status, giving the player something concrete to retain
after submitting a report. The focused moderation-feedback regression passes,
client clippy and the Rust file-size check pass, and `publish.ps1` passes
Windows/WebGL builds, packaging, Preview deployment, and catalog sync. No
full workspace gate was repeated because this was a bounded client
moderation-feedback slice, and no new external or deferred work was opened.

The Chronicle control now opens a touch-friendly archive panel with the
authoritative archive range, latest highlight, and recent community records;
the panel can be closed visibly without keyboard input. The focused chronicle
panel formatter regression passes, client clippy and the Rust file-size check
pass, and `publish.ps1` passes Windows/WebGL builds, packaging, Preview
deployment, and catalog sync. No full workspace gate was repeated because
this was a bounded client chronicle-presentation slice. Typed archive search
remains a documented follow-up if the player-facing archive needs searching
older than the cached recent window.

The Chronicle panel now submits its visible query to the authenticated archive
search endpoint, displays bounded returned records and the matching archive
range, and keeps an all-history search reachable through a visible Search
control. Focused query-encoding, frontier-queue, and no-match presentation
regressions pass, client clippy and the Rust file-size check pass, and
`publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was repeated because this was a bounded
client chronicle-search slice. True touch text entry and older-result
pagination remain in the follow-up register.

Chronicle archive search now advances through a full returned page with a
visible Next control, preserving the server cursor and query between requests.
The focused pagination queue regression, query-encoding regression, and
no-match presentation regression pass, client clippy and the Rust file-size
check pass, and `publish.ps1` passes Windows/WebGL builds, packaging, Preview
deployment, and catalog sync. No full workspace gate was repeated because
this was a bounded client chronicle-pagination slice. True touch text entry
for arbitrary queries remains in the follow-up register.

The Chronicle panel now includes a visible touch keyboard for arbitrary query
entry, plus Space, Delete, and Clear controls; physical keyboard input remains
an optional supplement. The focused chronicle query-edit regression and
chronicle search presentation regression pass, client clippy and the Rust
file-size check pass, and `publish.ps1` passes Windows/WebGL builds, packaging,
Preview deployment, and catalog sync. No full workspace gate was repeated
because this was a bounded client touch-input slice. The follow-up register
now leaves only playtest evidence for the search surface.

Linked players can now open a read-only Account and character panel from the
visible Account control. It names provider, account and character boundaries,
character status, session expiry, privacy policy, and retention without
exposing access or refresh credentials; guest players retain the Account link
action. The focused account-summary secret-safety regression passes, client
clippy and the Rust file-size check pass, and `publish.ps1` passes Windows/WebGL
builds, packaging, Preview deployment, and catalog sync. No full workspace gate
was repeated because this was a bounded client account-transparency slice. The
follow-up register records only identity-surface playtest evidence.

The account-link guard now stays closed during the short interval after a
successful link when the production refresh token exists but the account
projection has not returned, preventing a duplicate link request. The focused
account lifecycle regression and existing linked-account guard pass, client
clippy and the Rust file-size check pass, and `publish.ps1` passes Windows/WebGL
builds, packaging, Preview deployment, and catalog sync. No full workspace gate
was repeated because this was a bounded account-lifecycle correction.

The Phase 6 player-facing readiness cluster reached its milestone gate on
2026-08-30: `cargo test --workspace` passed 15 protocol tests, 416 server
tests, 144 client tests, the asset registry check, the Rust file-size check,
and all doc tests; `cargo clippy --workspace --all-targets --all-features --
-D warnings` also passed. The focused chronicle and account checks and the
publisher had already passed for the component slices. No target-environment
gate was claimed by this local milestone run.

Chronicle archive search now builds its matching summary from the same bounded
result page returned to the client, including matches that are still in the
recent window rather than only archived records. The focused server
chronicle-search suite passes all four tests, server clippy passes, and
`publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was repeated because this was a bounded
server chronicle-search correction, and no new external or deferred work was
opened.

Chronicle archive search now exposes a continuation cursor only when a
sentinel result proves another bounded page exists, so a short result set no
longer shows a misleading Next control or needs an empty follow-up request.
The focused server chronicle-search suite passes all four tests, server clippy
passes, and `publish.ps1` passes Windows/WebGL builds, packaging, Preview
deployment, and catalog sync. No full workspace gate was repeated because
this was a bounded server chronicle-pagination correction, and no new
external or deferred work was opened.

Rejected account-deletion responses now always leave a visible warning, even
when the optional server reason is absent. The focused client regression,
client clippy, and Rust file-size check pass, and `publish.ps1` passes
Windows/WebGL builds, packaging, Preview deployment, and catalog sync. No full
workspace gate was repeated because this was a bounded account-feedback
correction, and no new external or deferred work was opened.

Online recovery controls now remain actionable when the account, skill, or
chronicle modal is open: the modal action filters preserve `Reconnect` and
`Offline fixture` while continuing to reject unrelated sidebar actions. The
focused `ui::tests::modal_filters_keep_recovery_controls_touchable` test
passes; client-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. The project `publish.ps1` Windows/WebGL build, packaging,
Preview deployment, and catalog-sync checks pass. No full workspace gate was
repeated because this was a bounded touch-recovery action-filter correction,
and no new external or deferred work was opened.

Registry, Abandon, and Transfer controls now close while a lease lifecycle
command is queued or in flight, and direct duplicate claim mutations are
rejected until the first response resolves. The focused lease-control test
passes, client clippy, standards, and the Rust file-size check pass, and
`publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was repeated because this was a bounded
claim-control correction, and no new external or deferred work was opened.

Logout and account deletion controls now close while a link, revoke, or delete
command is queued or in flight, and direct duplicate identity queue attempts
are rejected. The account-focused filter passes 24 related tests; command
pending projections now live in `src/network/phase5/pending.rs`, keeping
`phase5.rs` below the 800-line hard limit. Client standards, clippy, file-size
checks, and `publish.ps1` pass Windows/WebGL builds, packaging, Preview
deployment, and catalog sync. No full workspace gate was repeated because this
was a bounded identity-control and file-ownership correction, and no new
external or deferred work was opened.

Travel Start, Interrupt, and Recover controls now close while a travel command
is queued or in flight, preventing duplicate journey mutations while the
regional projection catches up. The focused travel filter passes, client
clippy, standards, and the Rust file-size check pass, and `publish.ps1` passes
Windows/WebGL builds, packaging, Preview deployment, and catalog sync. No full
workspace gate was repeated because this was a bounded travel-control
correction, and no new external or deferred work was opened.

Route Repair, Escort road, and Improve road controls now close while a route
logistics command is queued or in flight, preventing duplicate work orders
before the authoritative route projection catches up. The focused route filter
passes, and the OnlineClient regional facade remains split into
`src/network/phase5/online.rs` so `phase5.rs` stays below the 800-line hard
limit; client standards, clippy, file-size checks, and `publish.ps1` pass for
the corrected layout. No full workspace gate was repeated because this was a
bounded route-control and file-ownership correction, and no new external or
deferred work was opened.

Regional Event and intervention controls now close while an event command is
queued or in flight, preventing a second event decision before the first
authoritative response arrives. The focused event filter passes, and the
regional OnlineClient facade was extracted into `src/network/phase5/online.rs`
after the standards gate found `phase5.rs` over the 800-line limit; the client
standards, clippy, file-size check, and `publish.ps1` all pass for the corrected
layout. No full workspace gate was repeated because this was a bounded event
control and file-ownership correction, and no new external or deferred work was
opened.

Market and Cancel controls now close while a market order command is queued or
in flight, preventing repeated taps from creating duplicate shipments or
submitting a second settlement for the same order before the authoritative
refresh arrives. The focused market filter covers the control helpers and
regional command projection, client clippy, standards, and the Rust file-size
check pass, and `publish.ps1` passes Windows/WebGL builds, packaging, Preview
deployment, and catalog sync. No full workspace gate was repeated because this
was a bounded market-control correction, and no new external or deferred work
was opened.

Knockout recovery choices now close as soon as one recovery request is queued
or in flight, while a failed request leaves the choices available again. The
focused recovery filter covers the UI state and frontier queue projection,
client clippy, standards, and the Rust file-size check pass, and `publish.ps1`
passes Windows/WebGL builds, packaging, Preview deployment, and catalog sync.
No full workspace gate was repeated because this was a bounded recovery-control
correction, and no new external or deferred work was opened.

The reconnect control now stays closed while the guest session is still
connecting, and the client rejects direct reconnect attempts while connecting
or already online, so repeated taps cannot discard an in-progress identity
request. The focused reconnect regressions pass, client clippy, standards, and
the Rust file-size check pass, and `publish.ps1` passes Windows/WebGL builds,
packaging, Preview deployment, and catalog sync. No full workspace gate was
repeated because this was a bounded connection-control correction, and no new
external or deferred work was opened.

The client now keeps a completed state response behind the maintenance gate
when the readiness probe has already reported an unavailable server, avoiding
a one-frame return to online dispatch while maintenance is active. The
focused maintenance suite passes all three tests, client clippy and the Rust
file-size check pass, and `publish.ps1` passes Windows/WebGL builds, packaging,
Preview deployment, and catalog sync. No full workspace gate was repeated
because this was a bounded client readiness correction, and no new external
or deferred work was opened.

Regional command rejection handling now leaves a visible fallback warning when
the server omits its optional reason, covering travel, routes, market, events,
and moderation commands. The focused client regression, client clippy, and
Rust file-size check pass, and `publish.ps1` passes Windows/WebGL builds,
packaging, Preview deployment, and catalog sync. No full workspace gate was
repeated because this was a bounded Phase 5 feedback correction, and no new
external or deferred work was opened.

The regional travel control now distinguishes an active journey from recovery
in progress: only `Travelling` exposes the enabled Interrupt action, while
`Recovering` is visibly unavailable until the authoritative journey advances.
The focused travel-selector regression, client clippy, and Rust file-size check
pass, and `publish.ps1` passes Windows/WebGL builds, packaging, Preview
deployment, and catalog sync. No full workspace gate was repeated because this
was a bounded Phase 5 recovery-control correction, and no new external or
deferred work was opened.

Legacy guest sessions now expire at the configured TTL tick instead of
surviving one additional server tick; this matches the exact boundary already
used by production sessions. The focused server session regression passes,
server clippy and the Rust file-size check pass, and `publish.ps1` passes
Windows/WebGL builds, packaging, Preview deployment, and catalog sync. No
full workspace gate was repeated because this was a bounded session-expiry
correction, and no new external or deferred work was opened.

Successful account linking now cancels any in-flight guest account projection
so a delayed pre-link response cannot overwrite the newly linked production
boundary. The focused client account-lifecycle regression passes, client
clippy and the Rust file-size check pass, and `publish.ps1` passes Windows/WebGL
builds, packaging, Preview deployment, and catalog sync. No full workspace
gate was repeated because this was a bounded account-projection ordering
correction, and no new external or deferred work was opened.

Account revocation now removes that account's cached refresh responses before
the revoke result is persisted, so replaying an old refresh request cannot
return credentials after logout. The focused server session-integrity
regression passes, server clippy and the Rust file-size check pass, and
`publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was repeated because this was a bounded
refresh-revocation correction, and no new external or deferred work was
opened.

Phase 4 command rejection handling now leaves a visible fallback warning when
the server omits its optional reason, covering governance, claims, professions,
knowledge, combat, and skill actions. The focused client feedback regression
passes, client clippy and the Rust file-size check pass, and `publish.ps1`
passes Windows/WebGL builds, packaging, Preview deployment, and catalog sync.
No full workspace gate was repeated because this was a bounded Phase 4
feedback correction, and no new external or deferred work was opened.

Frontier command rejection handling now leaves a visible fallback warning when
the server omits its optional reason, covering contracts, combat, recovery,
homestead claims, and pioneer actions. The focused client frontier-feedback
regression passes, client clippy and the Rust file-size check pass, and
`publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was repeated because this was a bounded
frontier feedback correction, and no new external or deferred work was
opened.

The remaining frontier rejection branches now also leave visible fallback
warnings: contract refusals identify the frontier contract, and pioneer
refusals identify the pioneer action when no server reason is supplied. The
focused client frontier rejection suite passes, client clippy and the Rust
file-size check pass, and `publish.ps1` passes Windows/WebGL builds, packaging,
Preview deployment, and catalog sync. No full workspace gate was repeated
because this was a bounded frontier-feedback completion, and no new external
or deferred work was opened.

Successful account linking now invalidates every in-flight regional projection
and clears cached guest regional data before the linked session refreshes. This
prevents delayed guest market, map, household, event, settlement, law, or
account reads from surviving the identity boundary, and schedules an immediate
authoritative reload. The focused client account-lifecycle regression passes,
client clippy and the Rust file-size check pass, and `publish.ps1` passes
Windows/WebGL builds, packaging, Preview deployment, and catalog sync. No full
workspace gate was repeated because this was a bounded regional identity-cache
correction, and no new external or deferred work was opened.

Successful account linking now invalidates in-flight Phase 4 ledgers and
clears cached guest skills, crafting state, ownership context, and queued
actions before the next linked-account refresh. The focused client Phase 4
account-lifecycle regression passes, client clippy and the Rust file-size check
pass, and `publish.ps1` passes Windows/WebGL builds, packaging, Preview
deployment, and catalog sync. No full workspace gate was repeated because this
was a bounded Phase 4 identity-cache correction, and no new external or
deferred work was opened.

The online client now treats the linked account handoff as a world-projection
boundary: stale state, event, trade, movement, chat, farming, and trade-action
requests are canceled, guest trade and frontier caches are cleared, and the
authoritative state refresh restarts immediately. The focused client online
handoff regression passes, client clippy and the Rust file-size check pass, and
`publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was repeated because this was a bounded
online identity-handoff correction, and no new external or deferred work was
opened.

Keyboard gameplay input now pauses behind account, regional-inspection, and
skill-selection panels, matching the existing touch-action filtering; the
chronicle panel retains its dedicated query-entry path. The focused client
modal-input regression passes, client clippy and the Rust file-size check pass,
and `publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment,
and catalog sync. No full workspace gate was repeated because this was a
bounded touch-first input correction, and no new external or deferred work was
opened.

Accepted frontier recovery responses now update the client map position from
the returned player projection, immediately placing a recovered traveller at
the authoritative Hearth tile instead of leaving the map on the knockout
location until a later refresh. The focused client recovery-position regression
passes, client clippy and the Rust file-size check pass, and `publish.ps1`
passes Windows/WebGL builds, packaging, Preview deployment, and catalog sync.
No full workspace gate was repeated because this was a bounded recovery
projection correction, and no new external or deferred work was opened.

Recovery now also invalidates a stale Phase 4 knockout combat cache when the
authoritative player projection returns recovered. This removes the old
risk/knocked-out controls and causes a fresh local-combat read before the next
Phase 4 dispatch. The focused client Phase 4 recovery regression passes, client
clippy and the Rust file-size check pass, and `publish.ps1` passes
Windows/WebGL builds, packaging, Preview deployment, and catalog sync. No full
workspace gate was repeated because this was a bounded recovery control-cache
correction, and no new external or deferred work was opened.

The online sidebar now offers frontier Retreat only when the active threat is
within the server's two-tile reach, so a recovered traveller at the Hearth no
longer receives an immediately invalid retreat control. The focused client UI
reachability regression passes for near, far, and quiet-threat states, client
clippy and the Rust file-size check pass, and `publish.ps1` passes Windows/WebGL
builds, packaging, Preview deployment, and catalog sync. No full workspace gate
was repeated because this was a bounded touch-control correction, and no new
external or deferred work was opened.

The client now suppresses the independent Phase 4 local-combat cache while a
player projection is knocked out, preventing an old engaged encounter from
appearing above the recovery choices. The cache is cleared before polling and
local-combat refresh is held until recovery restores control. Focused client
recovery-cache regressions pass, client clippy and the Rust file-size check
pass, and `publish.ps1` passes Windows/WebGL builds, packaging, Preview
deployment, and catalog sync. No full workspace gate was repeated because this
was a bounded recovery-state synchronization correction, and no new external
or deferred work was opened.

Accepted recovery now emits an authoritative online presence event at the
Hearth position after moving the identity, so other connected clients can
update the traveller's location through the normal event stream instead of
waiting for an unrelated snapshot or movement. The focused server recovery
stream regression passes, server clippy and the Rust file-size check pass, and
`publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was repeated because this was a bounded
recovery presence correction, and no new external or deferred work was opened.

Both frontier and local-combat knockouts now broadcast an online Hearth
presence event after moving the identity, so connected clients see the defeat
location change through the normal event stream as well as the command
response. Focused server regressions cover both defeat paths, server clippy and
the Rust file-size check pass, and `publish.ps1` passes Windows/WebGL builds,
packaging, Preview deployment, and catalog sync. No full workspace gate was
repeated because this was a bounded defeat presence correction, and no new
external or deferred work was opened.


The regional event stream now carries the traveller's move for automatic
arrival and support-cleared travel, using the active-session state instead of
assuming every repaired identity is online. Focused Phase 5 and Phase 6 server
regressions pass, server clippy and the Rust file-size check pass, and
`publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was repeated because this was a bounded
regional presence correction, and no new external or deferred work was opened.

The client now follows authoritative presence and movement changes in its
cached regional location label immediately, keeping travel controls and
regional telemetry aligned before the next scheduled region refresh. The
focused client location-sync regression passes, client clippy and the Rust
file-size check pass, and `publish.ps1` passes Windows/WebGL builds, packaging,
Preview deployment, and catalog sync. No full workspace gate was repeated
because this was a bounded regional projection correction, and no new external
or deferred work was opened.

The regional location cache now waits for an authoritative state, movement,
presence, combat, or recovery position instead of projecting the placeholder
Hearth coordinate during initial loading or carrying the previous account's
position across an identity link. Focused client authority and identity-handoff
regressions pass, client clippy, standards, and the Rust file-size check pass,
and `publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment,
and catalog sync. No full workspace gate was repeated because this was a
bounded regional projection authority correction, and no new external or
deferred work was opened.

The online combat action bar now disables local encounter actions until an
authoritative encounter is engaged and its server action window is open. This
removes misleading Strike, Technique, Guard, Bandage, Reposition, and Spell
requests while preserving Contract, reachable frontier retreat, and visible
recovery controls. The focused UI action-window regression passes, client
clippy, standards, and the Rust file-size check pass, and `publish.ps1` passes
Windows/WebGL builds, packaging, Preview deployment, and catalog sync. No full
workspace gate was repeated because this was a bounded combat presentation
correction, and no new external or deferred work was opened.

The regional journey lock now reaches every movement path: map taps and the
arrow pad become visibly unavailable during travel, interrupted recovery, and
active recovery, with an on-map instruction to use the visible travel control;
the keyboard supplement also stops before queuing a server-rejected step. The
focused regional travel-lock and UI regressions pass, client clippy, standards,
and the Rust file-size check pass, and `publish.ps1` passes Windows/WebGL
builds, packaging, Preview deployment, and catalog sync. No full workspace gate
was repeated because this was a bounded travel-input presentation correction,
and no new external or deferred work was opened.

Visible walking controls now require an Online connection for the shared road,
while the explicitly local fixture remains walkable. Direction-pad taps and map
targets no longer advertise movement during Connecting, Degraded, or Offline
online-client states when the request layer would silently discard them. The
focused binary test
`ui::ui_online::tests::walking_controls_wait_for_an_authoritative_connection`
passes; client-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. The project `publish.ps1` Windows/WebGL build, packaging,
Preview deployment, and catalog-sync checks pass. No full workspace gate was
repeated because this was a bounded touch-control availability correction, and
no new external or deferred work was opened.

The guest Account link control now closes as soon as a link command is queued
or in flight, preventing repeated taps from creating duplicate link requests
while the production identity boundary is resolving. The existing focused
account-link regression now covers the queued duplicate attempt, client
clippy, standards, and the Rust file-size check pass, and `publish.ps1` passes
Windows/WebGL builds, packaging, Preview deployment, and catalog sync. No full
workspace gate was repeated because this was a bounded identity-control
correction, and no new external or deferred work was opened.

Town Hall and Tax controls now close together while any governance command is
queued or in flight, and the client rejects a second governance cycle against
the stale settlement ledger. The focused governance filter passes four tests,
client formatting, clippy, standards, and the Rust file-size check pass, and
`publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was repeated because this was a bounded
governance-control correction, and no new external or deferred work was opened.

Trade, Accept, and Cancel controls now close while a trade command is queued
or in flight, and same-target Create/Review/Accept/Cancel requests no longer
duplicate the pending ledger action while preserving distinct queued trades.
The focused trade filter passes nine related tests; the trade tests were split
into `src/network/tests/trades.rs` to keep every Rust test file below 800 lines.
Client formatting, clippy, standards, and the Rust file-size check pass, and
`publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was repeated because this was a bounded
trade-control and test-ownership correction, and no new external or deferred
work was opened.

Practice and School controls now close while a skill command is queued or in
flight, and the discipline picker disables its choices while the skill ledger
settles. The focused Practice filter passes the two affected Phase 4 tests,
with the UI skill-control regression included; client formatting, clippy,
standards, and the Rust file-size check pass, and `publish.ps1` passes
Windows/WebGL builds, packaging, Preview deployment, and catalog sync. No full
workspace gate was repeated because this was a bounded skill-control
correction, and no new external or deferred work was opened.

Local-fight and local combat action controls now close while a Phase 4 combat
command is queued or in flight, preventing repeated Prepare or action requests
before the encounter projection catches up. The focused combat filter passes
seven related tests, including the existing authoritative action-window
checks; client formatting, clippy, standards, and the Rust file-size check
pass, and `publish.ps1` passes Windows/WebGL builds, packaging, Preview
deployment, and catalog sync. No full workspace gate was repeated because
this was a bounded local-combat-control correction, and no new external or
deferred work was opened.

Plant, Tend, Harvest, and Care controls now close while a farming request is
queued or in flight, while full farming queues still report the established
backpressure message. The focused farming filter passes six related tests;
client formatting, clippy, standards, and the Rust file-size check pass, and
`publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was repeated because this was a bounded
farming-control correction, and no new external or deferred work was opened.

The tavern Contract control now closes while a frontier contract command is
queued or in flight, and direct cycle attempts retain the established cooldown
and full-queue feedback. The focused contract filter passes seven related
tests; client formatting, clippy, standards, and the Rust file-size check pass,
and `publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment,
and catalog sync. No full workspace gate was repeated because this was a
bounded contract-control correction, and no new external or deferred work was
opened.

The Pioneer control now closes while an expedition command is queued or in
flight, preventing repeated Join, Supply, Launch, Resolve, or Announce choices
from being derived from the same stale expedition projection. The focused
expedition filter passes six related tests; client formatting, clippy,
standards, and the Rust file-size check pass, and `publish.ps1` passes
Windows/WebGL builds, packaging, Preview deployment, and catalog sync. No full
workspace gate was repeated because this was a bounded expedition-control
correction, and no new external or deferred work was opened.

Frontier reachable-threat retreat now closes while a frontier combat command is
queued or in flight, preventing duplicate retreat requests while the wilderness
projection catches up. The focused `frontier_combat` filter passes two tests;
the frontier OnlineClient facade was split into
`src/network/frontier/online.rs` after the standards gate found `frontier.rs`
over the 800-line hard limit. Client formatting, clippy, standards, and the
Rust file-size check pass, and `publish.ps1` passes Windows/WebGL builds,
packaging, Preview deployment, and catalog sync. No full workspace gate was
repeated because this was a bounded frontier-combat control and file-ownership
correction, and no new external or deferred work was opened.

The Knowledge control now closes while a knowledge command is queued or in
flight, preventing duplicate Discover, Record, Teach, or Apply requests while
the archive projection catches up. The focused `knowledge` filter passes four
related tests; client formatting, clippy, standards, and the Rust file-size
check pass, and `publish.ps1` passes Windows/WebGL builds, packaging, Preview
deployment, and catalog sync. No full workspace gate was repeated because this
was a bounded knowledge-control correction, and no new external or deferred
work was opened.

The Order control now closes while a Profession command is queued or in flight,
and also closes while its timing challenge is active, preventing duplicate
service-order requests from a stale professions ledger or a live crafting
interaction. The focused order-control filter passes two related tests; client
formatting, clippy, standards, and the Rust file-size check pass, and
`publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was repeated because this was a bounded
service-order control correction, and no new external or deferred work was
opened.

The Report control now closes while a moderation report is queued or in flight,
preventing duplicate submissions from the same visible chat or player evidence
while the regional projection catches up. The focused report-control filter
passes two related tests; client formatting, clippy, standards, and the Rust
file-size check pass, and `publish.ps1` passes Windows/WebGL builds, packaging,
Preview deployment, and catalog sync. No full workspace gate was repeated
because this was a bounded moderation-control correction, and no new external
or deferred work was opened.

The Frontier Claim control now closes while a claim request is queued or in
flight, preventing duplicate Request or Renew actions while the frontier lease
projection catches up. The focused `frontier_claim_controls` filter passes two
related tests; client formatting, clippy, standards, and the Rust file-size
check pass, and `publish.ps1` passes Windows/WebGL builds, packaging, Preview
deployment, and catalog sync. No full workspace gate was repeated because this
was a bounded frontier-claim control correction, and no new external or
deferred work was opened.

Frontier recovery now accepts only one Self, Rescuer, or Healer choice while a
recovery command is queued or in flight, preventing competing recovery requests
from the same knocked-out projection. The focused recovery-control filter
passes one related test; client formatting, clippy, standards, and the Rust
file-size check pass, and `publish.ps1` passes Windows/WebGL builds, packaging,
Preview deployment, and catalog sync. No full workspace gate was repeated
because this was a bounded frontier-recovery queue correction, and no new
external or deferred work was opened.

The Market queue now accepts only one market-region or cancellation request
while a Market command is queued or in flight, preventing duplicate fulfilment,
creation, or cancellation actions from a stale regional market projection. The
focused `market_controls_wait` filter passes two related tests; client
formatting, clippy, standards, and the Rust file-size check pass, and
`publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was repeated because this was a bounded
regional-market queue correction, and no new external or deferred work was
opened.

Regional events now accept only one cycle or selected intervention while an
Event command is queued or in flight, preventing duplicate resolutions from a
stale event projection. The focused `event_controls_wait` filter passes two
related tests; client formatting, clippy, standards, and the Rust file-size
check pass, and `publish.ps1` passes Windows/WebGL builds, packaging, Preview
deployment, and catalog sync. No full workspace gate was repeated because this
was a bounded regional-event queue correction, and no new external or deferred
work was opened.

Regional route actions now accept only one Repair, Escort, or Improve command
while a Route command is queued or in flight, preventing duplicate logistics
requests from a stale road projection. The focused `route_controls_wait`
filter passes two related tests; client formatting, clippy, standards, and the
Rust file-size check pass, and `publish.ps1` passes Windows/WebGL builds,
packaging, Preview deployment, and catalog sync. No full workspace gate was
repeated because this was a bounded regional-route queue correction, and no new
external or deferred work was opened.

Regional travel now accepts only one Start, Interrupt, or Recover command while
a Travel command is queued or in flight, preventing duplicate logistics
transitions from a stale journey projection. The focused
`travel_controls_wait` filter passes one related test; client formatting,
clippy, standards, and the Rust file-size check pass, and `publish.ps1` passes
Windows/WebGL builds, packaging, Preview deployment, and catalog sync. No full
workspace gate was repeated because this was a bounded regional-travel queue
correction, and no new external or deferred work was opened.

The implemented Phase 3–5 mutation surfaces now share queue-boundary protection
for stale projections: farming, trade, governance, skill, combat, frontier
contract, expedition, claim, and recovery, plus Phase 5 market, event, route,
travel, moderation, and Phase 4 knowledge and service-order controls. The major
command-hardening milestone passed the full workspace gate: 15 protocol tests,
419 server tests, 199 client tests, asset registry, code standards, doc tests,
and workspace clippy with warnings denied. The focused publisher checks for the
latest slices also pass; no new external or deferred work was opened.

General mutation queues now participate in the Phase 4 dispatch boundary:
queued movement, chat, farming, or trade work is treated like an in-flight
request before Phase 4 commands and regional refreshes are started. The focused
`queued_general_mutation_blocks_phase_four_dispatch_until_its_turn` filter
passes one regression test; client formatting, clippy, standards, and the Rust
file-size check pass, and `publish.ps1` passes Windows/WebGL builds, packaging,
Preview deployment, and catalog sync. No full workspace gate was repeated
because this was a bounded mutation-dispatch coordination correction, and no
new external or deferred work was opened.

Mutation dispatch now treats the queue boundary symmetrically: an in-flight
Phase 4 or regional command prevents later general, trade, or frontier
requests from starting, while older queued work retains priority and can still
be dispatched first. The focused coordinator filters pass three related tests
(`queued_general_mutation_blocks_phase_four_dispatch_until_its_turn`,
`queued_frontier_mutation_blocks_phase_four_dispatch_until_its_turn`, and
`in_flight_phase_four_mutation_blocks_later_general_and_frontier_dispatch`);
client formatting, clippy, standards, and the Rust file-size check pass, and
`publish.ps1` passes Windows/WebGL builds, packaging, Preview deployment, and
catalog sync. No full workspace gate was repeated because this was a bounded
cross-phase dispatch-order correction, and no new external or deferred work was
opened.

Moderation report replay lookup now precedes historical chat-evidence lookup
after the request fields are validated, so a retry keeps its accepted response
even if the referenced chat message has left the retained history window. The
focused `repository::phase6::tests::moderation_validation` filter passes five
tests, including the retention-era replay regression; workspace formatting and
`git diff --check` pass. No full workspace gate was repeated because this was
a bounded moderation idempotency correction, and no new external or deferred
work was opened.

Account deletion now keeps a bounded, token-fingerprinted terminal response so
a lost scheduled-deletion response can replay after the authoritative tick has
removed the identity and invalidated its session. The focused deletion queue
filter passes both pending coalescing and post-removal replay tests, and the
direct production-integrity filter passes eight tests; formatting, file-size,
and `publish.ps1` Windows/WebGL build, packaging, Preview deployment, and
catalog-sync checks pass. No full workspace gate was repeated because this was
a bounded deletion idempotency correction, and no new external or deferred
work was opened.

Session revocation now counts only production session records that are not
already revoked, so a refresh rotation followed by “revoke all” reports the
number of credentials actually transitioned instead of counting the retired
session again. The focused `repository::phase6::tests::session_integrity`
filter passes six tests; formatting, diff, Rust file-size, and `publish.ps1`
Windows/WebGL build, packaging, Preview deployment, and catalog-sync checks
pass. No full workspace gate was repeated because this was a bounded session
accounting correction, and no new external or deferred work was opened.

Coalesced account-deletion requests now retain their own token-fingerprinted
terminal replay response, so a second request ID received while deletion is
queued remains recoverable after the authoritative tick removes the identity.
The focused `repository::phase6::tests::deletion_queue` filter passes both
tests, including the post-removal replay of the coalesced request; formatting
and `git diff --check` pass. No full workspace gate was repeated because this
was a bounded deletion replay correction, and no new external or deferred
work was opened.

Chronicle paging now compares trimmed query values at the UI boundary, keeping
the visible `Next` control available when a touch-entered query ends with a
space and the server returns its canonical trimmed form. The focused binary
test `ui::ui_online::tests::chronicle_search_paging_survives_server_query_trimming`
passes; formatting, diff, and Rust file-size checks pass. No full workspace
gate was repeated because this was a bounded touch-first chronicle presentation
correction, and no new external or deferred work was opened.

Support account views now report an expiry only for an active, unrevoked
production access session still present in the authoritative session table;
refresh-window mirrors and revoked credentials no longer appear as usable
access. The focused `repository::phase6::tests::support_chronicle` filter
passes two tests, server-package formatting and diff checks pass, and the Rust
file-size scan is clean. No full workspace gate was repeated because this was
a bounded operator-read-model correction, and no new external or deferred
work was opened.

Operational metrics now sweep expired sessions before counting connected
sessions, so the support view cannot report a guest or production credential
past its tick deadline as still connected between world ticks. The focused
`repository::phase6::tests::operations_metrics` filter passes both tests;
server-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. No full workspace gate was repeated because this was a
bounded operator-session metrics correction, and no new external or deferred
work was opened.

Production session refresh now forces the client to request a fresh Account
projection immediately, so the visible session-expiry beat follows the newly
rotated credential instead of waiting for the periodic regional refresh. The
focused client lifecycle test
`network::phase5::tests::account_lifecycle::refreshed_session_requests_a_fresh_account_projection`
passes; client-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. No full workspace gate was repeated because this was a
bounded authenticated-session presentation correction, and no new external or
deferred work was opened.

Operational health now expires stale sessions before evaluating production
integrity, so a credential that crossed its access deadline between world
ticks does not falsely mark public readiness as degraded. The focused
`repository::phase6::tests::operations_metrics::operational_health_cleans_expired_sessions_before_checking_readiness`
test passes; server-package formatting and clippy, `git diff --check`, and the
Rust file-size scan pass. No full workspace gate was repeated because this was
a bounded operator-readiness session correction, and no new external or
deferred work was opened.

Coalesced account-deletion requests now persist their terminal replay mapping
before returning, so a second request ID remains replayable after a restart
and after the queued deletion removes the identity. The focused
`repository::phase6::tests::deletion_queue` filter passes three tests;
server-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. No full workspace gate was repeated because this was a
bounded deletion-replay durability correction, and no new external or
deferred work was opened.

Settlement facility rollups now refresh in every persisted snapshot, so claim
counts, available plots, and public works remain coherent after a mutation and
restart instead of waiting for the next presentation or world tick. The
focused `repository::phase5::tests::settlements` filter passes four tests;
server-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. No full workspace gate was repeated because this was a
bounded settlement-snapshot consistency correction, and no new external or
deferred work was opened.

Skills reads now persist expired or over-cap school-lesson pruning, so a lesson
removed from the authoritative view cannot return after a restart before the
next mutation or tick. The focused
`repository::phase4::tests::lesson_retention` filter passes two tests;
server-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. No full workspace gate was repeated because this was a
bounded skill-ledger retention correction, and no new external or deferred
work was opened.

Session expiry now persists its offline-presence event and cursor before any
request can return an authentication error, so a read that discovers an idle
credential cannot lose that world-history mutation on restart. The focused
`repository::session::tests` filter passes three tests; server-package
formatting and clippy, `git diff --check`, and the Rust file-size scan pass.
The project `publish.ps1` Windows/WebGL build, packaging, Preview deployment,
and catalog-sync checks pass. No full workspace gate was repeated because this
was a bounded session-expiry persistence correction, and no new external or
deferred work was opened.

All ordinary access-token endpoints now sweep expired sessions through the
same change-aware persistence boundary, including the Account, support,
chronicle-search, moderation, and support-repair surfaces that previously
authenticated directly. The focused `repository::session::tests` filter passes
three tests; server-package formatting and clippy, `git diff --check`, and the
Rust file-size scan pass. The project `publish.ps1` Windows/WebGL build,
packaging, Preview deployment, and catalog-sync checks pass. No full workspace
gate was repeated because this was a bounded access-endpoint session-boundary
correction, and no new external or deferred work was opened.

The client now returns a loaded world to Online when a later readiness poll
confirms that maintenance has cleared, resetting the state-refresh timer so
authoritative projections resume without forcing a needless manual Reconnect.
The pre-world case remains gated by the existing snapshot readiness decision.
The focused binary test `network::maintenance::tests` passes four tests;
client-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. The project `publish.ps1` Windows/WebGL build, packaging,
Preview deployment, and catalog-sync checks pass. No full workspace gate was
repeated because this was a bounded client readiness-recovery correction, and
no new external or deferred work was opened.

Clearing an online session now also clears the client’s loaded-world marker,
so a fresh reconnect that fails cannot claim to display a retained world and a
successful reconnect is treated as a first authoritative load. The focused
binary test `network::tests::session_reset_discards_world_and_frontier_projection_state`
passes; client-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. The project `publish.ps1` Windows/WebGL build, packaging,
Preview deployment, and catalog-sync checks pass. No full workspace gate was
repeated because this was a bounded client session-reset correction, and no
new external or deferred work was opened.

Cursor-restore recovery now clears the client’s loaded-world marker together
with cursor-derived projections, so a failed replacement snapshot is shown as
unavailable rather than as a degraded copy of cleared state. The focused binary
test `network::cursor::tests::restore_recovery_discards_stale_history_and_schedules_state_reload`
passes; client-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. The project `publish.ps1` Windows/WebGL build, packaging,
Preview deployment, and catalog-sync checks pass. No full workspace gate was
repeated because this was a bounded client cursor-recovery correction, and no
new external or deferred work was opened.

The map now draws the local character only from an authoritative player
position, except for the explicitly local offline fixture. Startup, cursor
reload, and failed snapshot states therefore no longer show a default or
cleared account marker as if it were current. The focused binary test
`ui::tests::map_player_marker_waits_for_authority_unless_using_the_offline_fixture`
passes; client-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. The project `publish.ps1` Windows/WebGL build, packaging,
Preview deployment, and catalog-sync checks pass. No full workspace gate was
repeated because this was a bounded client projection-presentation correction,
and no new external or deferred work was opened.

Direct production-session refresh now sweeps expired access credentials before
rotating a still-valid refresh session, so the boundary records the durable
offline-presence event that a direct refresh would otherwise skip. Replayed
refresh requests still return from the idempotency cache before the sweep. The
focused binary test
`repository::phase6::tests::session_integrity::direct_refresh_persists_presence_when_access_has_expired`
passes; server-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. The project `publish.ps1` Windows/WebGL build, packaging,
Preview deployment, and catalog-sync checks pass. No full workspace gate was
repeated because this was a bounded refresh-expiry persistence correction, and
no new external or deferred work was opened.

Guest-session resume now runs the shared expiry sweep before issuing a new
session, so reconnecting with an expired client key records the prior
character's durable offline-presence event instead of silently replacing its
token. The focused binary test
`repository::session::tests::guest_resume_records_departure_before_issuing_a_new_session`
passes; server-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. The project `publish.ps1` Windows/WebGL build, packaging,
Preview deployment, and catalog-sync checks pass. No full workspace gate was
repeated because this was a bounded guest-session expiry correction, and no
new external or deferred work was opened.

The online sidebar's “visible companions” count now excludes the local
character plus offline and stale presence records, matching the map's existing
stale-player treatment instead of counting historical entries as current
companions. The focused binary test
`ui::ui_online::tests::companion_count_ignores_own_stale_and_offline_presence`
passes; client-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. The project `publish.ps1` Windows/WebGL build, packaging,
Preview deployment, and catalog-sync checks pass. No full workspace gate was
repeated because this was a bounded social-presence presentation correction,
and no new external or deferred work was opened.

Normal online action buttons now wait for an authoritative player projection
after cursor or identity recovery, so cleared/default client state cannot
queue gameplay commands while the replacement snapshot is loading. Reconnect
and offline-fixture controls retain their dedicated availability rules. The
focused binary test
`ui::ui_online::tests::online_buttons_wait_for_authoritative_player_projection`
passes; client-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. The project `publish.ps1` Windows/WebGL build, packaging,
Preview deployment, and catalog-sync checks pass. No full workspace gate was
repeated because this was a bounded recovery-input presentation correction,
and no new external or deferred work was opened.

The online footer's total-player summary now uses the same active, non-stale
presence rule as the companion count, so cached departures no longer inflate
the population shown to a player. The focused binary test
`ui::ui_online::tests::companion_count_ignores_own_stale_and_offline_presence`
passes for both summaries; client-package formatting and clippy,
`git diff --check`, and the Rust file-size scan pass. The project
`publish.ps1` Windows/WebGL build, packaging, Preview deployment, and
catalog-sync checks pass. No full workspace gate was repeated because this was
a bounded social-presence presentation correction, and no new external or
deferred work was opened.

When a direct production refresh rotates an already-expired access session,
the server now records both sides of the presence transition: durable offline
departure from the expired credential followed by online presence for the
replacement session. Refreshes before access expiry do not emit a duplicate
presence event, and replayed refreshes still return their cached response. The
focused binary test
`repository::phase6::tests::session_integrity::direct_refresh_persists_presence_when_access_has_expired`
passes with the final account presence online; server-package formatting and
clippy, `git diff --check`, and the Rust file-size scan pass. The project
`publish.ps1` Windows/WebGL build, packaging, Preview deployment, and
catalog-sync checks pass. No full workspace gate was repeated because this was
a bounded refresh-presence continuity correction, and no new external or
deferred work was opened.

Map clicks, movement-pad taps, and direct movement requests now wait for an
authoritative player position after cursor or identity recovery instead of
using the cleared/default local position. The map tooltip names the loading or
visible reconnect/travel path, while the offline fixture keeps its local
movement behavior. The focused UI and network movement regressions plus the
existing disconnected and backpressure checks pass; client-package formatting
and clippy, `git diff --check`, and the Rust file-size scan pass. The project
`publish.ps1` Windows/WebGL build, packaging, Preview deployment, and
catalog-sync checks pass. No full workspace gate was repeated because this was
a bounded movement-recovery input correction, and no new external or deferred
work was opened.

Movement recovery messaging now prioritizes the connection state before the
cleared-position state, so a degraded client is told to tap the visible
Reconnect control even when its cached player position has already been
discarded. Online projection loading, knockout recovery, regional travel, and
ordinary walking retain distinct touch guidance. The focused tooltip state
matrix and movement-authority regression pass; client-package formatting and
clippy, `git diff --check`, and the Rust file-size scan pass. The project
`publish.ps1` Windows/WebGL build, packaging, Preview deployment, and
catalog-sync checks pass. No full workspace gate was repeated because this was
a bounded recovery-message correction, and no new external or deferred work
was opened.

The shared-road sidebar now uses the same state-specific movement guidance as
the map tooltip, so disabled movement controls no longer invite a tap that
cannot be accepted during connection, projection, knockout, or travel
recovery. The focused movement-guidance regression passes; client-package
formatting and clippy, `git diff --check`, and the Rust file-size scan pass.
The project `publish.ps1` Windows/WebGL build, packaging, Preview deployment,
and catalog-sync checks pass. No full workspace gate was repeated because this
was a bounded touch-guidance presentation correction, and no new external or
deferred work was opened.

Accepted current frontier contract responses now restore the authoritative
player-position marker along with the returned player projection. This keeps
walking controls closed only until a current server response supplies a real
position after cursor recovery. The focused
`network::frontier::tests::accepted_contract_response_restores_authoritative_player_position`
regression passes; client-package formatting and clippy, `git diff --check`,
and the Rust file-size scan pass. The project `publish.ps1` Windows/WebGL
build, packaging, Preview deployment, and catalog-sync checks pass. No full
workspace gate was repeated because this was a bounded frontier projection
consistency correction, and no new external or deferred work was opened.

Offline local-account presence events now update the companion roster without
marking the local position authoritative. A session-departure event therefore
cannot reopen movement controls with its last-known position while the client
is recovering or re-authenticating. The focused
`network::tests::location_projection::offline_presence_does_not_authorize_player_movement`
regression passes; client-package formatting and clippy, `git diff --check`,
and the Rust file-size scan pass. The project `publish.ps1` Windows/WebGL
build, packaging, Preview deployment, and catalog-sync checks pass. No full
workspace gate was repeated because this was a bounded presence-state
correction, and no new external or deferred work was opened.

An offline local-account presence now also revokes any previously authoritative
position, covering the online-to-offline transition rather than only a fresh
offline projection. The focused
`network::tests::location_projection::offline_presence_does_not_authorize_player_movement`
regression covers both directions and passes; client-package formatting and
clippy, `git diff --check`, and the Rust file-size scan pass. The project
`publish.ps1` Windows/WebGL build, packaging, Preview deployment, and
catalog-sync checks pass. No full workspace gate was repeated because this was
a bounded presence-transition correction, and no new external or deferred
work was opened.

Cursor recovery now clears the cached account character snapshot alongside
regional, reward, and history projections. The account panel therefore shows
its loading boundary until the scheduled authenticated account read returns,
instead of presenting stale position, gold, or status after a restored history
window. The focused
`network::phase5::tests::regional_events::regional_cursor_reset_discards_stale_events_and_restarts_refresh`
regression passes; client-package formatting and clippy, `git diff --check`,
and the Rust file-size scan pass. The project `publish.ps1` Windows/WebGL
build, packaging, Preview deployment, and catalog-sync checks pass. No full
workspace gate was repeated because this was a bounded stale-account-cache
correction, and no new external or deferred work was opened.

Global cursor recovery now cancels pending and queued movement, chat, farming,
and trade mutations, and clears their confirmation indicators before the
latest state is reloaded. A pre-restore response can no longer repopulate the
rebuilt projections or reopen movement from an obsolete request. The focused
`network::cursor::tests::restore_recovery_cancels_stale_low_level_mutations`
regression passes; client-package formatting and clippy, `git diff --check`,
and the Rust file-size scan pass. The project `publish.ps1` Windows/WebGL
build, packaging, Preview deployment, and catalog-sync checks pass. No full
workspace gate was repeated because this was a bounded cursor-recovery
mutation-cancellation correction, and no new external or deferred work was
opened.

Transport failure handling now cancels in-flight movement, chat, farming, and
trade requests as well as their queued follow-up work. Their pending UI
indicators are cleared at the same boundary, so a late response from the
degraded connection cannot mutate the recovered client. The focused
`network::tests::connection_recovery::connection_failure_discards_in_flight_low_level_requests_and_indicators`
regression passes; client-package formatting and clippy, `git diff --check`,
and the Rust file-size scan pass. The project `publish.ps1` Windows/WebGL
build, packaging, Preview deployment, and catalog-sync checks pass. No full
workspace gate was repeated because this was a bounded transport-recovery
correction, and no new external or deferred work was opened.

The Account link/details control remains available while the linked player's
authoritative position is being reloaded. Gameplay buttons still wait for that
position, but the session-level identity panel no longer becomes unreachable
after identity recovery clears the player projection. The focused
`ui::ui_online::tests::account_control_stays_available_during_player_projection_reload`
regression passes; client-package formatting and clippy, `git diff --check`,
and the Rust file-size scan pass. The project `publish.ps1` Windows/WebGL
build, packaging, Preview deployment, and catalog-sync checks pass. No full
workspace gate was repeated because this was a bounded account-recovery UI
correction, and no new external or deferred work was opened.

Healthy readiness responses now reopen only clients that were actually held by
the maintenance gate. A transport-degraded client remains in its explicit
Reconnect path instead of allowing an older pending request to resume merely
because a health response arrived later. The focused
`network::maintenance::tests` module passes all five tests; client-package
formatting and clippy, `git diff --check`, and the Rust file-size scan pass.
The project `publish.ps1` Windows/WebGL build, packaging, Preview deployment,
and catalog-sync checks pass. No full workspace gate was repeated because this
was a bounded readiness-recovery correction, and no new external or deferred
work was opened.

Logout and expiry now record one offline presence only after the last live
session for a character leaves. Authoritative world snapshots also collapse
multiple live sessions for the same character and retain the latest seen tick,
so multi-device sessions do not duplicate companions or make an active player
appear gone. The focused `repository::session::tests` module passes all eight
tests; server-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. The project `publish.ps1` Windows/WebGL build, packaging,
Preview deployment, and catalog-sync checks pass. No full workspace gate was
repeated because this was a bounded session-presence correction, and no new
external or deferred work was opened.

Older state snapshots are now rejected at the client readiness boundary and
immediately followed by a fresh authoritative state request. A stale response
can no longer mark a not-yet-loaded client as ready or postpone recovery while
leaving its previous projection in place. The focused
`network::tests::older_state_snapshot_is_reloaded_instead_of_opening_the_world`
test passes; client-package formatting and clippy, `git diff --check`, and the
Rust file-size scan pass. The project `publish.ps1` Windows/WebGL build,
packaging, Preview deployment, and catalog-sync checks pass. No full workspace
gate was repeated because this was a bounded state-ordering correction, and no
new external or deferred work was opened.

Transport reconnect now recognizes a linked production session, retains its
refresh credential in memory while clearing stale gameplay projections, and
rotates the production session before requesting any state. A linked character
can therefore return after a server restart without falling into guest login's
production-identity boundary; explicit logout and failed refresh still clear
the credential and use the fresh-guest recovery path. The focused
`network::tests::reconnect_rotates_a_linked_session_before_guest_fallback` test
passes; client-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. The project `publish.ps1` Windows/WebGL build, packaging,
Preview deployment, and catalog-sync checks pass. No full workspace gate was
repeated because this was a bounded production-session recovery correction,
and no new external or deferred work was opened.

Active JSON world snapshots now write and sync their temporary file before the
atomic replacement, matching the existing backup durability path. Failed writes
also remove their temporary artifact before reporting persistence failure. The
focused `repository::tests::persistence::relative_json_paths_write_state_and_backup_files`
test passes; server-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. The project `publish.ps1` Windows/WebGL build, packaging,
Preview deployment, and catalog-sync checks pass. No full workspace gate was
repeated because this was a bounded JSON persistence durability correction, and no
new external or deferred work was opened.

Knocked-out movement input now stops at the client recovery boundary instead of
sending a request that the server must reject. The status names the visible
recovery prompt as the next action. The focused
`network::tests::input_guards::knocked_out_input_waits_for_a_visible_recovery_prompt`
test passes; client-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. The project `publish.ps1` Windows/WebGL build, packaging,
Preview deployment, and catalog-sync checks pass. No full workspace gate was
repeated because this was a bounded knockout-input correction, and no new
external or deferred work was opened.

The unauthenticated production refresh boundary now rejects a persisted session
whose identity or account link has disappeared, instead of reaching an internal
assumption while the repository is already degraded. The focused
`repository::phase6::tests::session_integrity::refresh_rejects_a_session_with_a_missing_identity_without_panicking`
test passes; server-package formatting and clippy, `git diff --check`, and the
Rust file-size scan pass. The project `publish.ps1` Windows/WebGL build,
packaging, Preview deployment, and catalog-sync checks pass. No full workspace
gate was repeated because this was a bounded production-session integrity
correction, and no new external or deferred work was opened.

Authoritative local presence and movement updates now keep the cached player
projection's position synchronized with the map position, so account and combat
surfaces do not briefly describe the previous tile after a confirmed move. The
focused `network::tests::location_projection::authoritative_presence_keeps_player_projection_location_in_sync`
test passes; client-package formatting and clippy, `git diff --check`, and the
Rust file-size scan pass. The project `publish.ps1` Windows/WebGL build,
packaging, Preview deployment, and catalog-sync checks pass. No full workspace
gate was repeated because this was a bounded authoritative-position coherence
correction, and no new external or deferred work was opened.

Maintenance recovery now holds the client behind a fresh authoritative state
snapshot: it withdraws the cached position, keeps chat and gameplay mutations
from queuing or dispatching, and requests `/v1/state` before reopening the road.
The focused
`network::maintenance::tests::healthy_readiness_reopens_a_loaded_world_after_maintenance`
test passes; client-package formatting and clippy, `git diff --check`, and the
Rust file-size scan pass. The projection-version helpers were moved to
`src/network/world.rs` to keep every Rust file below 800 lines. The project
`publish.ps1` Windows/WebGL build, packaging, Preview deployment, and
catalog-sync checks pass. No full workspace gate was repeated because this was
a bounded maintenance-recovery authority correction, and no new external or
deferred work was opened.

Online modal action filters now preserve the visible `Reconnect` and `Offline
fixture` recovery controls while continuing to reject unrelated sidebar
actions. The focused
`ui::tests::modal_filters_keep_recovery_controls_touchable` test passes;
client-package formatting and clippy, `git diff --check`, and the Rust
file-size scan pass. The project `publish.ps1` Windows/WebGL build, packaging,
Preview deployment, and catalog-sync checks pass. No full workspace gate was
repeated because this was a bounded touch-recovery action-filter correction,
and no new external or deferred work was opened.

Cursor recovery and rejected older state snapshots now hold the client behind
the same authoritative reload gate as maintenance recovery. The cached
position is withdrawn and gameplay queues and subsystem dispatch remain closed
until a current `/v1/state` response is accepted, so a restore boundary cannot
invite actions against reset or stale projections. The focused
`network::cursor::tests::restore_recovery_discards_stale_history_and_schedules_state_reload`
test passes; client-package formatting and clippy, `git diff --check`, and the
Rust file-size scan pass. The project `publish.ps1` Windows/WebGL build,
packaging, Preview deployment, and catalog-sync checks pass. No full workspace
gate was repeated because this was a bounded cursor-recovery authority
correction, and no new external or deferred work was opened.

Development guest reset now emits the departing guest's offline presence
before replacing its identity, so observers do not keep a ghost online player
when a fixture is reset. The focused
`repository::tests::reset::guest_reset_records_departure_before_replacing_identity`
test passes; server-package formatting and clippy, `git diff --check`, and the
Rust file-size scan pass. The project `publish.ps1` Windows/WebGL build,
packaging, Preview deployment, and catalog-sync checks pass. No full workspace
gate was repeated because this was a bounded guest-session presence correction,
and no new external or deferred work was opened.

Account deletion now emits a durable offline presence after removing the last
access and refresh sessions, so connected observers can remove the deleted
character immediately instead of waiting for stale-presence aging. The focused
`repository::phase6::tests::account_cleanup::account_deletion_records_departure_for_connected_observers`
test passes; server-package formatting and clippy, `git diff --check`, and the
Rust file-size scan pass. The project `publish.ps1` Windows/WebGL build,
packaging, Preview deployment, and catalog-sync checks pass. No full workspace
gate was repeated because this was a bounded account-deletion presence
correction, and no new external or deferred work was opened.

Account linking now publishes the character's updated online presence after
the guest identity is replaced by its production account and display name.
Other clients can therefore reconcile the account boundary through their
normal event cursor instead of waiting for a full state refresh. The focused
`repository::phase6::tests::session_integrity::account_link_emits_the_updated_online_presence_for_other_clients`
test passes; server-package formatting and clippy, `git diff --check`, and the
Rust file-size scan pass. The project `publish.ps1` Windows/WebGL build,
packaging, Preview deployment, and catalog-sync checks pass. No full workspace
gate was repeated because this was a bounded account-link presence correction,
and no new external or deferred work was opened.

The offline development fixture now keeps its Walk pad and save, new-evening,
delete, and reconnect controls inside the sidebar instead of letting the lower
rows run into the footer. The focused `ui::tests` slice passes (4 tests), along
with client-package formatting and clippy, `git diff --check`, and the Rust
file-size scan. The project `publish.ps1` Windows/WebGL build, packaging,
Preview deployment, and catalog-sync checks pass. No full workspace gate was
repeated because this was a bounded offline-layout correction, and no new
external or deferred work was opened.

The regional inspection modal now preserves the visible `Reconnect` and
`Offline fixture` recovery actions while filtering out unrelated map and
sidebar commands. The focused `ui::tests` slice passes (5 tests), along with
client-package formatting and clippy, `git diff --check`, and the Rust
file-size scan. The project `publish.ps1` Windows/WebGL build, packaging,
Preview deployment, and catalog-sync checks pass. No full workspace gate was
repeated because this was a bounded regional-modal touch-recovery correction,
and no new external or deferred work was opened.

The woodworking timing overlay now preserves the visible `Reconnect` and
`Offline fixture` recovery actions alongside its timing button. A player can
therefore leave the challenge through the touch recovery path instead of being
trapped by the overlay's narrow action filter. The focused `ui::tests` slice
passes (6 tests), along with client-package formatting and clippy. `git diff --check`
and the Rust file-size scan pass. The project `publish.ps1` Windows/WebGL
build, packaging, Preview deployment, and catalog-sync checks pass. No full
workspace gate was repeated because this was a bounded crafting-overlay
touch-recovery correction, and no new external or deferred work was opened.

Session-level `Logout`, `Report`, and linked-account `Delete` controls now stay
enabled while the gameplay position projection is reloading, while gameplay
actions continue to wait for authoritative position and degraded connections
still require Reconnect. The focused
`ui::ui_online::tests::session_controls_stay_available_during_player_projection_reload`
regression passes, along with client-package formatting and clippy. `git diff --check`
and the Rust file-size scan pass. The project `publish.ps1`
Windows/WebGL build, packaging, Preview deployment, and catalog-sync checks
pass. No full workspace gate was repeated because this was a bounded
session-control readiness correction, and no new external or deferred work was
opened.

The session-only dispatch lane now sends account link, logout, report, and
account deletion commands while authoritative gameplay projection reload is in
progress, without releasing queued travel, route, market, or event commands or
opening new regional reads. The focused
`network::phase5::tests::session_dispatch::session_only_dispatch_sends_logout_without_releasing_gameplay_queue`
regression passes, along with client-package formatting and clippy. `git diff --check`
and the Rust file-size scan pass. The project `publish.ps1`
Windows/WebGL build, packaging, Preview deployment, and catalog-sync checks
pass. No full workspace gate was repeated because this was a bounded
session-dispatch recovery correction, and no new external or deferred work was
opened.

The regional client's identity and retained-history clearing logic now lives in
its own lifecycle module, keeping the main Phase 5 client below the 800-line
Rust source limit without changing behavior. The focused lifecycle regression
passes, along with client-package formatting and clippy. `git diff --check` and
the Rust file-size scan pass. The project `publish.ps1` Windows/WebGL build,
packaging, Preview deployment, and catalog-sync checks pass. No full workspace
gate was repeated because this was a bounded testability and file-ownership
maintenance correction, and no new external or deferred work was opened.

Gameplay-dependent overlays now wait for an authoritative player position
before showing their actionable inspection, skill, Chronicle, or crafting
surfaces during a state reload. The Account panel and visible Reconnect and
Offline fixture recovery controls remain available. The focused overlay
visibility regression passes, along with client-package formatting and clippy.
`git diff --check` and the Rust file-size scan pass. The project `publish.ps1`
Windows/WebGL build, packaging, Preview deployment, and catalog-sync checks
pass. No full workspace gate was repeated because this was a bounded touch
recovery presentation correction, and no new external or deferred work was
opened.

The authoritative world and event projection now lives in a dedicated child
module, reducing `src/network.rs` from 777 to 594 lines while preserving the
existing snapshot, presence, clock, chat, and cursor behavior. The focused
projection clock regression passes, along with client-package formatting and
clippy. `git diff --check` and the Rust file-size scan pass. The project
`publish.ps1` Windows/WebGL build, packaging, Preview deployment, and
catalog-sync checks pass. No full workspace gate was repeated because this was
a bounded file-ownership maintenance correction, and no new external or
deferred work was opened.

The Phase 5 route and cache tests now live in a dedicated `regional_commands`
child module, reducing the root test file from 788 to 581 lines while keeping
the route-authority cases together and preserving the project Rust source-size
rule. The focused route regression passes, along with client-package formatting
and clippy. `git diff --check` and the Rust file-size scan pass. No publisher or
full workspace gate was repeated because this was test-only organization
maintenance, and no new external or deferred work was opened.

The server's Phase 4 Wind Spark and Storm Magic scenarios now live in the
dedicated `phase4/tests/magic.rs` child module, reducing the combat test root
from 741 to 572 lines while keeping the magic rules together. The focused Storm
Magic discovery and power regression passes, along with server-package
formatting and strict clippy. `git diff --check` and the Rust file-size scan
pass. No publisher or full workspace gate was repeated because this was
test-only organization maintenance, and no new external or deferred work was
opened.

The Phase 4 combat queue and weapon-cycle cases now live in a dedicated
`combat_actions` child module, reducing the root Phase 4 test file from 784 to
739 lines while preserving the project Rust source-size rule. The focused
combat-queue regression passes, along with client-package formatting and
clippy. `git diff --check` and the Rust file-size scan pass. No publisher or
full workspace gate was repeated because this was test-only organization
maintenance, and no new external or deferred work was opened.

The local crafting timing marker now pauses while the authoritative player
projection reloads, matching the hidden gameplay overlay and preserving the
player's opportunity to tap SET QUALITY after recovery. The focused crafting
reload regression passes, along with client-package formatting and clippy.
`git diff --check` and the Rust file-size scan pass. The project `publish.ps1`
Windows/WebGL build, packaging, Preview deployment, and catalog-sync checks
pass. No full workspace gate was repeated because this was a bounded crafting
recovery correction, and no new external or deferred work was opened.

The Phase 6 persistence schemas now live in a dedicated `state.rs` child module,
keeping the production repository authority below the 800-line Rust source
limit without changing its serialized shape or behavior. The focused session
refresh regression passes, along with server-package formatting and strict
clippy. `git diff --check` and the Rust file-size scan pass. No publisher or
full workspace gate was repeated because this was server-only ownership
maintenance; the existing session-integrity test also replaced an iterator
warning exposed by the package clippy gate, and no new external or deferred
work was opened.

The client farming queue and target-selection cases now live in a dedicated
`network/tests/farming.rs` child module, keeping the root network test ledger
below the 800-line Rust source limit while preserving the farming coverage.
The focused farming target-selection regression passes, along with
client-package formatting and strict clippy. `git diff --check` and the Rust
file-size scan pass. No publisher or full workspace gate was repeated because
this was test-only organization maintenance, and no new external or deferred
work was opened.

The online sidebar's control-state, movement-recovery, population, and pioneer
status helpers now live in `ui_online/controls.rs`, reducing the primary online
UI module from 751 to 600 lines without changing its parent exports or touch
behavior. The focused movement-recovery tooltip regression passes, along with
client-package formatting and strict clippy. `git diff --check` and the Rust
file-size scan pass. No publisher or full workspace gate was repeated because
this was behavior-preserving ownership maintenance, and no new external or
deferred work was opened.

The Phase 4 local combat button and Storm Magic capability cases now live in
`network/phase4/tests/combat_controls.rs`, reducing the Phase 4 test root from
764 to 582 lines while keeping the combat controls together. The focused spell
action regression passes, along with client-package formatting and strict
clippy. `git diff --check` and the Rust file-size scan pass. No publisher or
full workspace gate was repeated because this was test-only organization
maintenance, and no new external or deferred work was opened.

The linked compact identity summary now names both supported actions: Account
opens the read-only identity view, while Logout leaves the session safely. The
focused account-summary regression passes, along with client-package
formatting and strict clippy. `git diff --check` and the Rust file-size scan
pass. The project `publish.ps1` Windows/WebGL build, packaging, Preview
deployment, and catalog-sync checks pass. No full workspace gate was repeated
because this was a bounded identity-summary correction, and no new external or
deferred work was opened.

Rate-limited client failures now explain that the player should wait briefly
and use the visible Reconnect control, while preserving the existing degraded
road state and cooldown. The focused connection-recovery regression passes,
along with client-package formatting and strict clippy. `git diff --check` and
the Rust file-size scan pass. The project `publish.ps1` Windows/WebGL build,
packaging, Preview deployment, and catalog-sync checks pass. No full workspace
gate was repeated because this was a bounded player incident-path correction,
and no new external or deferred work was opened.

Formal school lessons now accept a discovered, directly teachable advanced
skill when the teacher has the required Teaching depth; the previous root-only
mastery check made the catalogue's advanced teaching policy unreachable. The
focused `repository::skills::tests::a_discovered_advanced_skill_can_be_taught_without_granting_mastery`
regression passes, along with server-package formatting and strict clippy.
`git diff --check` and the Rust file-size scan pass. The project `publish.ps1`
Windows/WebGL build, packaging, Preview deployment, and catalog-sync checks
pass. No full workspace gate was repeated because this was a bounded school
lesson correction, and no new external or deferred work was opened.

Advanced skill discovery is now separate from actual usability. The server
reuses one readiness predicate for discovery, advanced teaching, the Storm
combat capability, and the serialized `SkillView.usable` signal; the client
uses that signal to keep a taught-but-unready Storm art on the basic Spell
control. The touch School control now opens a subject chooser for mastered
roots and ready discovered advanced arts, while an existing learner lesson
still joins directly. Focused protocol, server, client-network, combat, and
UI regressions pass, and the server/client package formatting checks pass.
The full workspace test/clippy gate, asset and source-size checks, and project
publisher also pass: Windows/WebGL builds, packaging, Preview deployment,
tracker recording, and catalog sync completed successfully. The remaining
follow-up is player playtesting of advanced lesson pacing and Storm feedback,
tracked in the Phase 6 register.

The follow-up hardening checks confirm that older serialized `SkillView`
payloads default the new usability signal to false and that a teacher carrying
only an advanced discovery cannot open a lesson until its own requirements are
complete. These focused protocol and server regressions pass; no additional
publisher or workspace-wide gate was repeated for this test-only correction.
The advanced-arts line now distinguishes a ready discovered art from a
learner-gated discovery, so the Practice ledger no longer presents both as
equivalent states. The focused UI regression, client clippy, formatting,
source-size and diff checks pass, and `publish.ps1` again passes the
Windows/WebGL build, packaging, Preview deployment, tracker recording, and
catalog sync. No new external or deferred work was opened.

The online sidebar now surfaces the latest server-owned tavern notice, or a
current tavern rumour when no notice is available, instead of leaving the
persisted feed hidden behind the world projection. The focused
`ui::ui_online::tests::tavern_feed_line_prefers_recent_notice_and_falls_back_to_rumour`
regression passes, along with client-package clippy, formatting, `git diff
--check`, and the Rust file-size scan. `publish.ps1` passes the Windows/WebGL
packages, Preview deployment, tracker recording, and catalog synchronization.
No full workspace gate is repeated because this is a bounded tavern-feed
presentation change; no new external or deferred work is opened.

The tavern signal now occupies the social feed area beside the newest chat line
so active pioneer progress and chronicle context remain visible in their
existing ledger line. The same focused tavern-feed regression, client clippy,
formatting, diff, and source-size checks pass; `publish.ps1` passes the
Windows/WebGL build, packaging, Preview deployment, tracker recording, and
catalog synchronization. No full workspace gate is repeated for this bounded
layout correction, and no new follow-up work is opened.

The empty online chat state now names the settlement channel consistently with
the client request path; the previous wilderness-presence check mislabeled the
shared chat even while the player stood at the Hearth. The focused chat-copy
regression, client clippy, formatting, diff, and source-size checks pass, and
`publish.ps1` passes the Windows/WebGL build, packaging, Preview deployment,
tracker recording, and catalog synchronization. No full workspace gate is
repeated for this bounded copy correction, and no new follow-up work is opened.

The Phase 4 summary now calls discovered knowledge “knowledge records” instead
of “lessons,” matching the server’s discover, archive, teach, and apply actions.
The focused `network::phase4::summary::tests::knowledge_summary_names_discovered_records`
regression passes, along with client-package clippy, formatting, `git diff
--check`, and the Rust file-size scan. `publish.ps1` passes the Windows/WebGL
build, packaging, Preview deployment, tracker recording, and catalog
synchronization. No full workspace gate is repeated because this is a bounded
summary-copy correction, and no new external or deferred work is opened.

The same summary copy now uses the singular “knowledge record” for one item
and the plural form for multiple items, keeping the compact ledger readable.
The focused knowledge-summary regression passes after covering both counts;
package clippy, formatting, `git diff --check`, the Rust file-size scan, and
the project publisher also pass. No full workspace gate is repeated because
this remains a bounded copy correction, and no new external or deferred work
was opened.

The online Hearth feed now treats the permanent startup board notice as a
fallback, allowing the current authoritative wilderness or household rumour
to surface when no newer actionable notice exists. The focused
`ui::ui_online::tests::tavern_feed::startup_board_notice_yields_to_the_current_rumour`
regression passes, along with client-package clippy, formatting,
`git diff --check`, and the Rust file-size scan. The project `publish.ps1`
Windows/WebGL build, packaging, Preview deployment, tracker recording, and
catalog synchronization pass. No full workspace gate is repeated because
this is a bounded tavern-feed selection correction, and no new external or
deferred work was opened.

Open online panels now visibly disable unrelated sidebar controls instead of
showing tappable actions that the modal filter would discard. Reconnect,
Offline, knockout recovery, and the regional route actions remain available
where their visible recovery or inspection paths require them. The focused
modal-filter regressions pass, along with client-package clippy, formatting,
`git diff --check`, and the Rust file-size scan. The project `publish.ps1`
Windows/WebGL build, packaging, Preview deployment, tracker recording, and
catalog synchronization also pass. No full workspace gate is repeated because
this is a bounded touch-focus correction, and no new external or deferred work
is opened.

The same modal boundary now covers the woodworking timing overlay: background
sidebar controls and map taps are visibly inactive, route controls remain
enabled only for the regional inspection that owns them, and keyboard movement
or chat cannot pass through the crafting challenge. The focused modal and
keyboard-input regressions pass, along with client-package clippy, formatting,
`git diff --check`, and the Rust file-size scan. The project `publish.ps1`
Windows/WebGL build, packaging, Preview deployment, tracker recording, and
catalog synchronization also pass. No full workspace gate is repeated because
this is a bounded modal-input correction, and no new external or deferred work
is opened.

The world-map tooltip now agrees with the modal input gate: when an inspection,
account, skill, school, chronicle, or woodworking panel is open, it tells the
player to close that panel instead of suggesting a map tap that will be
ignored. The focused tooltip regression passes, along with client-package
clippy, formatting, `git diff --check`, and the Rust file-size scan. The
project `publish.ps1` Windows/WebGL build, packaging, Preview deployment,
tracker recording, and catalog synchronization also pass. No full workspace
gate is repeated because this is a bounded guidance correction, and no new
external or deferred work is opened.

The online UI control regressions are now split into a dedicated child test
file, keeping the parent suite readable and below the 800-line Rust standard
as further player-facing controls are added. The focused online UI suite
(45 tests), client-package clippy, formatting, `git diff --check`, and the
Rust file-size scan pass. This is test-only organization maintenance, so no
publisher or full workspace gate is repeated, and no new external or deferred
work is opened.

The Phase 4 combat and animal-state integrity regressions now live in
`repository/tests/phase4_state_integrity/combat.rs`, reducing the shared
integrity test root from 600 to 533 lines while preserving health/status,
position, and care-day readiness contracts. The focused combat-integrity filter
passes (4 tests), along with server-package clippy with warnings denied,
package formatting, `git diff --check`, and the Rust file-size scan. No
publisher or full workspace gate is repeated because this is test-source
organization maintenance, and no new external or deferred work is opened.

The linked-account deletion confirmation now remains armed when the bounded
client command queue is full; the failed enqueue is reported so the player can
retry without silently losing the safety confirmation. The focused normal and
full-queue deletion regressions pass, along with client-package clippy,
formatting, `git diff --check`, and the Rust file-size scan. The project
`publish.ps1` Windows/WebGL build, packaging, Preview deployment, tracker
recording, and catalog synchronization also pass. No full workspace gate is
repeated because this is a bounded account-safety correction, and no new
external or deferred work is opened.

The online chronicle panel now lives in a named child module, keeping the
parent panel source below the 800-line Rust standard while preserving the
existing archive, search, and paging behavior. The focused online UI suite
(45 tests), client-package clippy with warnings denied, package formatting,
`git diff --check`, and the Rust file-size scan pass. This is source
organization maintenance, so no publisher or full workspace gate is repeated,
and no new external or deferred work is opened.

The Phase 6 operational module now keeps its readiness integrity evaluator and
alert thresholds in `operations/integrity.rs`, reducing the endpoint module
from 717 lines while preserving the public health and metrics surfaces. The
focused operations suite (3 tests), server-package clippy with warnings
denied, server formatting, `git diff --check`, and the Rust file-size scan pass.
This is source organization maintenance, so no publisher or full workspace
gate is repeated, and no new external or deferred work is opened.

Chronicle search now treats only a cursor ahead of the authoritative world as
invalid: all-history and paginated searches continue scanning the durable
archive after the bounded shared-event projection advances past an older
cursor. The focused chronicle-search suite (5 tests), server-package clippy
with warnings denied, server formatting, `git diff --check`, and the Rust
file-size scan pass. The project `publish.ps1` Windows/WebGL build, packaging,
Preview deployment, tracker recording, and catalog synchronization also pass.
No full workspace gate is repeated because this is a bounded chronicle-search
fix, and no new external or deferred work is opened.

The repository's shared error, request-text, optional-identifier, and event-
cursor boundaries now live in a dedicated child module, reducing the core
repository source from 726 to 641 lines without changing its authority
behavior. Server-package clippy with warnings denied, server formatting,
`git diff --check`, and the Rust file-size scan pass. This is source
organization maintenance, so no focused runtime suite, publisher, or full
workspace gate is repeated, and no new external or deferred work is opened.

The server skills entrypoints now keep discovery, mastery, qualifying-history,
and teaching rules in `skills/logic.rs`, reducing the endpoint module from 722
lines while preserving its catalogue and cross-phase hooks. The focused skills
suite (13 tests), server-package clippy with warnings denied, server formatting,
`git diff --check`, and the Rust file-size scan pass. This is source
organization maintenance, so no publisher or full workspace gate is repeated,
and no new external or deferred work is opened.

The Phase 3 repository now keeps its durable state, response cache, chronicle
retention, and compatibility normalization in `phase3/state.rs`, reducing the
main endpoint module from 715 to 607 lines without changing the frontier or
chronicle contract. The focused Phase 3 suite (13 tests), server-package clippy
with warnings denied, package formatting, `git diff --check`, and the Rust
file-size scan pass. No publisher or full workspace gate is repeated because
this is source organization maintenance, and no new external or deferred work
is opened.

Frontier expedition cycle tests now live in `network/frontier/tests/expedition.rs`,
reducing the frontier test root from 712 to 557 lines while preserving the
finished-cycle, missing-role, and retreat-notice contracts. The focused
frontier expedition filter passes (5 matching tests, including the remaining
expedition control regression), along with client-package clippy with warnings
denied, package formatting, `git diff --check`, and the Rust file-size scan. No
publisher or full workspace gate is repeated because this is test-source
organization maintenance, and no new external or deferred work is opened.

The online client now keeps the full shared-road sidebar renderer in
`ui_online/sidebar.rs`, reducing `ui_online.rs` from 676 to 217 lines while
preserving every farming, combat, trade, travel, recovery, account, and
touch-facing control. The focused UI controls suite (27 tests), client-package
clippy with warnings denied, package formatting, `git diff --check`, and the
Rust file-size scan pass. No publisher or full workspace gate is repeated
because this is source organization maintenance, and no new external or
deferred work is opened.

The Phase 5 client now keeps shared cursor polling in `phase5/polling.rs`,
regional event merging in `phase5/events.rs`, and market/action feedback in
`phase5/feedback.rs`, reducing the orchestration module from 716 to 581 lines.
The focused feedback suite (9 tests) and regional-event suite (8 tests), client
package clippy with warnings denied, package formatting, `git diff --check`,
and the Rust file-size scan pass. No publisher or full workspace gate is
repeated because this is source organization maintenance, and no new external
or deferred work is opened.

Phase 4 governance now keeps office inactivity, infrastructure upkeep, and
settlement-tax ticking in `governance/tick.rs`, reducing the governance module
from 693 to 516 lines while preserving its public office and proposal boundary.
The focused governance, tax, infrastructure-history, and upkeep-boundary tests
(4 tests), server-package clippy with warnings denied, package formatting,
`git diff --check`, and the Rust file-size scan pass. No publisher or full
workspace gate is repeated because this is source organization maintenance,
and no new external or deferred work is opened.

The client loop now keeps offline save, load, delete, and save-slot refresh
behavior in `game/offline.rs`, reducing `game.rs` from 725 to 665 lines without
changing the online or offline action contract. Client-package clippy with
warnings denied, package formatting, `git diff --check`, and the Rust file-size
scan pass; no focused runtime suite was added because this move changes no
behavior. No publisher or full workspace gate is repeated because this is
source organization maintenance, and no new external or deferred work is
opened.

The HTTP server now keeps bounded request bodies, bearer parsing, URL splitting,
and form-query decoding in `http/request.rs`, reducing `http.rs` from 691 to
598 lines while preserving the route and CORS boundary. The focused HTTP suite
(14 tests), server-package clippy with warnings denied, package formatting,
`git diff --check`, and the Rust file-size scan pass. No publisher or full
workspace gate is repeated because this is source organization maintenance,
and no new external or deferred work is opened.

Phase 4 now keeps its durable state, default records, response cache, and
startup construction in `phase4/state.rs`, reducing the main repository module
from 516 to 402 lines after the earlier governance lifecycle split. The two
directly affected regressions for legacy animal restoration and governance
construction pass, along with server-package clippy with warnings denied,
package formatting, `git diff --check`, and the Rust file-size scan. No
publisher or full workspace gate is repeated because this is source
organization maintenance, and no new external or deferred work is opened.

The Phase 4 client now keeps command-response projection updates and readable
feedback dispatch in `phase4/commands.rs`, reducing the orchestration root
from 698 to 623 lines while preserving cursor ordering and touch-facing
responses. The focused projection-ordering suite (5 tests), client-package
clippy with warnings denied, package formatting, `git diff --check`, and the
Rust file-size scan pass. No publisher or full workspace gate is repeated
because this is source organization maintenance, and no new external or
deferred work is opened.

The Phase 6 account module now keeps the authenticated account read and
deletion-scheduling endpoints in `phase6/account/endpoints.rs`, leaving
cross-phase guest-account migration in the parent module and reducing the
account migration file from 725 to 544 lines. The focused account-validation
suite (4 tests) and deletion-queue suite (3 tests), server-package clippy with
warnings denied, package formatting, `git diff --check`, and the Rust
file-size scan pass. No publisher or full workspace gate is repeated because
this is source organization maintenance, and no new external or deferred work
is opened.

Phase 6 account identity reads and deletion scheduling now live in the
`phase6/account.rs` child alongside guest-account migration, reducing the
authority root from 694 to 520 lines while preserving the account, privacy,
and deletion contracts. The focused account-validation suite (4 tests) and
deletion-queue suite (3 tests), server-package clippy with warnings denied,
package formatting, `git diff --check`, and the Rust file-size scan pass. No
publisher or full workspace gate is repeated because this is source
organization maintenance, and no new external or deferred work is opened.

Phase 5 commodity inventory, regional stock notes, indexed pricing, and season
helpers now live in `phase5/logic/commodities.rs`, reducing the regional logic
module from 674 to 512 lines while preserving the market and projection
boundaries. The focused market-history suite (3 tests) and price-boundary test
(1 test) pass, along with server-package clippy with warnings denied, package
formatting, `git diff --check`, and the Rust file-size scan. No publisher or
full workspace gate is repeated because this is source organization
maintenance, and no new external or deferred work is opened.

The Phase 5 regional event read and action endpoints now live in
`phase5/events.rs`, leaving event transition authority in the existing logic
module and reducing the endpoint root from 626 to 543 lines. The focused event
choice suite (2 tests) passes, along with server-package clippy with warnings
denied, package formatting, `git diff --check`, and the Rust file-size scan. No
publisher or full workspace gate is repeated because this is source
organization maintenance, and no new external or deferred work is opened.

The client action dispatcher now lives beside the existing interaction routing
in `game/actions.rs`, reducing the runtime root from 665 to 526 lines while
preserving online, offline, recovery, chat, and touch-facing action IDs. The
focused UI controls suite (27 tests) passes, along with client-package clippy
with warnings denied, package formatting, `git diff --check`, and the Rust
file-size scan. No publisher or full workspace gate is repeated because this
is source organization maintenance, and no new external or deferred work is
opened.

The frontier client now keeps command-response projection updates and player
notices in `network/frontier/feedback.rs`, reducing the frontier orchestration
module from 667 to 443 lines while preserving contract, combat, recovery,
expedition, and homestead response handling. The focused frontier feedback
tests pass, along with client-package clippy with warnings denied, package
formatting, `git diff --check`, and the Rust file-size scan. No publisher or
full workspace gate is repeated because this is source organization
maintenance, and no new external or deferred work is opened.

The Phase 4 client now keeps crafting challenge start, projection, and submit
behavior beside combat timing in `network/phase4/combat.rs`, reducing the
orchestration root from 623 to 580 lines while preserving the bounded quality
score and queue-full recovery path. The focused crafting suite (4 tests)
passes, along with client-package clippy with warnings denied, package
formatting, `git diff --check`, and the Rust file-size scan. No publisher or
full workspace gate is repeated because this is source organization
maintenance, and no new external or deferred work is opened.

The repository now keeps world ticking, persistence writes, session-expiry
persistence, and tick telemetry in `repository/tick.rs`, reducing the
repository root from 641 to 531 lines while preserving the shared authority
hooks used by every phase. The focused telemetry (1), numeric-boundary (4),
and persistence (11) regressions pass, along with server-package clippy with
warnings denied, package formatting, `git diff --check`, and the Rust file-size
scan. No publisher or full workspace gate is repeated because this is source
organization maintenance, and no new external or deferred work is opened.

Phase 3 chronicle and opportunity read endpoints now live in
`phase3/presentation.rs`, reducing the phase repository module from 607 to 566
lines while leaving contract, combat, and tick authority together. The focused
chronicle history and cursor suite (2 tests) passes, along with server-package
clippy with warnings denied, package formatting, `git diff --check`, and the
Rust file-size scan. No publisher or full workspace gate is repeated because
this is source organization maintenance, and no new external or deferred work
is opened.

The crafting authority regression now submits a zero timing score and confirms
an accepted completion still produces positive bounded quality without changing
the requester's escrowed materials. The focused profession test (1 test)
passes, along with server-package clippy with warnings denied, package
formatting, `git diff --check`, and the Rust file-size scan. No publisher or
full workspace gate is repeated because this evidence-only change targets one
GDD crafting contract, and no new external or deferred work is opened.

Phase 6 backup and readiness integration tests now live in
`repository/phase6/tests/integration/backup.rs`, reducing the integration test
root from 710 to 633 lines while preserving the complete-snapshot and failed-
backup recovery contracts. The focused backup filter passes (2 tests), along
with server-package clippy with warnings denied, package formatting,
`git diff --check`, and the Rust file-size scan. No publisher or full workspace
gate is repeated because this is test-source organization maintenance, and no
new external or deferred work is opened.

The event-manifest validation regressions now live in
`content/tests/events.rs`, reducing the content test root from 575 to 521
lines while preserving implemented-effect, nonblank-system, and affected-
location contracts. The focused event-content filter passes (3 tests), along
with server-package clippy with warnings denied, package formatting,
`git diff --check`, and the Rust file-size scan. No publisher or full workspace
gate is repeated because this is test-source organization maintenance, and no
new external or deferred work is opened.

The Phase 6 operator account-view integration regression now lives in
`repository/phase6/tests/integration/support_account.rs`, reducing the
integration test root from 633 to 553 lines while preserving operator-only,
secret-free account, claim, trade, chronicle, and missing-account contracts.
The focused support-account filter passes (1 test), along with server-package
clippy with warnings denied, package formatting, `git diff --check`, and the
Rust file-size scan. No publisher or full workspace gate is repeated because
this is test-source organization maintenance, and no new external or deferred
work is opened.

The Phase 4 farming counter, knockout, and same-beat tending regressions now
live in `repository/phase4/tests/farming/guards.rs`, reducing the farming test
root from 658 to 502 lines while preserving numeric saturation, recovery
gating, and tool-condition timing contracts. The focused farming-guards filter
passes (3 tests), along with server-package clippy with warnings denied,
package formatting, `git diff --check`, and the Rust file-size scan. No
publisher or full workspace gate is repeated because this is test-source
organization maintenance, and no new external or deferred work is opened.

The shared regional route and settlement topology integrity regressions now
live in `repository/tests/integrity/regional_topology.rs`, reducing the
integrity test root from 638 to 574 lines while preserving bounds, collection,
reference, and duplicate-location readiness contracts. The focused regional-
topology filter passes (5 tests), along with server-package clippy with
warnings denied, package formatting, `git diff --check`, and the Rust file-size
scan. No publisher or full workspace gate is repeated because this is
test-source organization maintenance, and no new external or deferred work is
opened.

The Phase 5 travel lock regression now lives beside the existing travel
percentage boundary in `repository/phase5/tests/travel_boundary.rs`, reducing
the Phase 5 test root from 600 to 544 lines while preserving server-arrival
gating and large-route progress contracts. The focused travel-boundary filter
passes (2 tests), along with server-package clippy with warnings denied,
package formatting, `git diff --check`, and the Rust file-size scan. No
publisher or full workspace gate is repeated because this is test-source
organization maintenance, and no new external or deferred work is opened.

The repository's validated farm-layout and legacy-migration regressions now
live in `repository/tests/farm_migration.rs`, reducing the shared repository
test root from 667 to 599 lines while preserving fresh-manifest, empty-legacy,
and populated-legacy crop-state contracts. The focused farm-migration filter
passes (3 tests), along with server-package clippy with warnings denied,
package formatting, `git diff --check`, and the Rust file-size scan. No
publisher or full workspace gate is repeated because this is test-source
organization maintenance, and no new external or deferred work is opened.

Phase 3 chronicle retention and cursor-boundary tests now live in
`repository/phase3/tests/chronicle.rs`, reducing the phase test root from 673
to 604 lines while preserving archive search and stale/ahead cursor contracts.
The focused chronicle filter passes (2 tests), along with server-package clippy
with warnings denied, package formatting, `git diff --check`, and the Rust
file-size scan. No publisher or full workspace gate is repeated because this is
test-source organization maintenance, and no new external or deferred work is
opened.

Phase 4 lesson-state integrity coverage now lives in
`repository/tests/phase4_state_integrity/lessons.rs`, reducing the shared
integrity test root from 666 to 600 lines while preserving malformed text,
future start, expiry ordering, and capacity readiness contracts. The focused
lesson-integrity filter passes (4 tests), along with server-package clippy with
warnings denied, package formatting, `git diff --check`, and the Rust file-size
scan. No publisher or full workspace gate is repeated because this is
test-source organization maintenance, and no new external or deferred work is
opened.

Phase 4 restart and missing-data migration coverage now lives in
`repository/phase4/tests/restart.rs`, reducing the phase test root from 671 to
577 lines while preserving movement, combat cooldown, governance persistence,
and Phase 4 default-migration contracts. The focused restart filter passes (1
test), along with server-package clippy with warnings denied, package
formatting, `git diff --check`, and the Rust file-size scan. No publisher or
full workspace gate is repeated because this is test-source organization
maintenance, and no new external or deferred work is opened.

The account-cleanup privacy regression now lives in
`repository/phase6/tests/account_cleanup/settlement_history.rs`, reducing the
account-cleanup test root from 651 to 588 lines while preserving copied
settlement-history name updates across identity linking and deletion. The
focused settlement-history filter passes (1 test), along with server-package
clippy with warnings denied, package formatting, `git diff --check`, and the
Rust file-size scan. No publisher or full workspace gate is repeated because
this is test-source organization maintenance, and no new external or deferred
work is opened.

Shared network projection, clock, chronicle-cache, presence, version, and
snapshot tests now live in `network/tests/projection.rs`, reducing the network
test root from 611 to 493 lines while preserving the six projection contracts.
The focused projection filter passes (6 tests), along with client-package
clippy with warnings denied, package formatting, `git diff --check`, and the
Rust file-size scan. No publisher or full workspace gate is repeated because
this is test-source organization maintenance, and no new external or deferred
work is opened.

The Phase 4 client crafting timing and reload tests now live in
`network/phase4/tests/crafting.rs`, reducing the phase test root from 618 to
582 lines while preserving the wide-target movement and authoritative-reload
pause contracts. The focused crafting filter passes (4 matching tests,
including the existing completion and queue-capacity regressions), along with
client-package clippy with warnings denied, package formatting,
`git diff --check`, and the Rust file-size scan. No publisher or full workspace
gate is repeated because this is test-source organization maintenance, and no
new external or deferred work is opened.
