# The Years of Tarrowyn

## Game Design Document

**Version:** 0.1 - Concept Foundation  
**Status:** Working design  
**Platform/technology baseline:** 2D multiplayer client + authoritative server, developed in Rust

> A cozy, slow-burn fantasy MMORPG about living in a persistent society where communities, professions, settlements, and history matter more than racing to a level cap.

## Decision Status

| Area | Status | Current Decision |
|---|---|---|
| Working title | Locked for project | The Years of Tarrowyn |
| Genre | Locked | Cozy slow-burn 2D MMORPG / persistent fantasy society |
| Technology | Locked | Rust; separate authoritative server and client from the first slice |
| Progression | Direction locked | Skill/credential based, horizontal, no conventional global level |
| World time | Direction locked | Roughly 3 real hours per in-game day; exact non-even duration TBD |
| Normal defeat | Direction locked | Knockout with item/goods risk; no routine permanent character death |
| Settlements | Direction locked | Organic growth and decline, potentially homestead to kingdom |
| NPCs | Direction locked | Opportunity-driven migration, households, departure, memory |
| Land | Direction locked | Aging claims/leases rather than permanent first-come ownership |
| Frontier | Direction locked | Player-led pioneer expeditions and new settlements |
| Combat details | Open | Not yet selected |
| Generational succession | Open | Possible rare/opt-in legacy system, not normal defeat |

## 1. Executive Summary

### High Concept

The Years of Tarrowyn is a cozy, slow-burn 2D MMORPG about living in a persistent medieval fantasy society rather than racing through a conventional MMO progression ladder.

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

The target is roughly three real hours per in-game day. The exact duration is not locked.

The final duration should preferably not divide evenly into 24 real-world hours. This prevents a player who logs in at the same real time every day from always seeing the same phase of the in-game day.

### Time-of-Day Design

Different times should provide different flavour and opportunities. Shops, NPC routines, wildlife, rumours, lighting, travel risk, and social activity may change.

Time should rarely hard-lock essential systems. If a blacksmith is unavailable overnight, an apprentice, order system, travelling service, or next-day collection can preserve access.

### Long-Term Calendar

Seasons and years are intended to matter eventually, particularly for farming, land leases, migration, settlement history, and long-term personal goals. Their exact real-time duration is still open.

## 5. Progression

### No Conventional Character Level

The game should avoid a single global level as the main measure of worth. Skills and credentials provide progression instead.

### Skill Progression

Skills should unlock capabilities, techniques, specialisations, efficiency, quality, and breadth rather than only increasing numerical power.

A numerical 1-100 skill treadmill should not simply recreate character levels under a different name. Skill mastery should increasingly depend on practice, teachers, discoveries, tools, demonstrations, and meaningful milestones.

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

### Adventuring

Adventuring is a profession within society rather than the default path every player must follow.

Adventurers take contracts, investigate threats, explore, escort, recover resources, gather information, and protect routes or settlements.

### Cross-System Example

A wyvern settles near a farming region. Wildlife is displaced into fields, crop losses increase, food prices rise, hunters find more work, the tavern posts contracts, blacksmiths receive demand for suitable equipment, and adventurers eventually investigate the source. The event becomes economic and social content as well as combat content.

## 7. Economy and Item Value

### Item Importance

Equipment and goods should carry enough value that acquisition, maintenance, trade, damage, theft, and replacement create meaningful stories.

A farmer losing a good hoe should be inconvenienced and may need to borrow, improvise, repair, buy, or commission another. An adventurer without a sword may temporarily rely on a dagger, spear, club, borrowed weapon, or lower-quality substitute.

### Economic Sinks

Persistent economies need consumption. Food, potions, seeds, building materials, workshop inputs, maintenance materials, expedition supplies, festivals, and NPC settlement demand can remove goods from circulation.

Durability should not become constant busywork. Maintenance and occasional damage are preferred over rapid equipment decay.

### NPC Baseline vs Player Excellence

NPC services should keep settlements functional but generally be more limited, expensive, slower, less specialised, or less customisable than strong player-run services.

NPCs fill cracks in the economy rather than replacing the incentive to become a profession.

## 8. Defeat, Risk, and Recovery

### Normal Defeat

Ordinary combat defeat should normally result in being knocked unconscious rather than permanent character death.

Unconsciousness must not require the player to sit unable to act for a long period. Recovery should return control promptly while preserving consequences.

### Consequences

- Carried goods may be lost, stolen, scattered, or damaged. Carried equipment may also be damaged or taken depending on circumstances.
- Owned property, stored goods, land rights, and the majority of long-term assets should generally remain safe unless a separate system explicitly places them at risk.
- Other possible consequences include injury, recovery costs, being transported elsewhere, or owing a rescuer.

### Social Recovery

Defeat can create player interaction. Another player, caravan, guard, or NPC may recover an unconscious character and return them to safety.

Risk should therefore generate stories rather than merely impose a timer.

### Generational Characters

Generational succession has been considered but is not selected as the normal defeat system. It remains a possible future system for rare death, voluntary legacy play, aging, or long-term world history.

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

Login inactivity windows, election systems, taxation, councils, and authority boundaries remain open design questions.

## 11. Land, Property, and Leases

### Recognised Claims

Mechanically important land should not be owned forever on a first-come-first-served basis.

Land ownership is better represented as a recognised claim or lease recorded by a settlement, realm, guild, or other authority.

### Aging Ownership

Claims should eventually require renewal and may lapse after extended inactivity or non-payment. Exact lease duration is not locked.

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

## 13. Dynamic NPC Ecology

### NPCs Follow Opportunity

NPCs should arrive because they perceive sustained economic and social opportunity, not because the settlement crossed a simple population threshold or lacks a specific class.

Example: players establish a homestead with demand for metal. After sustained unmet demand, a miner may choose to move nearby. Their spouse may be a tailor, creating a second service as a natural consequence rather than a designed unlock.

### Demand Signals

NPC migration can consider unmet purchase orders, local prices, imports, local resources, active population, housing, safety, food, profitability, family suitability, and existing competition.

Demand should be sustained over time so one unusual purchase does not summon a new industry.

### Households

NPCs can move as households rather than isolated vendors. Family members may have different professions, needs, relationships, and future paths.

This can gradually produce settlement demographics and history without every service being spawned independently.

### Departure

If demand disappears or living conditions worsen, NPCs may eventually consider leaving. Departure timing should include randomness over an extended period rather than a perfectly predictable countdown.

Players should receive clues through dialogue, behaviour, reduced investment, notices, or local rumours so migration feels causal rather than arbitrary.

### Player Influence

Players can influence but not fully command NPC decisions. A town might subsidise a forge, offer a contract, improve housing, make the roads safer, or guarantee purchases.

An NPC may still decide to leave.

### NPC Memory

NPCs should remember meaningful former homes, relationships, employers, occupations, and moves. A former resident encountered years later can recognise players or ask about their previous settlement.

### Guiding Rule

NPCs keep society functioning, but whenever possible they should create opportunities and vacancies for human players rather than replace them.

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

Knowledge can potentially be taught, written into books, kept in journals, traded, stored in guilds, or collected in libraries.

The exact secrecy and discoverability model is open. The goal is for expertise to feel like lived experience.

### Reputation

Professional reputation, guild standing, settlement history, successful contracts, and social trust can become important forms of progression.

## 16. Population and Server Philosophy

### Community Scale

The design does not require thousands of characters standing in the same city. Smaller persistent communities may better support recognition and repeated encounters.

The game should be designed to remain functional at low concurrency and become richer as more players gather.

### No Dependency Dead Ends

A settlement should not become unusable because the only player blacksmith stopped logging in. NPCs, travelling services, imports, substitutes, repair options, or pioneer choices provide fallback paths.

Players should be valuable without becoming single points of failure.

## 17. Technical Foundation

### Language

Development will be done in Rust.

### Architecture

The first playable build must already have a separate multiplayer server and client. Multiple clients must be able to connect to the same persistent world from the beginning.

The server should be authoritative for world state, time, characters, inventory, trades, skills, crops, NPC state, and persistence.

### Deliberately Unselected Technology

Networking library, rendering framework, database, protocol, deployment target, authentication provider, and hosting architecture are not yet locked. These should be chosen to suit the smallest slice rather than assumed by the design.

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

Countermeasure: NPC fallback services, households, travelling services, imports, and opportunity-driven migration keep low-population settlements functional while leaving room for players to outperform them.

### Slow Progress Feels Like No Progress

Risk: players enjoy the concept but feel an evening achieved nothing.

Countermeasure: every session needs small outcomes, while medium and long ambitions provide direction. Progress includes knowledge, relationships, trade, reputation, property, discoveries, and world change.

### Profession Silos

Risk: farming, crafting, and adventuring become separate games sharing a chat server.

Countermeasure: systemic events and resource chains should affect multiple professions and create cross-role demand.

### Economic Saturation

Risk: persistent production eventually makes all common goods worthless.

Countermeasure: ongoing consumption, maintenance, construction, food use, events, expeditions, NPC demand, and meaningful material sinks.

### NPC Replacement of Players

Risk: players learn that missing professions will always be solved by spawned NPCs.

Countermeasure: NPCs respond imperfectly to opportunity, have limitations, make independent decisions, and often create partial solutions rather than supplying the exact missing profession.

### Land Hoarding

Risk: early players permanently monopolise desirable territory.

Countermeasure: leases, aging claims, inactivity rules, expandable frontier space, and reclaimable abandoned land.

### Real-Time Lockout

Risk: players with fixed schedules repeatedly miss content.

Countermeasure: a non-even day length, soft rather than hard opening hours, alternative service access, and varied time-sensitive opportunities.

### Settlement Collapse

Risk: population loss makes a town feel punished rather than historically dynamic.

Countermeasure: decline is gradual, signposted, reversible, and creates vacancies, cheaper property, rebuilding goals, travelling services, and opportunities for new settlers.

## 20. Open Design Questions

### Still Unresolved

- Exact real-time length of an in-game day, season, and year.
- Whether true character death exists at all, and whether generational succession becomes an opt-in or rare legacy system.
- Combat model and degree of action versus tactical play.
- PvP, criminal behaviour, theft between players, and law enforcement.
- World topology, shard structure, instancing, and maximum intended server population.
- Exact skill trees, profession specialisation, teaching, and knowledge transfer rules.
- Crafting interaction and how much player execution affects quality.
- Government structure, elections, taxes, offices, and local authority.
- Land lease length, renewal, inheritance, abandonment, and reclamation.
- Depth of NPC family simulation, aging, birth, death, marriage, and career changes.
- How dynamic threats and world events are generated and resolved.
- How much of the economy is fully simulated versus abstracted.

## 21. Definition of Done for the Concept Prototype

### The Concept Is Proven When

- Multiple clients can connect to a persistent Rust server and interact in the same world.
- The world clock, farming, a small economy, combat risk, and social space function together rather than as disconnected demos.
- Three or more players can play different roles and find spontaneous reasons to trade, cooperate, share information, or travel together.
- A normal session feels worthwhile without a level-up or major unlock.
- Loss has enough consequence to make preparation matter without making defeat so punishing that players avoid leaving town.
- The server can restart without erasing player or world progress.
- Players can describe at least one event as something that happened to their community, not merely something scripted that happened to their character.
