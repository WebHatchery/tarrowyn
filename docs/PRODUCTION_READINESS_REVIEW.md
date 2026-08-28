# Tarrowyn production-readiness review

The lasting-realm gate is substantially represented in the release candidate:
the regional authority is cursorable and recoverable; account linking,
expiration, refresh, revocation, moderation reporting, support audit, health,
metrics, backups, content validation, and touch recovery are present; and the
calendar, topology, law, privacy, legacy, and operational decisions are
written down.

The measured scope is deliberately regional: one worker, 24 concurrent-client
target, bounded queues, and selectable JSON/MySQL persistence. The MySQL
backend applies an explicit migration and writes the authoritative snapshot
and identity index transactionally. Public launch remains blocked until the
deployment owner runs that backend against the target database, proves
concurrent-write behavior, completes a MySQL restore drill, and supplies the
real identity-gateway configuration, TLS termination, secret rotation, and
external alert routing. Preview connection settings belong in the ignored
`.env.preview` file; production credentials must come from its secret manager.
Those are explicit remaining risks, not hidden behind the game client.

After launch, content operations should add new routes, crops, contracts,
events, households, and settlement opportunities through the validated JSON
pipeline while preserving newcomer access, fallback services, and searchable
chronicle history.
