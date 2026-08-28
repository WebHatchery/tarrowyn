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
settlement, region, calendar, and game-config manifests with typed schemas and
cross-reference checks; duplicate IDs and incompatible records fail startup.
The server crop-rotation regression confirms planting follows the validated
crop manifest rather than a separate hard-coded order.
The server event-template regression confirms regional event seeding follows
the validated event manifest, including its narrative, effects, and
intervention options.
The settlement supply regression confirms abundant and scarce goods in the
authoritative projections follow the validated settlement manifest.
The market price regression confirms every traded commodity's base price
comes from the validated item manifest.
The calendar regression confirms regional season labels follow the validated
region calendar at season and year boundaries.
The route-profile regression confirms authoritative route transport and
endpoint topology follow the validated region manifest.
The location-profile regression confirms authoritative location roles and
resources follow the validated region manifest.
The contract-template regression confirms the repeatable Brambleback watch
uses the validated contract manifest for its target and progression contract.
The threat-template regression confirms the launch wilderness threat follows
the validated threat manifest for its monster, position, health, and risk.
The Phase 5 fixture verifies that travel, market, event, household, identity,
refresh, and revocation state survive the authoritative repository boundary.
The client Phase 5 tests verify that a linked account's visible deletion
control requires two taps, a development guest cannot arm deletion, and the
deletion response is decoded as its dedicated command rather than the
ambiguous market response.
The selected MySQL bridge now has a checked-in migration, startup pool/migration
failure handling, transactional snapshot/index writes, and driver-selection
tests. The configured local preview MySQL service passed
`scripts/verify_mysql.ps1`: storage version 20 readiness, authoritative animal
state, duplicate chat/movement/auth/moderation replay, concurrent duplicate chat replay,
temporary backup creation, and identity/state persistence across a server
restart all succeeded. It then restored a native
`mysqldump` into a generated temporary database and verified the current world
row and identity index before cleanup. The script uses a unique guest key and
does not reset or delete the configured database.

The target environment still owns the remaining migration, multi-worker
concurrent-write, database failover, and rollback gates. The local script
exercises the JSON backup companion, native dump/restore, overlapping retries,
and the single-worker MySQL bridge, not production topology or database
failover.

## Security gate

Representative checks cover unsupported identity providers, bounded provider
subjects, empty repair notes, request-ID and 64 KiB request-body validation, idempotent regional
mutations, expired/revoked access, refresh rotation, chat limits, and the
protected no-PvP law response. Chat metadata, direct trades, claims, governance,
moderation reports, and support repairs are audit-linked without copying chat
text into the audit stream. Moderation reports are queued and audit-linked. The
support account view is operator-only, returns the requested character-facing
records and cursor, and excludes session tokens and provider subjects.
The provider secret and TLS termination remain deployment concerns and are not
stored in the repository.

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
with 24 clients and three rounds: 552 HTTP requests completed in 4,479.15 ms of
mixed-load wall time, with 104 accepted and 88 rejected command outcomes. The
run exercised state, events, movement, chat, markets, travel, the autonomous
tick, scheduled backup, operator metrics, server-owned arrival, and restart
recovery. The result is evidence for the bounded 24-client regional target, not
for multi-worker production concurrency or several-hundred-player capacity.

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
