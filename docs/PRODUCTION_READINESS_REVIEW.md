# Tarrowyn production-readiness review

The lasting-realm gate is substantially represented in the release candidate:
the regional authority is cursorable and recoverable; account linking,
expiration, refresh, revocation, moderation reporting, support audit, health,
measured tick metrics, operational alert visibility, backup-failure visibility,
economy/population monitoring, content validation, and touch recovery are present;
and the
calendar, topology, law, privacy, legacy, and operational decisions are
written down. The player-facing privacy path now includes a visible,
two-tap linked-account deletion control that clears the session and exposes
Reconnect after the server schedules the authoritative deletion tick.
The client also polls readiness and turns configured maintenance or degraded
health into visible next-step status guidance.

The measured scope is deliberately regional: one worker, 24 concurrent-client
target, bounded queues, and selectable JSON/MySQL persistence. The local Phase 6
mixed-load drill passed with 24 clients, three rounds, 624 requests, and 4,578.51
ms wall time, including backup and restart recovery. The accelerated long-session
fixture also crosses all four development seasons while checking lease, tax,
market, household, chronicle, and newcomer continuity. Season labels also come
from the validated region calendar; the configured real-time day length remains
an explicit deployment setting. The MySQL backend applies
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
The snapshot bridge now holds a process-lifetime MySQL authority lock, so an
additional worker sharing the database fails closed at startup; this protects
the one-worker contract but is not a substitute for the deferred multi-worker
relational design.
Boundary probes also completed the same recovery path at 50 and 100 clients when
the expected `market_backlog` alert was explicitly allowed; these runs are
evidence for monitored operation, not for the several-hundred-player goal.
The 250-client boundary probe also completed the recovery path with the same
allowlist, but took 82,299.85 ms for 2,500 requests. That result is retained as
capacity evidence against promoting the one-worker snapshot bridge to the
GDD's several-hundred-player direction; the supported release target remains
24 clients.
The standalone JSON restore-on-a-copy drill also passed: it loaded the generated
backup, confirmed readiness and a replacement backup, ran the regional Phase 5
tests, and preserved the active state file.

After launch, content operations should add new routes, farm plots, crops, contracts,
events, households, and settlement opportunities through the validated JSON
pipeline. The current validator protects the canonical manifests and their
identifiers, and server planting, regional event seeding, settlement profiles,
infrastructure projections, and market base prices consume validated
crop/event/settlement/item content. Settlement condition and opportunity
profiles also consume validated settlement content. The repeatable Brambleback
contract and
launch wilderness threat consume validated contract/threat content, while
route topology, season labels, and location roles/resources consume validated
region content, including location presentation and route tuning. Opportunity
and regional household projections consume validated household content.
Infrastructure projections consume validated infrastructure content, and the
field-tool repair order consumes validated recipe content. Each live
simulation consumer, including the fixed Bellweather household, has its own
fixture and compatibility test before future content is treated as
runtime-authoritative. General NPC family simulation remains a documented GDD
deferral.
Chronicle history keeps a 64-entry current window, moves older records into a
durable searchable archive, and exposes a bounded archive summary to the normal
client view. Account deletion anonymises matching chronicle text in the recent
window, archive, and event stream. New content must preserve newcomer access,
fallback services, and this history boundary.
