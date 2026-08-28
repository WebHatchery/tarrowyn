# Phase 6 release-candidate test report

## Design and persistence gate

The workspace has a versioned storage document, atomic replacement, scheduled
backup metadata, integrity readiness, production session records, audit records,
and a support repair API. Storage version 14 also persists field-tool condition,
real-time lease timestamps, and public tax receipts,
the per-character skill ledger, Bellweather animal condition, and daily care
state, and loads older documents through serde defaults. A Phase 1–4
document without Phase 5/6 fields loads through serde defaults and receives the
current regional and operations state.
The Phase 5 fixture verifies that travel, market, event, household, identity,
refresh, and revocation state survive the authoritative repository boundary.
The selected MySQL bridge now has a checked-in migration, startup pool/migration
failure handling, transactional snapshot/index writes, and driver-selection
tests. No live MySQL service is available in this workspace, so the migration,
restart, concurrent-write, backup/restore, and rollback checks remain target-
environment gates.

## Security gate

Representative checks cover unsupported identity providers, bounded provider
subjects, empty repair notes, request-ID validation, idempotent regional
mutations, expired/revoked access, refresh rotation, chat limits, and the
protected no-PvP law response. Moderation reports are queued and audit-linked.
The provider secret and TLS termination remain deployment concerns and are not
stored in the repository.

## Load and failure gate

The accepted regional target is 24 connected clients, 50 open orders, and a
250 ms tick. The repository's bounded projections and event cursors avoid
broadcasting every regional entity to every client. The release scripts
exercise concurrent fixture requests, backup parsing, server readiness, and
restore-on-a-copy. Node-failure and clock-restart behavior are reconciled by
the durable travel/order/event cursors; a duplicate request returns its cached
result instead of paying twice.

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
