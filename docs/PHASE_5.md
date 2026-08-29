# Phase 5 — The Roads Between

## Purpose

Phase 5 expands Tarrowyn from one durable settlement into a connected region.
It proves that settlements can grow, decline, specialise, exchange goods and
people, and respond to the same changing world without collapsing into one
global task list.

Phase 4 is the dependency: the regional model must reuse the tested rules for
governance, leases, households, professions, and chronicle entries. Phase 5
should multiply a proven local society, not introduce three incompatible
versions of it.

## Build scope

### Regional map and travel

- Define a small authoritative region containing the original settlement, the
  Phase 3 outpost, and at least one additional settlement or meaningful
  frontier site with a different role or resource.
- Add server-owned travel between locations, including route length, travel
  risk, arrival, interruption, and recovery. A client may render an intended
  route but cannot grant arrival locally.
- Add transport and logistics in a deliberately small form: a road, bridge,
  caravan, boat, or pack route that can be repaired, delayed, threatened, or
  improved.
- Introduce region or location interest management so clients receive the
  nearby world they need without polling every entity in every settlement.
- Define how a player, trade, event cursor, and durable command behave while
  crossing a location boundary or reconnecting during travel.

### Settlement growth and decline

- Let population, food, safety, infrastructure, industry, governance, and
  player activity change a settlement’s condition over time.
- Use descriptive settlement states or milestones rather than a mandatory
  level ladder. Growth should emerge from what the community actually built.
- Give decline readable stages, vacancies, and recovery opportunities. A
  quiet settlement should become a historical and economic opportunity, not an
  instant dead end.
- Allow each settlement to have different demand, services, prices, claims,
  public works, and chronicle history while sharing the same domain rules.

### Regional economy and exchange

- Add cross-settlement orders, imports, exports, caravans, and substitute goods
  so regional distance matters to prices and availability.
- Add at least one material, profession, or service that is locally scarce and
  another that is locally abundant. Players should have a reason to plan a
  journey or organise a shipment.
- Keep NPC and travelling-service fallback capacity limited, slower, more
  expensive, or less specialised than a successful player supply chain.
- Add sinks and maintenance that prevent persistent production from making
  common goods worthless. The first regional economy needs explicit telemetry
  for stock, prices, demand, and failed fulfilment.

### Dynamic events and ecology

- Replace the single authored threat with a small event lifecycle: signal,
  escalation, intervention, resolution, and aftermath.
- Allow one event to cross settlement boundaries and affect at least three of
  travel, farming, crafting, prices, households, contracts, rumours, safety,
  or governance.
- Add a seasonal or regional condition that changes opportunity without hard
  locking essential services. The exact real-time calendar length must be
  decided and recorded before it affects leases or crop promises.
- Make event causes, interventions, and outcomes part of the chronicle so a
  later player can learn what the region remembers.

### NPC movement and pioneer networks

- Let households evaluate opportunities across the connected region rather
  than only inside one settlement. A move should have a visible reason and a
  durable history.
- Let player groups supply, repair, defend, or abandon frontier sites while
  preserving character safety and recoverable failure.
- Ensure a new player can enter the region through an open settlement, a
  service vacancy, a caravan role, or a frontier opportunity even when the
  original settlement is well established.

### PvP and law decision

The GDD leaves PvP, crime, theft, and law enforcement open. Phase 5 must make
an explicit product decision before regional travel and property are exposed:

- If PvP is selected, implement a bounded, opt-in ruleset with consent,
  protected spaces, evidence, consequences, reporting, and recovery.
- If PvP is not selected, document the boundary and protect the economy,
  claims, and travel systems from accidental player-versus-player ownership
  or theft mechanics.

The phase is not complete with an ambiguous mixture of both behaviours.

## Server, protocol, and client work

The server remains authoritative for location, travel, logistics, settlement
condition, regional prices, event resolution, household movement, and any law
rules. The protocol needs location-aware snapshots and cursors, durable travel
commands, event IDs, and idempotent trade/order operations.

Suggested additions include:

| Endpoint | Purpose |
|---|---|
| `GET /v1/region` | Read locations, routes, visible conditions, and the player’s current travel state. |
| `POST /v1/travel` | Start, interrupt, resume, or complete an authoritative journey. |
| `GET /v1/settlements` | Read regional settlement projections, access, demand, and visible decline/growth signals. |
| `GET /v1/routes` / `POST /v1/routes` | Inspect and act on roads, bridges, caravans, or other regional logistics. |
| `GET /v1/market/orders` / `POST /v1/market/orders` | Place, fulfil, cancel, and settle cross-settlement orders. |
| `GET /v1/events/region` | Read regional event signals, interventions, outcomes, and aftermath. |
| `GET /v1/households/region` | Read visible migration and service changes across settlements. |
| `POST /v1/law` | Use only if the Phase 5 PvP/law decision selects a bounded law system. |

The client needs a touch-capable regional map, travel confirmation and
recovery controls, route/market views, settlement comparison, event notices,
and clear boundary/loading states. The shared-road sidebar now keeps each
settlement's condition and open-vacancy count visible beside the local
recovery signal, while chronicle context remains on the next line. Crossing a
region must not freeze the render loop or silently discard an accepted command.
The same sidebar now exposes compact authoritative road availability and risk,
open market orders, and the protected-law boundary, so the Travel and Market
controls have visible regional telemetry. Its visible Inspect control opens
route names/status/condition/risk plus the first stock and price notes without
displacing the touch recovery controls.
It also keeps the latest regional event stage visible beside that telemetry, so
an event signal or escalation remains readable between refreshes and its Event
control remains discoverable.
The map overlay uses the authoritative location positions and route statuses
when the regional projection is loaded, while the offline fixture retains its
local landmarks.
The visible Repair control queues the existing authenticated route action for
the current fixture road, so recovery work has a touch path rather than an
unreachable client-only command.
The client also polls the regional household projection and shows a compact
travelling-service status beside the road, market, law, and event telemetry.
The online header also shows the authoritative regional season beside the
calendar day; the development cadence remains subject to the Phase 6 pacing
decision.

## Acceptance test

The phase succeeds when a group can:

1. travel between at least three meaningful locations and recover from an
   interrupted journey;
2. move a useful good or service across a route and observe the authoritative
   inventory, price, and order changes;
3. see two settlements with different conditions, opportunities, and
   historical records;
4. help a settlement grow or recover, or witness a signposted decline and
   later reclamation opportunity;
5. observe a household or travelling service choose a regional opportunity;
6. respond to one cross-settlement event with at least three downstream
   effects;
7. reconnect during travel or after a regional event without duplicated
   rewards or lost durable state; and
8. complete the chosen PvP/law boundary test, including the protected path if
   PvP is not selected.

The automated gate should include multiple settlement fixtures, route and
market idempotency, event-cursor recovery, household migration, and a soak
that exceeds the Phase 3 twenty-client check without requiring every client to
receive every regional entity.

## Explicitly deferred

Global-scale world topology, final shard or instancing architecture, public
authentication, production deployment, cross-region disaster recovery, and
optional generational legacy systems remain Phase 6. Phase 5 is a regional
proof, not a promise of an unlimited seamless world.

## Exit artifacts

- A region map and topology decision record covering location boundaries,
  travel handoff, interest management, and cursor recovery.
- An economy and calendar decision record covering route costs, sinks, market
  telemetry, and seasonal timing.
- A written PvP/law decision with the corresponding acceptance fixtures.
- A human regional playthrough and decline/recovery report.
- A future `PHASE_5_RUNBOOK.md` covering multiple server locations or region
  fixtures, travel recovery, market inspection, event seeding, and soak-test
  collection.
