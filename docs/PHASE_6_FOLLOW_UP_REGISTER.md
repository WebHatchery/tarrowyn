# Phase 6 follow-up register

This register keeps release-candidate evidence separate from work that needs a
deployment environment, a later product decision, or more content. It is the
short list to review before expanding Tarrowyn beyond the supported regional
preview.

| Work item | Status | Evidence in this repository | Exit evidence still required |
|---|---|---|---|
| Target MySQL migration and restore | Target-required | `scripts/verify_mysql.ps1` proves the migration, restart, replay, backup, and native dump/restore flow against the configured local preview. | Deployment owner runs the same checks against the target database, including a known-good restore and integrity comparison. |
| Multi-worker and regional handoff | Target-required | The MySQL bridge holds one process-lifetime world-authority lock; a second worker fails closed. Client cursors are recoverable, but ownership handoff is not implemented. | Choose relational ownership and cursor handoff, then prove node/region failure without duplicate rewards, split-brain state, impossible travel, or cursor gaps. |
| Deployment identity, TLS, and secrets | Target-required | The OIDC link/refresh/revoke fixture, secret-free account view, and client session recovery are covered locally. | Configure the real identity gateway, TLS termination, secret manager, rotation, credential recovery/MFA boundary, and external alert routing in the target environment. |
| Scale beyond the supported regional target | Target-required | The 24-client mixed-load drill passes. Exploratory 50/100/250-client runs are retained as monitored capacity evidence, not a capacity claim. | Select topology and establish target-environment limits for concurrency, latency, memory, tick drift, queue pressure, and restart recovery. |
| Richer route and market inspection | Implemented surface | The online sidebar keeps compact authoritative road availability/risk, open orders, and the protected-law boundary, while the visible `Inspect` control opens route names/status/condition/risk plus the first stock and price notes from the same server projection. | Playtest the notice wording and information density; adjust the surface if player evidence shows that detail competes with recovery, comparison, or chronicle controls. |
| Season and year pacing | Deliberately deferred | The 80-minute day is locked; the four-season, 14-day season and 56-day year values are development fixtures. The long-session test crosses boundaries without locking access. | Run player-facing crop, lease, access, and economy playtests; record the final calendar decision before seasonal timing becomes a product promise. |
| Content expansion and support cadence | Post-launch content work | The manifest gate and typed server validators protect IDs, required fields, cross-references, fallback services, and newcomer access. | Add content in small reviewed batches, attach compatibility fixtures, and monitor population, scarcity, abandoned claims, settlement decline, and newcomer access after each batch. |
| Storm Magic merger interaction path | Post-launch content work | `assets/data/skills.json` defines the server-owned prerequisites and qualifying event, but the bounded prototype has no severe-weather three-element interaction producer or Storm Magic combat action. | Design and implement an explicit, resource-bounded Wind/Water/Electricity interaction with severe-weather validation, durable qualifying history, discovery feedback, and client touch controls before treating Storm Magic as playable content. |
| Human regional playthrough | Evidence still desired | Automated three-client, Phase 4, Phase 5, MySQL, failure, and long-session checks cover the current release candidate. | Record a human multi-session regional playthrough when the client is available for that study; treat it as product evidence, not a substitute for server acceptance tests. |
| PvP, criminal roles, generational play, and general NPC life simulation | Deliberately deferred | The GDD protects the launch law boundary, immortal characters, and fixed authored NPCs. | Reopen each system only with a separate design decision, migration plan, recovery rules, and acceptance fixtures. |

The register does not authorize a public launch by itself. The target-required
rows remain deployment gates; the deliberately deferred rows remain product
choices rather than missing implementation. Any new row should name its owner,
evidence, and exit condition before code or content is added.
