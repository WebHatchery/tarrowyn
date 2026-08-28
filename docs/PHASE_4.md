# Phase 4 — The Enduring Society

> Implementation status: complete for the bounded first-settlement scope. The
> design lock, runbook, and two-character playthrough are recorded in
> [`PHASE_4_DESIGN.md`](PHASE_4_DESIGN.md), [`PHASE_4_RUNBOOK.md`](PHASE_4_RUNBOOK.md),
> and [`PHASE_4_PLAYTHROUGH.md`](PHASE_4_PLAYTHROUGH.md).

## Purpose

Phase 4 turns the Phase 3 settlement and outpost into a durable local society.
The existing slice proves that threats, households, leases, history, and a
pioneer project can coexist. This phase gives those systems institutional
rules: players can maintain a settlement, hold responsibility for shared
infrastructure, develop meaningful professions, and understand how land and
services change over time.

The phase must begin with a short design-lock pass. The GDD still leaves
governance, lease duration, skill specialisation, crafting interaction,
household lifecycle, and knowledge transfer open. Implementation should not
silently choose rules that would later invalidate player property or
progression. Record each decision, its fallback behaviour, and the migration
impact before adding durable state.

## Build scope

### Settlement governance

- Add a small, explicit governance model for the first settlement: offices,
  deputies or councils, vacancy rules, and the authority boundary for each
  decision.
- Let authorised players propose and complete a limited set of public actions,
  such as road repair, service funding, a festival, a public work, or a local
  contract board change.
- Make public resources and decisions auditable in the settlement chronicle.
  A player must be able to see who changed a rule, what it cost, and what
  service or project it affected.
- Add inactivity and succession behaviour so a missing office-holder weakens
  administration without making the settlement permanently unplayable.
- Use the launch tax contract: the mayor controls a 0–10% rate, nearby carried
  gold is collected once per world day, exemptions and the Hearth territory are
  explicit, and every receipt enters the public governance ledger before it
  can fund a predefined settlement upgrade.

### Land, property, and local infrastructure

- Extend the Phase 3 homestead lease into a complete lifecycle: request,
  approval, renewal, transfer or inheritance, abandonment, reclamation, and
  inspection.
- Separate recognised land rights from stored inventory and personal
  progression. An expired claim may change access to a building without
  deleting a character or silently destroying protected goods.
- Add settlement-owned infrastructure records for the first roads, bridges,
  plots, public buildings, and services. Infrastructure should have condition,
  upkeep, and a readable failure or recovery state.
- Ensure late players can find available land, abandoned opportunities, or a
  path to contribute without competing only for the original homestead.

### Professions, skills, and knowledge

- Replace the Phase 3 placeholder progression with a small number of explicit
  professional capabilities. Skills should unlock techniques, quality,
  reliability, access, or specialisation rather than create a large combat
  power gap.
- Add one complete player-facing crafting or service order loop tied to
  materials, tools, expertise, and demand from another profession.
- Add discoverable knowledge such as a crop technique, monster clue, route,
  recipe, or material property. Decide whether it can be taught, written down,
  traded, or stored in a guild or settlement record, and make that transfer
  server-authoritative.
- Give reputation and credentials a visible use in contracts, offices,
  orders, or access while keeping new characters useful beside veterans.

### Households and local life

- Expand the Phase 3 household from an opportunity score into a bounded
  household state with members, roles, relationships, home, needs, work, and
  service quality.
- Let sustained demand, housing, safety, food, and competition influence
  arrival, investment, reduced service, and departure. Expose causal clues
  before a household leaves.
- Keep the simulation bounded and deterministic enough to test. Full birth,
  death, marriage, and generational population modelling is not required until
  the design review proves it is worth the persistence and support cost.
- Add at least one household whose members provide complementary services so
  the settlement feels inhabited rather than populated by isolated vendors.

### Combat and recovery completion

- Replace the single demonstration encounter with a small, coherent local
  combat loop: readable attack intent, weapon/tool differences, bounded
  injuries, retreat, knockout, recovery, and item-risk rules.
- Keep ordinary defeat reversible and prompt. Stored property remains safe,
  and every loss or recovery cost is shown before the player chooses a path.
- Connect combat outcomes to contracts, household demand, repairs, crafting,
  and settlement safety so combat remains part of society rather than a
  separate arena.

## Server, protocol, and client work

The server remains authoritative for offices, permissions, leases,
infrastructure condition, skills, knowledge, household decisions, combat
outcomes, and all costs. Add stable IDs, request idempotency, event cursors,
and migration coverage for every new durable record.

Suggested additions include:

| Endpoint | Purpose |
|---|---|
| `GET /v1/settlement/governance` | Read offices, vacancies, proposals, permissions, and recent decisions. |
| `POST /v1/settlement/governance` | Propose, vote on, approve, or complete a bounded public action. |
| `GET /v1/infrastructure` | Read public structures, condition, upkeep, and repair needs. |
| `GET /v1/claims` / `POST /v1/claims/lifecycle` | Inspect and advance the complete lease lifecycle. |
| `GET /v1/professions` / `POST /v1/professions/orders` | Read capabilities and create or fulfil one meaningful service order. |
| `GET /v1/knowledge` / `POST /v1/knowledge` | Discover, record, teach, or apply a knowledge item according to the design lock. |
| `GET /v1/households` | Read visible household services, demand clues, and local-life changes. |
| `POST /v1/combat/actions` | Continue the authoritative combat and recovery contract from Phase 3. |
| `GET /v1/combat/local` / `POST /v1/combat/local` | Continue the bounded multi-turn local encounter. |

The client needs visible town-hall, registry, claim, order, knowledge, and
recovery controls. A player must be able to inspect a lease, understand a
public decision, recover from defeat, and complete the core profession loop by
touch alone. No governance or property action may depend on a hidden keyboard
command or an optimistic local mutation.

## Acceptance test

The phase succeeds when a human group can spend several sessions in the first
settlement and:

1. fill or recover a local office and complete an auditable public project;
2. renew, transfer, abandon, and reclaim a test lease without losing
   unrelated character or stored-goods state;
3. complete one craft or service order that materially helps another role;
4. discover and use or teach one knowledge item;
5. observe a household arrive, change service, or leave with a visible cause;
6. complete the local combat, knockout, and recovery loop with clear losses;
7. reconnect after each mutation and see the same governance, property,
   progression, and household state; and
8. keep a new player useful beside an established character.

## Explicitly deferred

Multiple active settlements, regional travel and trade, broad world topology,
realm-wide events, production authentication, public deployment, and final
PvP/law rules remain Phase 5 or Phase 6 decisions. Generational succession is
not a requirement for this phase; it needs a separate design decision rather
than being smuggled into ordinary defeat or household ageing.

## Exit artifacts

- A short design decision record covering governance, taxation if selected,
  leases, skills, crafting, knowledge, household lifecycle, and combat scope.
- Storage migrations and repository fixtures for every new durable rule.
- A human settlement playthrough report covering a new player and an
  established player.
- A `PHASE_4_RUNBOOK.md` with server start, migration, governance fixtures,
  recovery checks, and visual touch-control verification.
