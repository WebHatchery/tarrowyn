# Phase 4 design lock — The Enduring Society

Status: implemented in protocol version 4 and storage version 3.

This record closes the open choices identified before Phase 4 work. The rules
are deliberately bounded: one settlement, a small public treasury, a few
capabilities, one order loop, one teachable technique, one complementary
household, and one local encounter.

## Governance and public resources

The Hearth has three offices:

- the Settlement Steward approves and completes every bounded public action;
- the Works Warden owns road, bridge, and public-work completion; and
- the Settlement Registrar owns the contract-board record.

Any player may propose a costed action, so a new character can contribute
before holding office. Completion is still server-authorised by the office
boundary. The first public actions are road repair, service funding, a
festival, a public work, and a contract-board update. Every completion deducts
from the public treasury and writes actor, proposal, cost, target service, and
tick to the governance decision record and settlement chronicle.

There is no player-facing taxation in Phase 4. `taxation` is persisted as
`None`, which keeps payer, recipient, exemptions, accounting, and recovery
unambiguous until a later design review selects a tax. Public upkeep is paid
from the same visible treasury; if it runs dry, infrastructure condition falls
and the failure is recorded.

An office-holder who exceeds `governance_inactivity_ticks` becomes vacant.
Administration quality falls, but the settlement remains usable and any player
can claim the vacant office. This is the succession fallback rather than a
permanent lockout.

## Claims and infrastructure

Phase 4 uses a separate recognised land-right ledger alongside the Phase 3
homestead demonstration. A claim moves through requested, active, renewed,
transferred or inherited, abandoned, expired, and reclaimed states. Building
access is a claim property; the identity, character progression, and protected
stored goods are never deleted by expiry or reclamation.

The registry exposes three available plots at start. Abandoned or expired plots
return after the configured grace period, which gives late players a visible
entry path. The first infrastructure records are the north road, stone bridge,
town hall, Hearth services, and an optional public workshop. Each has condition,
upkeep, service quality, failure state, and last-maintained tick.

## Professions and knowledge

Every new character begins with a useful Farmer capability and a small material
stock. A character can learn a second capability with a visible credential.
The complete order loop is:

1. a requester escrows wood, iron, and a tool while creating an order;
2. a different player with the required credential accepts it; and
3. that provider completes the visible timing interaction, receives gold and
   skill progress, and records the bounded service quality and benefit for the
   requesting role. A missing timing value remains compatible with older
   clients and receives the neutral midpoint.

The first discoverable knowledge is the Moonberry trellis method. It can be
discovered, written into the guild archive, taught to another account, and
applied. A teaching action must name another recognised player and records
practice in the Teaching root only after the transfer succeeds. The server owns
all discovery and transfer checks; the client only shows the resulting
projection.

Formal school lessons use `POST /v1/skills`. A teacher must have mastered the
subject and have Teaching mastery at least equal to its depth; the learner must
be present beside the teacher. A root lesson starts one practice, while an
advanced lesson grants only the discovery and leaves mastery and all personal
requirements to the learner. The current client offers mastered roots through
the visible School control; the server remains the authority for future school
membership, tuition, halls, and advanced lesson policy.

Formal school lessons use `POST /v1/skills`. A teacher must have mastered the
subject and have Teaching mastery at least equal to its depth; the learner must
be present beside the teacher. A root lesson starts one practice, while an
advanced lesson grants only the discovery and leaves mastery and all personal
requirements to the learner. The current client offers mastered roots through
the visible School control; the server remains the authority for future school
membership, tuition, halls, and advanced lesson policy.

## Households and local life

The Bellweather household has a miller and an herbal healer. Their work is
complementary: grain and field planning support food demand while bandages and
recovery advice support safe work. On a fixed decision interval the server
updates demand, housing, safety, food, competition, and service quality. Poor
conditions progress through reduced service and a departure warning before
departure. A clue is updated with the causal condition each time. The model is
bounded and has no birth, death, marriage, or generational persistence.

## Combat and recovery

The local encounter has prepare, strike, guard, and retreat intents. Iron sword
and improvised club damage differ; the threat has bounded health, the player has
bounded health and injuries, and knockout returns the character to the Hearth.
Stored property is always safe. At most one carried seed is shown as the risk,
and the recovery cost is visible before a recovery choice. The existing Phase 3
combat/recovery contract remains available for the Brambleback contract; the
Phase 4 local endpoint completes the readable multi-turn loop.

## Persistence and migration

The repository storage version is now 3. `StoredState.phase4` is serde-defaulted
so Phase 1–3 documents without the field load a fresh, safe Phase 4 society.
Existing identities, inventory, plots, Phase 3 claims, events, and chronicle
entries are retained. New request caches use stable account/request keys and
are persisted with the new records, so replaying a request returns its original
authoritative result without paying twice.
