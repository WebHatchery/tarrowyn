# Tarrowyn production-readiness review

The lasting-realm gate is substantially represented in the release candidate:
the regional authority is cursorable and recoverable; account linking,
expiration, refresh, revocation, moderation reporting, support audit, health,
measured tick metrics, operational alert visibility, backup-failure visibility,
economy/population monitoring, content validation, and touch recovery are present;
and the
calendar, topology, law, privacy, legacy, and operational decisions are
written down.

The measured scope is deliberately regional: one worker, 24 concurrent-client
target, bounded queues, and selectable JSON/MySQL persistence. The local Phase 6
mixed-load drill passed with 24 clients, three rounds, 552 requests, and 4,479.15
ms wall time, including backup and restart recovery. The accelerated long-session
fixture also crosses all four development seasons while checking lease, tax,
market, household, chronicle, and newcomer continuity. The MySQL backend applies
an explicit migration and writes the authoritative snapshot
and identity index transactionally. Public launch remains blocked until the
deployment owner runs that backend against the target database, proves
concurrent-write behavior, completes a MySQL restore drill, and supplies the
real identity-gateway configuration, TLS termination, secret rotation, and
external alert routing. Preview connection settings belong in the ignored
`.env.preview` file; production credentials must come from its secret manager.
Those are explicit remaining risks, not hidden behind the game client. Client
connection-failure alerting remains deployment/client-owned because the server
cannot observe a client after its connection disappears.

After launch, content operations should add new routes, crops, contracts,
events, households, and settlement opportunities through the validated JSON
pipeline. The current validator protects the canonical manifests and their
identifiers; each new live simulation consumer still needs its own fixture and
compatibility test before that content is treated as runtime-authoritative.
New content must preserve newcomer access, fallback services, and searchable
chronicle history.
