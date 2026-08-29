# Phase 3 — The Living Frontier

> Implementation status: complete for the documented vertical slice. The
> release proof is recorded in [`PHASE_3_RUNBOOK.md`](PHASE_3_RUNBOOK.md).
>
> Current validation focus: deterministic fixtures, automated HTTP acceptance,
> and concurrent client polling. Community playthrough recording is deferred
> until the client is more playable.

## Purpose

Add the first systems that make Tarrowyn feel like a society with consequences
rather than a shared task board. Phase 3 connects the settlement to nearby
wilderness, opportunity-driven NPC life, local history, and one player-led
frontier project.

This is still a deliberately small vertical slice. It does not attempt the
whole MMO, a complete combat system, or kingdom-scale governance. It proves
that one world event can ripple through multiple professions and leave a record
that later players can discover.

## Build scope

### Wilderness and risk

- Add one forest/wilderness zone and one monster type.
- Add one repeatable adventurer contract that can be accepted, progressed, and
  reported through the tavern.
- Add a basic weapon plus one inferior/improvised substitute.
- Add server-authoritative knockout and prompt recovery. Defeat should return
  control promptly while applying bounded carried-item, injury, recovery-cost,
  or rescuer consequences. Owned property and stored goods remain safe in this
  phase.
- Make one threat alter more than combat: it should affect a road, field,
  resource, price, rumour, contract, or crafting demand.

### Opportunity-driven NPCs

Implement the smallest useful household model:

- a household has members, occupations, a home settlement, and a current
  opportunity score;
- sustained unmet demand can produce a candidate arrival or travelling service;
- the household can decline or leave after sustained poor conditions; and
- players receive clues through notices, dialogue, rumours, or reduced service
  before departure.

NPCs are fallback capacity and story participants, not exact replacements for a
missing human profession. Their decisions remain server-owned and are recorded
as world events.

### Adventurer credentials

The frontier projection now exposes a bounded rank earned from varied evidence,
not a global level: a completed Brambleback watch grants the Trailhand
credential, a successful pioneer expedition advances a participating player to
Pathfinder, and sustained Hearth standing plus repeated watch reports can earn
Road Warden. The rank and earned credentials are derived by the server and
appear in every player projection. Expedition participation is retained as a
durable credential when a later expedition replaces the current registry
record, while account linking, development reset, and deletion keep that
identity boundary clean. Credentials do not grant raw combat power.

### History, claims, and the frontier

- Record settlement events such as founding work, major threats, arrivals,
  departures, repairs, contracts, and abandoned structures.
- Show a small chronicle in the tavern or settlement registry so players can
  understand that a change happened to the community, not only to their HUD.
- Add one recognised, renewable homestead claim or lease with inactivity and
  reclamation rules explicit in data.
- Let a prepared group announce and attempt one pioneer expedition to establish
  a small outpost beyond the first settlement. The expedition needs food, tools,
  construction materials, safety, and people with complementary skills. Failure
  creates a retreat/recovery story instead of deleting characters. The first
  pioneer party is capped at 20 named members so its durable planning record
  remains a bounded small-group projection.

## Server and protocol requirements

The server remains authoritative for combat outcomes, contract state, NPC
decisions, claims, event history, and expedition resolution. Add domain events
with stable IDs and cursors so a reconnecting client can catch up without
replaying rewards.

Suggested additions include:

| Endpoint | Purpose |
|---|---|
| `GET /v1/contracts` / `POST /v1/contracts/{id}` | List, accept, progress, and report the repeatable contract. |
| `POST /v1/combat/actions` | Submit a bounded combat intent and receive authoritative results. |
| `POST /v1/recovery` | Resolve knockout recovery, rescuer, and consequence choices. |
| `GET /v1/settlement/chronicle` | Read cursorable community history. |
| `GET /v1/settlement/opportunities` | Show demand and NPC/service signals without exposing hidden decisions. |
| `POST /v1/claims` | Request, renew, abandon, or inspect a small land claim/lease. |
| `POST /v1/expeditions` | Announce, join, supply, launch, or resolve one pioneer expedition. |

The client continues to use the toolkit’s frame-polled HTTP transport. These
endpoints may return errors, delayed outcomes, or event-cursor gaps; the client
must expose recovery and retry actions instead of silently mutating local
state.

## Acceptance test

The prototype succeeds when:

1. three or more players naturally occupy different roles and regroup at the
   tavern without a forced matchmaking step;
2. one wilderness threat creates at least three downstream effects across
   combat, farming, trade, travel, crafting, rumours, or contracts;
3. a player can be knocked out, regain control quickly, and understand the
   consequence and recovery path;
4. an NPC household arrives, provides a limited opportunity, or leaves in
   response to sustained conditions with a visible causal clue;
5. the settlement chronicle records a player/community event and another
   player can read it later;
6. one pioneer expedition can succeed or retreat without deleting characters;
7. the server survives a restart without losing the above durable state; and
8. a soak test reaches the GDD’s initial 10–20 simultaneous-client target with
   movement, chat, clock updates, and event polling still responsive.

## Explicitly deferred

Kingdom-scale governance, elections and taxation, full NPC birth/death/family
simulation, generational succession, PvP law systems, permanent death, broad
world topology/sharding, and the final deployment/authentication architecture
remain later decisions. Phase 3 should prove the world’s ability to create
stories before it expands the number of systems that can create them.

## Exit artifact

The current automated gate is `cargo test --workspace`, the Phase 1–3 HTTP
acceptance scripts, client connection-state fixtures, event-cursor/reconnect
tests, NPC decision and lease-reclamation fixtures, expedition persistence
fixtures, and the concurrent 20-client polling check. Community playthrough
recording and a human soak report are intentionally deferred until the client
is more playable. Production-scale content still requires a design review of
the open GDD questions.
