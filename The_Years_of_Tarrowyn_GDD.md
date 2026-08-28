# The Years of Tarrowyn

## Game Design Document

**Version:** 0.3 - Traditional MMORPG Systems
**Status:** Working design  
**Platform/technology baseline:** 2D multiplayer client + authoritative server, developed in Rust

> A cozy, slow-burn fantasy MMORPG about living in a persistent society where communities, professions, settlements, and history matter more than racing to a level cap.

## Decision Status

| Area | Status | Current Decision |
|---|---|---|
| Working title | Locked for project | The Years of Tarrowyn |
| Genre | Locked | Cozy slow-burn 2D MMORPG / persistent fantasy society |
| Technology | Locked | Rust; separate authoritative server and client from the first slice |
| Database | Locked | MySQL; preview connection settings come from ignored `.env.preview` configuration |
| Progression | Direction locked | Classless, horizontal skill mastery with hidden skill mergers; no conventional global level |
| Skill teaching | Direction locked | Players can form schools, share discoveries, and unlock direct teaching through teaching expertise |
| World time | Locked | 80 real minutes per in-game day; season and year scale deferred |
| Normal defeat | Locked | Characters cannot die; defeat is knockout with item/goods risk |
| Settlements | Direction locked | Organic growth and decline, potentially homestead to kingdom |
| NPCs | Launch locked | Fixed NPCs; family simulation, aging, migration, and career changes deferred |
| Land | Locked | Renewable three-real-month leases rather than permanent first-come ownership |
| Frontier | Direction locked | Player-led pioneer expeditions and new settlements |
| Regional history | Direction locked | Recent chronicle display with durable searchable archive and deletion-safe public records |
| Combat | Direction locked | Real-time action-bar combat; melee attacks and spells are explicit skill uses |
| PvP | Launch locked | No launch PvP; criminal play is a future option only if it cannot spoil non-participants' play |
| Zone population | Direction locked | Several hundred concurrent players per zone instance |
| Crafting | Direction locked | Short timing minigames influence quality |

## 1. Executive Summary

### High Concept

The Years of Tarrowyn is a cozy, slow-burn, traditional 2D MMORPG about living in a persistent medieval fantasy society rather than racing through a conventional MMO progression ladder. It retains the familiar pleasures of inhabiting a shared world, fighting, gathering, crafting, trading, exploring, and building a reputation, but replaces fixed classes and a conventional level ladder with an open-ended skill system.

Players may become farmers, craftspeople, traders, explorers, adventurers, community leaders, or drift between several callings. The world is intended to move on a long timescale: a remote homestead could eventually become a town or kingdom, while a once-busy settlement could decline and be abandoned if its community disappears.

The smallest playable slice is still a real multiplayer game. It begins with a Rust server, a Rust client, persistence, and multiple clients connected to the same world.

### Core Promise

Players should feel that other people matter. A farmer, blacksmith, adventurer, innkeeper, miner, tailor, and mayor should each create opportunities for the others.

Progress is deliberately slow. The reward for playing on an ordinary evening is not necessarily a level increase. It may be a useful trade, a new piece of knowledge, a repaired bridge, a successful harvest, a returned quest, a new friendship, or hearing that a group is preparing to settle beyond the frontier.

## 2. Design Pillars

### The World Moves Slowly

Progress is measured across days, weeks, months, and eventually years. Large goals should have weight because they take time, but each session still needs small satisfying outcomes.

### Veterans Become Broader, Not Infinitely Stronger

Older characters accumulate skills, techniques, knowledge, relationships, property, reputation, and access. Raw combat power should remain compressed enough that new and veteran characters can still adventure together.

### Discovery Replaces Class Choice

Characters never choose a permanent class. They become distinctive through the skills they practise, the combinations they uncover, the experiences they accumulate, and the people from whom they learn. The system should reward experimentation and shared knowledge without requiring every player to follow a published optimal build.

### Professions Matter to Other Players

Farming, crafting, trade, exploration, and combat are not isolated minigames. They should feed one another through demand, supply, information, risk, infrastructure, and settlement growth.

### Time Changes Texture, Not Basic Access

Morning, afternoon, evening, and night should alter opportunities, atmosphere, NPC behaviour, and social rhythms without routinely locking players out because they can only log in at a particular real-world time.

### The World Creates Stories

Events should ripple through several systems. A monster problem can affect roads, farming, prices, hunting, crafting demand, tavern rumours, and local government rather than existing only as a combat encounter.

### The Game Allows Non-Progress

Fishing, talking in a tavern, helping a neighbour, travelling with a caravan, or attending a local event should not feel like wasted playtime simply because no progression bar moved.

## 3. Player Fantasy and Session Rhythm

### Typical Farmer Session

Check crops and animals, plant, weed, maintain tools or equipment, trade or arrange supplies, then head into town or the tavern as the working day ends.

### Typical Adventurer Session

Take a contract or follow a rumour, travel to the destination, deal with danger or complete the task, return to report it, recover, and spend the evening in a social space.

### Typical Craftsperson Session

Acquire or arrange materials, hand-craft a small number of meaningful items, maintain equipment or workshop needs, fulfil orders, and finish the day in town.

### Session Goal

A 60-90 minute session should produce at least one memorable result even when it produces no major advancement. The daily rhythm should make the world feel inhabited rather than optimized.

## 4. World Time and Calendar

### Accelerated Day

One in-game day lasts exactly 80 real minutes. Because 80 minutes does not divide evenly into 24 hours, a player who logs in at the same real-world time each day encounters a different part of Tarrowyn's day instead of being trapped in the same daylight or night-time window.

### Time-of-Day Design

Different times should provide different flavour and opportunities. Shops, NPC routines, wildlife, rumours, lighting, travel risk, and social activity may change.

Time should rarely hard-lock essential systems. If a blacksmith is unavailable overnight, an apprentice, order system, travelling service, or next-day collection can preserve access.

### Long-Term Calendar

Seasons and years are intended to matter eventually, particularly for farming, settlement history, and long-term personal goals. Their exact number of in-game days remains deferred until crop cadence and access can be tested. Land leases use real time and are not shortened by the accelerated calendar.

## 5. Progression

### No Classes or Conventional Character Level

The game has no class system and should avoid a single global level as the main measure of worth. A character is not a warrior, mage, or farmer because a creation-screen choice says so; the character becomes capable through practised skills, mastered techniques, credentials, discoveries, and relationships.

Players may freely move between combat, magic, gathering, farming, crafting, social, exploratory, and civic skills. Earlier choices can create identity and opportunity costs, but should not permanently lock a character out of another path.

### Skill Progression

Skills should unlock capabilities, techniques, specialisations, efficiency, quality, and breadth rather than only increasing numerical power.

A numerical 1-100 skill treadmill should not simply recreate character levels under a different name. Skill mastery should increasingly depend on practice, teachers, discoveries, tools, demonstrations, and meaningful milestones.

Individual skills begin with concrete activities or bodies of knowledge, such as Sword Fighting, Spear Fighting, Axe Fighting, Wind Magic, Water Magic, Electricity Magic, Crop Tending, Animal Husbandry, or Teaching. Mastery means demonstrated experience with that skill, not merely spending a generic point currency.

### Skill Depth

Every skill has a depth from one to five. Depth describes how many layers of understanding are combined in the skill, not the character's numeric proficiency with it:

- depth one: a root discipline that can be learned directly;
- depth two: an initial merger or advanced specialisation;
- depth three and four: increasingly specialised mergers built from established disciplines;
- depth five: a rare capstone art combining several mature branches.

A depth-five skill is not automatically five times stronger than a root skill. Depth measures complexity, prerequisite breadth, and teaching difficulty. A deep skill should usually be narrower, more demanding, or more situational as it becomes more capable.

### Launch Root Skill Catalogue

The initial full-game catalogue should establish these depth-one families. The smallest prototype may expose only a representative subset, but every listed root has a direct entry path and does not depend on guessing a hidden merger.

- Combat: Unarmed Fighting, Sword Fighting, Axe Fighting, Spear Fighting, Bow Fighting, and Shield Use.
- Magic: Wind Magic, Water Magic, Electricity Magic, Fire Magic, Earth Magic, and Restoration Magic.
- Gathering and wilderness: Foraging, Forestry, Mining, Fishing, Hunting, Survival, and Navigation.
- Farming: Crop Tending and Animal Husbandry.
- Production: Carpentry, Smithing, Tailoring, Cooking, Alchemy, Masonry, and General Crafting.
- Social and civic: Trade, Teaching, and Leadership.

Roots are learned through an obvious first action, introductory trainer, tool, worksite, book, attunement, or short quest. The entry method should fit the fiction but must not strand a player behind an undisclosed combination.

Carpentry is therefore a root skill. A player can learn it by receiving or improvising basic carpentry tools and completing an introductory project at a woodworking bench, with an NPC lesson or beginner manual as dependable alternatives. Axe Fighting concerns combat and is not a Carpentry prerequisite; Forestry supplies timber, while Carpentry turns prepared timber into structures, fittings, tools, and furniture. Future skills such as Timbercraft may merge Forestry, Carpentry, and relevant axe experience without blocking entry to ordinary Carpentry.

### Initial Advanced Discoveries

The first internal merger catalogue should include at least two depth-two discoveries:

- Weapon Fighting requires mastery of Sword Fighting, Spear Fighting, and Axe Fighting plus 100 qualifying enemy defeats across those weapon families, with at least 20 defeats credited to each. It improves switching, stance transfer, and reading unfamiliar melee weapons rather than multiplying damage.
- Storm Magic requires mastery of Wind Magic, Water Magic, and Electricity Magic plus 25 successful three-element interactions performed during severe weather. It grants deliberate storm techniques with substantial preparation and resource demands.

These recipes are internal design truth, not automatically player-facing documentation. Additional advanced skills are added when gameplay creates a useful role for them; the launch catalogue does not need speculative mergers merely to fill a tree.

### Hidden Skill Mergers

Compatible skills can merge into broader, hybrid, or advanced skills. The initial examples are Weapon Fighting from three mastered melee-weapon disciplines and Storm Magic from mastered Wind, Water, and Electricity Magic. Future mergers may use fewer, more, or already-merged ingredients.

These examples express the system's shape, not guaranteed launch recipes. A merger may require any combination of:

- possession or mastery of prerequisite skills;
- a particular amount or kind of practice, such as defeating 100 enemies or harvesting 1,000 crops;
- performing the relevant skills in the same encounter, place, weather, season, or sequence;
- a discovery, mentor, text, rare event, tool, material, reputation, or community achievement;
- a server milestone or content release that makes the merger available in the world.

A merged skill is a new capability rather than a replacement for its ingredients. The source skills retain their identity and uses. Mergers may themselves become ingredients in later mergers, allowing the content library to expand without adding classes or raising a global level cap.

### Discovery and Secrecy

The game does not publish a complete recipe list. Players should discover combinations through experimentation, observation, rumours, records, mentorship, and play. Requirements may be partly signposted so discovery feels mysterious but not arbitrary: a character can sense that two practices resonate, hear an NPC theory, or see progress described in natural language without receiving the entire formula.

Hints use three broad stages: no special hint before the character has relevant experience; a vague sense of resonance once a meaningful prerequisite is mastered; and a statement that further experience or circumstances are required once the known disciplines align. Hidden activity thresholds are not shown as exact numbers by default.

The server evaluates unlocks authoritatively and records the qualifying history needed by each character. Progress counters may be hidden, approximate, or revealed through suitable knowledge skills; the design should avoid turning every mystery into a visible checklist.

Discovery belongs to the persistent world. Some valid-looking combinations may not exist yet. New content can add a merger later, at which point the server should evaluate existing character history where practical. A veteran who already satisfied durable requirements should not need to repeat years of play merely because the recipe was added later.

Unknown combinations must fail gracefully. Experimenting should still exercise and improve the component skills where appropriate, rather than consuming rare resources solely to return an opaque failure message.

### Balance and Combinatorial Control

The merger system should be data-driven and expandable, but not generate every mathematical combination automatically. Designers add intentional mergers with a clear fantasy, gameplay role, prerequisite logic, and balance budget. This keeps discoveries meaningful and prevents the number of shallow skills from exploding.

Advanced skills primarily add new options, synergies, mastery expression, and social value. They should not stack into unchecked numerical power. Prerequisite breadth, situational strengths, equipment needs, preparation, counters, and diminishing returns keep specialised veterans valuable without making newer players irrelevant.

### Player Schools and Teaching

Players can form schools devoted to a fighting tradition, magical discipline, craft, profession, philosophy, or unusual combination of skills. A school is a social institution rather than a class-selection interface. It can maintain membership, teachers, a physical hall or meeting place, lessons, records, reputation, and its own decision about which discoveries are public, reserved, traded, or taught only after a trial.

At the basic level, players teach by sharing clues, demonstrating techniques, practising together, and helping a learner satisfy the normal merger requirements. Learning Teaching and developing it to a high level can unlock formal instruction. A qualified teacher may then teach certain advanced skills directly, bypassing some discovery steps, but not necessarily all mastery, experience, tool, moral, or world-state requirements.

Direct teaching should require meaningful participation from both teacher and student and may take time, suitable facilities, materials, demonstrations, or repeated lessons. It must not become an instant menu transaction or a universal shortcut. Each skill definition specifies whether it is directly teachable and which requirements teaching can substitute for.

The default rule is that a teacher must have mastered the subject and have Teaching mastery at least equal to the subject's depth. Lesson duration and required demonstrations then scale with skill complexity and the teacher's proficiency above that minimum. A character with depth-one Teaching cannot directly teach a depth-five merged skill even if they possess that skill. Learners must still meet any requirements marked personal or non-teachable.

Schools turn player knowledge into social and economic content. Renowned schools may recruit apprentices, charge tuition, exchange secrets, publish manuals, compete over doctrine, split into rival traditions, or become part of a settlement's identity. A school's greatest advantage is organised knowledge and trusted instruction, not exclusive permanent ownership of a skill.

### Content Expansion Contract

Skill content should be represented as server-owned definitions containing stable identifiers, prerequisites, qualifying counters or events, unlock rules, teaching rules, player-facing hints, and version information. Adding a merger must not require changing character classes or invalidating existing builds.

The game may preserve a character's private discovery date, teacher or school lineage, and first-discovery context. World-first or school-first discoveries can become chronicle events when appropriate, but should not reveal the recipe automatically unless the discoverer chooses to share it.

### Adventurer Ranks

Adventurer ranks must be earned over time and represent credibility, not raw XP.

Ranks should use varied credentials rather than a single grind target. A promotion might require a body of completed contracts plus expeditions, guild trials, endorsements, dangerous encounters, exploration, or helping other adventurers.

The early example of '100 quests before an upgrade' establishes the desired slowness, but a literal repetitive quest count is not a final design.

### Horizontal Progression

Veterans should have more options, specialisations, knowledge, assets, social standing, and access, but should not become orders of magnitude stronger than new players.

This keeps late arrivals relevant and allows mixed-experience groups to play together.

### Progression Timescales

- Minutes: complete a task, harvest something, craft an item, trade, discover a rumour, meet another player.
- Days or weeks: improve a farm, gain professional recognition, learn a technique, establish reliable trade, develop a local relationship.
- Months: gain a major guild rank, own a developed workshop, establish a settlement, become a recognised breeder or craftsperson.
- Years: dynasties, kingdoms, historic settlements, legendary items, political legacy, and world history.

## 6. Professions and Interdependence

### Profession Philosophy

A profession is successful when another player has a reason to care that someone chose it.

NPCs provide resilience for low-population periods, but the best outcomes should come from player participation.

### Farming

Farming includes planting, weeding, tending animals, maintaining equipment, responding to weather or pests, and making long-term choices about land and production.

Crops may continue growing while the player is offline, but active farming should improve quality, yield, reliability, or resilience so logging out is not the optimal form of play.

### Crafting

Crafted equipment and tools should be valuable enough that losing or replacing them matters. A craftsperson should normally produce a small number of meaningful items rather than hundreds of disposable copies.

Crafting depends on material supply, local industry, orders, expertise, tools, and possibly learned techniques.

Most crafting actions include a short timing minigame, usually asking the player to tap a visible control while a moving indicator crosses a target area. Materials, tools, recipe knowledge, and skill establish the possible quality range; execution nudges the result within that range. The minigame should reward attention without making latency, motor precision, or one missed tap destroy valuable materials. Accessibility options can widen targets or offer a slower equivalent interaction.

### Adventuring

Adventuring is a profession within society rather than the default path every player must follow.

Adventurers take contracts, investigate threats, explore, escort, recover resources, gather information, and protect routes or settlements.

### Cross-System Example

A wyvern settles near a farming region. Wildlife is displaced into fields, crop losses increase, food prices rise, hunters find more work, the tavern posts contracts, blacksmiths receive demand for suitable equipment, and adventurers eventually investigate the source. The event becomes economic and social content as well as combat content.

This kind of player-driven threat is a far-future direction, not a current implementation concern. When the core world is mature enough, major threats should arise from or respond to accumulated player activity and should be resolved primarily through player preparation and action. Procedural event generation rules should not be designed until the ordinary combat, economy, profession, and settlement loops provide the necessary causes and consequences.

## 7. Economy and Item Value

### Item Importance

Equipment and goods should carry enough value that acquisition, maintenance, trade, damage, theft, and replacement create meaningful stories.

A farmer losing a good hoe should be inconvenienced and may need to borrow, improvise, repair, buy, or commission another. An adventurer without a sword may temporarily rely on a dagger, spear, club, borrowed weapon, or lower-quality substitute.

### Economic Sinks

Persistent economies need consumption. Food, potions, seeds, building materials, workshop inputs, maintenance materials, expedition supplies, festivals, and NPC settlement demand can remove goods from circulation.

Immortal characters do not require sleep, food, or drink to remain alive and playable. Beds, meals, and drinks may provide comfort, temporary benefits, social rituals, role-play, crafting demand, or recovery options, but the economy must not pretend that immortal characters have unavoidable biological upkeep. Durable simulation should focus on goods players actually produce, move, use, damage, improve, build with, or choose to consume.

Durability should not become constant busywork. Maintenance and occasional damage are preferred over rapid equipment decay.

### NPC Baseline vs Player Excellence

NPC services should keep settlements functional but generally be more limited, expensive, slower, less specialised, or less customisable than strong player-run services.

NPCs fill cracks in the economy rather than replacing the incentive to become a profession.

The economy should be simulated in detail where transactions create visible fantasy-world consequences: inventories, local supply, orders, prices, transport, production inputs, maintenance, and settlement projects. Invisible background life can be abstracted when simulating it would add server cost without creating player decisions. Plausibility and useful interaction matter more than modelling a modern economy or inventing needs the characters do not have.

## 8. Defeat, Risk, and Recovery

### Real-Time Combat

Combat plays in real time. Every meaningful action is a skill use selected from visible touch/click controls: a basic melee attack, weapon technique, block, movement skill, item use, or spell cast. A physical keyboard may provide shortcuts, but the complete combat loop must remain playable from the on-screen action bar.

Weapons, positioning, timing, range, cast time, recovery, resources, and enemy intent create tactical decisions without changing combat into a turn-based exchange. The authoritative server validates targets, timing, resources, movement, hits, effects, and defeat; the client presents immediate readable feedback and prediction only where it cannot decide outcomes unfairly.

### Normal Defeat

Characters cannot permanently die. Combat defeat results in being knocked unconscious and may cause carried items or goods to be lost, scattered, damaged, or taken according to the encounter.

Unconsciousness must not require the player to sit unable to act for a long period. Recovery should return control promptly while preserving consequences.

### Consequences

- Carried goods may be lost, stolen, scattered, or damaged. Carried equipment may also be damaged or taken depending on circumstances.
- Owned property, stored goods, land rights, and the majority of long-term assets should generally remain safe unless a separate system explicitly places them at risk.
- Other possible consequences include injury, recovery costs, being transported elsewhere, or owing a rescuer.

### Social Recovery

Defeat can create player interaction. Another player, caravan, guard, or NPC may recover an unconscious character and return them to safety.

Risk should therefore generate stories rather than merely impose a timer.

### Generational Characters

Generational succession is not part of the character model. Persistent identity, history, and relationships belong to the same immortal character; no death or heir system is required.

## 9. Settlements and Player Society

### Organic Settlement Growth

A remote homestead can potentially become a hamlet, village, town, city, duchy, or kingdom, but these are descriptions of what the place has become rather than mandatory 'settlement levels'.

Growth emerges from population, infrastructure, food, trade, safety, industry, governance, and player activity.

### Decline and Abandonment

Settlements may also shrink. If players leave and no one takes responsibility, businesses can close, NPC families can migrate, infrastructure can deteriorate, and a former town may eventually become an abandoned site.

Decline should create new problems and opportunities rather than instantly making a location unplayable.

### Settlement as History

A settlement should preserve a record of what happened there. Buildings, roads, ruins, registries, former residents, names, and major events can create genuine server history.

A later player may discover the remains of a settlement that other players founded years earlier.

## 10. Governance

### Offices, Not Permanent Ownership

Leadership positions such as mayor should belong to the settlement as an office rather than being a permanent personal privilege.

If a mayor disappears, deputies, councils, elections, or vacancy rules can allow another player to step in.

### Leadership Failure

If nobody takes over, administration may gradually weaken. Taxes, public maintenance, NPC confidence, or services can suffer.

An ungoverned settlement is not automatically a failed game state. Rebuilding or taking responsibility can become player-driven content.

### Exact Rules

Launch governance stops at a town mayor. The mayor receives settlement taxes from nearby eligible players into a public town treasury and may spend that treasury on predefined settlement upgrades. Tax rates, eligible territory, treasury income, purchases, and office actions must be visible in a public ledger.

Deeper government structures, elections, councils, inheritance, criminal law, and wider authority are not designed in advance. They should be discovered and specified only when the growing world demonstrates a need for them. Until then, the mayoral office must have bounded powers and cannot take items directly from players or spend public funds outside settlement systems.

## 11. Land, Property, and Leases

### Recognised Claims

Mechanically important land should not be owned forever on a first-come-first-served basis.

Land ownership is better represented as a recognised claim or lease recorded by a settlement, realm, guild, or other authority.

### Aging Ownership

Each land lease lasts three real months and must be renewed before expiry. The accelerated in-game calendar does not affect this duration.

An expired lease enters a clearly announced grace and reclamation process rather than deleting property instantly. Exact renewal price, warning cadence, treatment of improvements, and inheritance are content and policy details, but continued control always requires renewal.

Abandoned buildings should not necessarily vanish immediately. Neglected property can become part of the world's history and may later be restored or reclaimed.

### Late Player Access

The world should be large enough that players who dislike an established region can travel outward and create new opportunities rather than being permanently locked out by early settlers.

## 12. Pioneer and Frontier System

### Player-Led Expansion

Players can choose to leave established settlements, gather companions, load supplies, and travel into less-developed territory to create a new camp or homestead.

Expansion should require logistics. Food, tools, seed, animals, construction materials, defence, transport, and skilled people make a pioneer expedition viable.

### Social Recruitment

A player should be able to announce an expedition and encourage others to come. New settlements are therefore social projects rather than automatically generated unlocks.

### Failure and Recovery

A poorly prepared expedition can struggle or fail without deleting the players. Lack of food, tools, professions, safety, or suitable land can force retreat, improvisation, trade, or a call for help.

## 13. Fixed NPC Foundation

### Launch Scope

Initial NPCs are fixed authored characters with stable locations, roles, services, dialogue, and inventories or restock rules. They do not age, marry, have children, die, change careers, form simulated households, or migrate autonomously.

This boundary keeps development focused on player society and the core MMORPG systems. NPC state may still react in small authored ways to quests, settlement upgrades, discoveries, and world flags, but there is no general family or life simulation.

### Future Reconsideration

Opportunity-driven migration, household simulation, memory, and departure are deferred rather than promised systems. They should be reconsidered only after fixed NPCs become a demonstrated limitation. Existing NPC identifiers and persistence should remain stable enough that later behaviour can be added without replacing characters players already know.

### Guiding Rule

NPCs keep essential services and stories available, while the most interesting professions, schools, markets, expeditions, and settlement decisions should belong to human players.

## 14. Social Spaces and the Tavern

### Daily Convergence

The tavern is a natural meeting point as working activities wind down. Farmers, adventurers, craftspeople, travellers, and NPCs can converge there without every player being forced into the same profession.

### Soft Mechanical Value

Taverns can provide rumours, notice boards, contracts, meals, recruitment, local news, travellers, music, games, and information.

The main reward should be information, opportunity, and social contact rather than a mandatory numerical bonus that encourages AFK behaviour.

## 15. Knowledge, Reputation, and Experience

### Knowledge as Progression

Experienced characters should know more, not merely hit harder.

Players may discover agricultural practices, monster behaviours, recipes, routes, material properties, crafting techniques, local history, or environmental clues.

### Teaching and Records

Knowledge can be taught, written into books, kept in journals, traded, stored in guilds or schools, and collected in libraries. Records may contain anything from a vague clue to a complete method, depending on the author's knowledge and Teaching ability.

Skills and recipes are not automatically public merely because one player discovers them. The discoverer chooses whether to explain, demonstrate, sell, record, or conceal what they know, subject to the possibility that others discover it independently. The goal is for expertise to feel like lived experience and for knowledge communities to become part of server history.

### Reputation

Professional reputation, guild standing, settlement history, successful contracts, and social trust can become important forms of progression.

## 16. Population and Server Philosophy

### Community Scale

The population goal is several hundred concurrent players in one zone instance. The design does not require thousands of characters standing in the same city, and smaller persistent communities may better support recognition and repeated encounters.

The game should be designed to remain functional at low concurrency and become richer as more players gather.

The world is divided into persistent zones that may run multiple instances when population or performance requires it. Exact topology, instance selection, overflow rules, and cross-instance social behaviour should be designed during scale testing. Instancing must not duplicate unique property, markets, or settlement authority in ways that fracture the persistent world.

### PvP and Criminal Play

The initial game has no player-versus-player combat, player theft, or player actions that forcibly destroy another player's property. PvP may be added later only alongside a design that lets players pursue criminal roles without ruining the experience of players who did not choose to participate.

Future criminal play should therefore begin from consent, bounded opt-in spaces or activities, readable risk, enforceable consequences, and reliable recovery. No present system should assume that open-world PvP will eventually exist.

### No Dependency Dead Ends

A settlement should not become unusable because the only player blacksmith stopped logging in. NPCs, travelling services, imports, substitutes, repair options, or pioneer choices provide fallback paths.

Players should be valuable without becoming single points of failure.

## 17. Technical Foundation

### Language

Development will be done in Rust.

### Architecture

The first playable build must already have a separate multiplayer server and client. Multiple clients must be able to connect to the same persistent world from the beginning.

The server should be authoritative for world state, time, characters, inventory, trades, skills, crops, NPC state, and persistence.

### Database

MySQL is the selected durable database for shared preview and production worlds. The server repository must use explicit schema migrations, transactions for multi-record mutations, connection pooling, indexed authoritative identifiers, and backup-and-restore procedures appropriate to MySQL.

Local preview connection settings live in the ignored `.env.preview` file using this contract:

```dotenv
DB_DRIVER=mysql
DB_HOST=localhost
DB_PORT=3306
DB_DATABASE=tarrowyn
DB_USERNAME=
DB_PASSWORD=
```

The blank username and password in documentation are placeholders, not defaults. Credentials must never be committed, bundled with the client, printed in player-visible errors, or copied into browser artifacts. Production supplies the same settings through its secret-management environment rather than a committed file.

The existing versioned JSON repository remains a deterministic development and
restore companion. The MySQL bridge is implemented for shared preview and the
selected production path, but public release still requires the target
environment to pass migration, concurrent-write, backup, restore, failover,
and rollback validation. Selecting MySQL does not by itself make the current
deployment production-ready.

The client treats a restore-invalidated event cursor as a recoverable boundary:
the shared toolkit preserves the server error code, and the client discards
stale cursor-derived projections before reloading authoritative state and
history from cursor zero. A restore must never turn cached history into a new
reward. The regional event view follows the same cursor contract, merging
stage updates by stable event ID and restarting from cursor zero when a restore
invalidates its cached regional cursor.

### Regional Chronicle

The regional chronicle is server-owned public history. The normal settlement
view keeps a bounded recent window and a compact archive summary so the client
remains readable after years of events. Entries that leave the recent window
are retained in a durable append-only archive and remain searchable through an
authenticated history endpoint; old achievements are not removed merely to
keep the player-facing feed small. Account deletion anonymises matching names
in recent entries, archived entries, and retained event records while leaving
the public event's historical shape intact.

### Selected Technology and Deployment Boundaries

The release candidate selects Rust for both the authoritative server and the
2D client, Macroquad with the shared `macroquad-toolkit` for client runtime,
rendering, grid, data-loading, and frame-polled HTTP support, and a versioned
JSON protocol for server-owned requests and responses. The browser target is
WebGL and the development desktop target is native Windows; both use the same
authoritative server contract. MySQL is the durable shared-world backend,
while the versioned JSON repository remains available for deterministic local
fixtures and restore-on-a-copy drills.

Production identity is an OIDC gateway boundary. The checked-in link fixture
proves the character-preserving session and revocation contract, but the real
provider, TLS termination, secret rotation, hosting topology, database
failover, rollback, and any multi-worker regional decomposition remain
deployment-owned gates rather than client assumptions. The current server
enforces one MySQL world authority at a time until that later topology is
designed and tested.

## 18. Smallest Multiplayer Slice

### Purpose

The first slice exists to answer one question: does this world feel pleasant and meaningful to inhabit with other people?

### Required Scope

- Rust authoritative server and Rust 2D client.
- Persistent account and character identity.
- At least 10-20 simultaneous clients as an initial engineering target.
- One tiny settlement and nearby wilderness.
- Movement, collision, chat, and basic social presence.
- Accelerated day/night cycle.
- One tavern.
- One shared farming area or small set of farm plots.
- Three crops with planting, tending, growth, and harvest.
- Inventory and persistence.
- Simple direct player trading.
- One forest or wilderness zone.
- One monster type.
- One basic weapon and at least one inferior/improvised substitute.
- One repeatable adventurer contract.
- Basic persistent skill progress.
- Server restart without losing the world.

### Prototype Test

Place at least three real players into the slice: one primarily farming, one primarily adventuring, and one moving between activities.

The prototype succeeds if they naturally exchange goods or information, recognise reasons to depend on one another, and choose to regroup socially without being mechanically forced.

A particularly strong signal is players organically saying some version of 'meet you at the tavern tonight'.

## 19. Major Risks and Design Countermeasures

### Community Dependency

Risk: the game is only enjoyable after a critical mass of players forms.

Countermeasure: fixed NPC fallback services, travelling services, imports, and substitutes keep low-population settlements functional while leaving room for players to outperform them.

### Slow Progress Feels Like No Progress

Risk: players enjoy the concept but feel an evening achieved nothing.

Countermeasure: every session needs small outcomes, while medium and long ambitions provide direction. Progress includes knowledge, relationships, trade, reputation, property, discoveries, and world change.

### Profession Silos

Risk: farming, crafting, and adventuring become separate games sharing a chat server.

Countermeasure: systemic events and resource chains should affect multiple professions and create cross-role demand.

### Economic Saturation

Risk: persistent production eventually makes all common goods worthless.

Countermeasure: optional consumables, maintenance, construction, crafting inputs, settlement upgrades, expeditions, NPC demand, and meaningful material sinks. Mandatory hunger, thirst, or sleep are not used as artificial demand.

### NPC Replacement of Players

Risk: players learn that missing professions will always be solved by spawned NPCs.

Countermeasure: fixed NPCs offer limited baseline services, inventories, and quality. They do not dynamically expand to erase unmet demand, leaving clear room for player specialists.

### Land Hoarding

Risk: early players permanently monopolise desirable territory.

Countermeasure: three-real-month renewable leases, expiry and reclamation rules, expandable frontier space, and reclaimable abandoned land.

### Real-Time Lockout

Risk: players with fixed schedules repeatedly miss content.

Countermeasure: a non-even day length, soft rather than hard opening hours, alternative service access, and varied time-sensitive opportunities.

### Settlement Collapse

Risk: population loss makes a town feel punished rather than historically dynamic.

Countermeasure: decline is gradual, signposted, reversible, and creates vacancies, cheaper property, rebuilding goals, travelling services, and opportunities for new settlers.

### Skill Discovery Becomes a Solved Build Guide

Risk: external guides and data mining turn hidden mergers into a mandatory optimal path, undermining experimentation and player schools.

Countermeasure: make many skills situational rather than strictly superior, support several routes to comparable capabilities, keep server-only conditions out of the client data, add new mergers over time, and let discoveries gain value from teachers, local history, reputation, and social context rather than secrecy alone. The game should remain enjoyable after a recipe becomes widely known.

### Teaching Becomes Power Levelling

Risk: direct instruction lets established schools instantly manufacture advanced characters or exclude everyone outside their network.

Countermeasure: teaching substitutes only for explicitly allowed discovery steps. Learners still contribute time, practice, participation, and any non-teachable requirements; independent discovery always remains possible.

## 20. Locked Scope and Deliberate Deferrals

### Locked Direction

- An in-game day lasts 80 real minutes; season and year scale waits for crop-cadence testing.
- Characters are immortal. Defeat means knockout and carried-item risk, never death or generational succession.
- Combat is real-time and every attack, technique, defensive action, item, and spell is an explicit skill use.
- Launch has no PvP, player theft, or criminal actions against non-consenting players.
- Capacity targets several hundred concurrent players per zone instance.
- Root skills have direct entry paths, while advanced mergers and exact progress remain discoverable.
- Teaching depth must meet or exceed the depth of a mastered subject before direct instruction is possible.
- Crafting uses a short, accessible timing interaction to influence quality within a skill-and-material-defined range.
- Launch governance has only a bounded mayor, public settlement taxes, a public treasury, and predefined upgrades.
- Land leases last three real months and expire when not renewed, subject to warning and reclamation rules.
- Launch NPCs are fixed authored characters without family or lifecycle simulation.
- Economic detail follows meaningful fantasy production and exchange; immortal characters do not need food, drink, or sleep.

### Deliberately Deferred

- Exact season and year length, pending playtests of farming access and pacing.
- PvP or criminal roles, unless a later opt-in and consequence model protects non-participants.
- Detailed zone topology, overflow, and cross-instance rules until scale testing.
- Government beyond the mayoral settlement loop until players demonstrate a need for it.
- NPC aging, birth, death, marriage, households, migration, and career changes.
- Player-driven dynamic threat generation until ordinary combat, economy, profession, and settlement systems are mature.

## 21. Definition of Done for the Concept Prototype

### The Concept Is Proven When

- Multiple clients can connect to a persistent Rust server and interact in the same world.
- The world clock, farming, a small economy, combat risk, and social space function together rather than as disconnected demos.
- Three or more players can play different roles and find spontaneous reasons to trade, cooperate, share information, or travel together.
- A normal session feels worthwhile without a level-up or major unlock.
- Loss has enough consequence to make preparation matter without making defeat so punishing that players avoid leaving town.
- The server can restart without erasing player or world progress.
- Players can describe at least one event as something that happened to their community, not merely something scripted that happened to their character.
