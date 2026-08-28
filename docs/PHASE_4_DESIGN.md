# Phase 4 design lock — The Enduring Society

Status: implemented in protocol version 4 and storage version 9.

This record closes the open choices identified before Phase 4 work. The rules
are deliberately bounded: one settlement, a small public treasury, a few
capabilities, one order loop, one teachable technique, one complementary
household, and one local encounter.

## Governance and public resources

The Hearth has three offices. The Settlement Steward is the bounded launch
mayor role:

- the Settlement Steward approves and completes every bounded public action;
- the Works Warden owns road, bridge, and public-work completion; and
- the Settlement Registrar owns the contract-board record.

Any player may propose a costed action, so a new character can contribute
before holding office. Completion is still server-authorised by the office
boundary. The first public actions are road repair, service funding, a
festival, a public work, and a contract-board update. Every completion deducts
from the public treasury and writes actor, proposal, cost, target service, and
tick to the governance decision record and settlement chronicle.

The launch mayoral loop also posts a narrow public settlement tax. The default
policy charges 5% of carried gold once per world day from recognised players
within four Manhattan tiles of the Hearth; the policy never removes items and
exempts players outside the territory or currently knocked out. The mayor can
cycle the rate through 0%, 5%, and 10% from the visible touch control, while
the server rejects every value outside the 0–10% bound. No mayor means no new
collection, so leadership failure weakens income without silently draining
players. Each receipt records payer, amount, rate, territory, day, and tick in
the public governance ledger, while the policy records payer, recipient,
exemptions, accounting note, and recovery path. Public upkeep and predefined
upgrades are paid from the same visible treasury; if it runs dry,
infrastructure condition falls and the failure is recorded.

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

An approved lease lasts 90 real days (the prototype's explicit three-real-month
boundary), stored as Unix timestamps rather than accelerated world ticks. The
world clock may advance through seasons and years without shortening the lease.
Expiry closes building access and starts the configured visible reclamation
grace; the expiry transition resets that grace window so an expired claim is not
reclaimed in the same tick.

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
be present beside the teacher. The teacher opens a persisted lesson and the
learner joins it through a second visible School action; an abandoned lesson
expires after a bounded window. A root lesson starts one practice, while an
advanced lesson grants only the discovery and leaves mastery and all personal
requirements to the learner. The current client offers mastered roots through
the visible School control and surfaces open lessons in the skill projection;
the server remains the authority for future school membership, tuition, halls,
and advanced lesson policy.

Every depth-one catalogue entry also has a dependable first-practice path. A
visible Practice control selects the next unstarted root and sends
`SkillAction::Practice`; the server records it in the same ledger used by
farming, travel, combat, trade, and profession activity. This is an entry
path, not a replacement for the richer tools, worksites, encounters, and
teachers that later raise mastery.

## Households and local life

The Bellweather household has a miller and an herbal healer. Their work is
complementary: grain and field planning support food demand while bandages and
recovery advice support safe work. On a fixed decision interval the server
updates demand, housing, safety, food, competition, and service quality. Poor
conditions progress through reduced service and a departure warning before
departure. A clue is updated with the causal condition each time. The model is
bounded and has no birth, death, marriage, or generational persistence.

## Combat and recovery

The local encounter has prepare, strike, guard, and retreat intents. Iron sword,
spear, axe, bow, shield, and improvised club are explicit weapon choices with
different readable damage and recovery profiles; the visible Local fight
control cycles through them and the companion Guard control sends an explicit
defensive intent, while the server records the matching root practice and
sword/spear/axe qualifying history. The threat has bounded health,
the player has bounded health and injuries, and knockout returns the character
to the Hearth. Stored property is always safe. At most one carried seed is
shown as the risk, and the recovery cost is visible before a recovery choice.
The existing Phase 3 combat/recovery contract remains available for the
Brambleback contract; the Phase 4 local endpoint completes the readable
multi-turn loop.

## Persistence and migration

The repository storage version is now 9. `StoredState.phase4` is serde-defaulted
so Phase 1–3 documents without the field load a fresh, safe Phase 4 society.
Existing identities, inventory, plots, Phase 3 claims, events, and chronicle
entries are retained. New request caches use stable account/request keys and
are persisted with the new records, so replaying a request returns its original
authoritative result without paying twice.
