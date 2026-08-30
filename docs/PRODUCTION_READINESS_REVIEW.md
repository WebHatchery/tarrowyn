# Tarrowyn production-readiness review

The lasting-realm gate is substantially represented in the release candidate:
the regional authority is cursorable and recoverable; account linking,
expiration, refresh, revocation, moderation reporting, support audit, health
with bounded integrity-failure diagnostics, measured tick metrics,
operational alert visibility, backup-failure visibility,
economy/population monitoring, content validation, and touch recovery are present;
and the
calendar, topology, law, privacy, legacy, and operational decisions are
written down. The player-facing privacy path now includes a visible,
two-tap linked-account deletion control that clears the session and exposes
Reconnect after the server schedules the authoritative deletion tick.
The client also polls readiness and turns configured maintenance or degraded
health into visible next-step status guidance.
It preserves structured API error codes through the shared toolkit and, when a
restore or retained-history boundary invalidates its event cursor, clears stale
history projections and reloads the authoritative state from cursor zero while
keeping the road open.

The readiness projection now validates every durable layer represented by the
regional release candidate: world clock, players, crops, trades, event history,
frontier state, Phase 4 civic records, Phase 5 regional records, and Phase 6
production identity, session, audit, moderation, replay, deletion, and backup
metadata. Malformed cross-references degrade operator readiness instead of
being served as authoritative state. Focused Phase 6 regressions and the
cross-subsystem release gate are recorded in PHASE_6_TEST_REPORT.md.

The measured scope is deliberately regional: one worker, 24 concurrent-client
target, bounded queues, and selectable JSON/MySQL persistence. The local Phase 6
mixed-load drill passed with 24 clients, three rounds, 624 requests, and 5,390.96
ms wall time, including backup and restart recovery. The same drill crossed the
2,048-record event window and verified `cursor_stale` on both event endpoints.
The accelerated long-session
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
The configured preview credentials pass password-safe `SELECT 1`, and
`SHOW GRANTS` confirms `ALL PRIVILEGES` on the configured `tarrowyn` schema.
The account has no server-level `CREATE` or `DROP` grant, so it cannot provision
the disposable restore database (`ERROR 1044`); no clean-schema MySQL
migration, replay, restart, or native restore assertion is claimed. The
configured legacy database remains untouched; the acceptance runner needs
`CREATE/DROP DATABASE` permission and a clean current snapshot before that
gate can run.
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
The latest 24-client baseline additionally recorded 67.06 MB of server working
set after load and 2,837.56 ms from worker stop through restart readiness. These
local measurements inform the target-environment gate but are not production
memory or recovery SLOs.
The standalone JSON restore-on-a-copy drill also passed: it loaded the generated
backup, confirmed readiness and a replacement backup, ran the regional Phase 5
tests, and preserved the active state file.
The live Phase 6 journey now also proves the allowlisted support-account HTTP
view, its character and cursor boundary, secret-free response shape, and the
ordinary-player 403 boundary.

The complete list of target-environment gates, desired human evidence, and
deliberate product deferrals is maintained in
[`PHASE_6_FOLLOW_UP_REGISTER.md`](PHASE_6_FOLLOW_UP_REGISTER.md); this review
does not treat any of those external or deferred rows as silently complete.

After launch, content operations should add new routes, farm plots, crops, contracts,
events, households, and settlement opportunities through the validated JSON
pipeline. The current validator protects the exact canonical manifest set,
required record fields, and their identifiers, and server planting, regional
event seeding, settlement profiles,
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
