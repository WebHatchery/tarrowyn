# Phase 4 design lock — The Enduring Society

Status: implemented in protocol version 4; its fields are part of storage
version 15 (Phase 4 was first introduced at storage version 14).

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

The town-hall proposal ledger retains at most 64 entries. Completed or rejected
proposals leave room for new work while proposed and approved work remains
addressable; when every retained slot is active, another proposal is rejected
without changing the ledger.

The direct-trade ledger retains at most 128 offers. Pending offers remain
addressable until they are accepted, cancelled, or expired; terminal history is
evicted oldest-first when room is needed. If all retained offers are still
pending, a new offer is rejected without changing either player's inventory.

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
Only a requested or live lease may be abandoned, and only a live lease (active,
renewed, transferred, or inherited) may change hands. Expired, abandoned, and
reclaimed rows cannot be reactivated by a lifecycle command, so their
reclamation grace cannot be postponed by a stale owner.

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
The online client also shows the current player's lease status and remaining
real-time days (or hours near expiry), including the requested and grace-open
states, beside the registry summary.

## Farming equipment and professions

Every recognised character starts with a field tool at condition 3. Active
tending consumes one condition and improves the crop; a worn-out tool blocks
further tending with a readable repair instruction. The existing service-order
loop supplies the recovery path: a completed order whose service repairs a
field tool restores the requesting farmer's condition to 3, while the provider
receives the normal timed-work reward.

The shared field also reports a bounded environmental outlook from the
server-owned day: clear, dry wind, or heavy rain, alongside pest pressure from
0 to 2. On a growth pulse, weather and pests can lower crop quality when the
plot has not been tended recently; recent active tending protects that pulse.
The current outlook is included in the player projection and the touch ledger,
so the player can read why tending matters without the client simulating risk.
The shared fields also include Bellweather, an authored goat whose condition is
persisted in the Phase 4 state. Its condition falls by one at each shared-day
boundary unless cared for, making the visible Care control a repeatable choice;
care restores the bounded condition and records Animal Husbandry practice. The
animal appears in the world snapshot, client map, and farming response.
Breeding, herd size, feed, and wider animal ecology remain documented follow-up
work.

## Professions and knowledge

Every new character begins with a useful Farmer capability and a small material
stock. A character can learn a second capability with a visible credential.
The complete order loop is:

1. a requester escrows wood, iron, and a tool while creating an order;
2. a different player with the required credential accepts it; and
3. that provider completes the visible timing interaction, receives gold and
   skill progress, and records the bounded service quality and benefit for the
   requesting role. A missing timing value remains compatible with older
   clients and receives the neutral midpoint; an explicit score outside the
   visible 0–100 range is rejected by the server.

The server derives the profession, service label, material escrow, reward, and
benefit from the validated recipe manifest. Optional client fields remain
accepted for older clients, but a conflicting profession is rejected and a
custom service label cannot change the authoritative order.

The service-order board retains at most 64 records. Completed or cancelled
history makes room for new work while open and accepted orders remain
addressable; when every retained slot still carries live work, a new order is
rejected before its materials or tool are escrowed.

The land registry retains at most 128 claim records. Reclaimed claim history
makes room for a new request, while active, requested, expired, and abandoned
claims remain addressable until their lifecycle can safely continue. If every
retained row is still live, a new request is rejected before a free plot is
removed from the available-land ledger.

The public decision and tax ledgers retain their newest 64 records, and the
infrastructure view retains its newest 32 records, so a bounded projection
continues to show the latest accepted settlement work and receipts.

The first discoverable knowledge is the Moonberry trellis method. It can be
discovered, written into the guild archive, taught to another account, and
applied. Until the discoverer teaches it or records it, other players receive
only a redacted clue rather than the method's description or effect. A teaching
action must name another recognised player and records
practice in the Teaching root only after the transfer succeeds. The server owns
all discovery and transfer checks; the client cycles its visible control through
the record, teach, and apply actions using the resulting projection.

Formal school lessons use `POST /v1/skills`. A teacher must have mastered the
subject and have Teaching mastery at least equal to its depth; the learner must
be present beside the teacher. The teacher opens a persisted lesson and the
learner joins it through a second visible School action; an abandoned lesson
expires after a bounded window. The active school ledger retains at most 128
lessons and rejects another opening while every retained lesson is active. A
root lesson starts one practice, while an
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

The local encounter has prepare, strike, weapon technique, guard, item use, and
retreat intents. Iron sword, spear, axe, bow, shield, and improvised club are
explicit weapon choices with different readable damage and recovery profiles;
the visible Local fight control cycles through them, while Technique spends the
first exchange on a weapon-specific opening, Guard sends an explicit defensive
intent, Bandage consumes one carried bandage to restore one injury point,
Reposition creates one protected opening for the next strike, and Spell spends
one Wind Spark per encounter for a bounded magical hit that records Wind Magic
practice on victory.
The server records the matching root practice and sword/spear/axe qualifying
history. The threat has bounded health,
the player has bounded health and injuries, and knockout returns the character
to the Hearth. While knocked out, the server rejects local-combat commands,
including direct attempts to prepare a new encounter, until a recovery choice
clears the state. Every active encounter command also remains bound to the
threat's local range; walking away pauses the encounter until the character
returns to Whisperwood Edge. Stored property is always safe. At most one
carried seed is shown as the risk, and the recovery cost is visible before a
recovery choice. Accepted local actions also advance a persisted server-side
action window. The default one-tick window prevents same-tick request bursts
from resolving several actions; `TARROWYN_COMBAT_ACTION_COOLDOWN_TICKS` can
shorten or lengthen that deployment boundary. The response exposes the next
available tick, and the client shows the remaining window beside the action
bar so timing is readable without requiring a keyboard.
Recovery choices are explicit rather than a single generic escape: `Self`
spends one carried seed and reduces one injury, `Rescuer` returns the traveller
with a small reputation gain, and `Healer` clears injuries for the displayed
gold cost. If the carried seed or healer gold is unavailable, the server keeps
the character knocked out and names the remaining visible choices. The shared
recovery endpoint resets the local encounter only after an accepted choice.
When Wind Magic, Water Magic, and Electricity Magic are each mastered, the
same visible Spell control becomes a severe-weather three-element working.
Each encounter can contribute one successful interaction, and 25 such
interactions reveal Storm Magic. After discovery, that touch control becomes
a one-use Storm technique for the encounter with a readable discovery prompt;
the server still owns weather, mastery, discovery, damage, and timing.
The existing Phase 3 combat/recovery contract remains available for the
Brambleback contract; the Phase 4 local endpoint completes the readable
multi-turn loop.

## Persistence and migration

The repository storage version is now 15. `StoredState.phase4` is serde-defaulted
so Phase 1–3 documents without the field load a fresh, safe Phase 4 society.
Existing identities, inventory, plots, Phase 3 claims, events, and chronicle
entries are retained, and older local-combat records default the new
action window to tick zero, with the reposition opening and wind spark closed.
New request caches use stable account/request keys and
are persisted with the new records, so replaying a request returns its original
authoritative result without paying twice.
