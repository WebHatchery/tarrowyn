# Foundational playability audit

Status: F0 baseline established on 2026-09-02.

This audit treats the attached **Tarrowyn Foundational Game Design Brief** as
the design authority and uses the GDD plus the historical Phase 0–6 records as
implementation evidence. `Usable` has a strict meaning here: a connected
player can complete the experience through the current client. A protocol
type, endpoint, content row, repository rule, or offline-only presentation is
not enough by itself.

## Readiness assessment

The release candidate is technically deep but not yet a cohesive foundational
settlement experience. Its connected client already proves shared authority,
movement, farming, direct trade, persistence, touch controls, exploration and
several advanced systems. It does not yet present the new player with an
undeveloped tent camp, a visible First Beacon, a builder-led local need, basic
logging/mining/smithing, physical storage or construction, or contextual
world-first interactions. Some existing mature-town, registry-lease and
permanent-command assumptions actively conflict with the brief and must be
reconciled without removing the advanced systems.

F0 adds no onboarding or gameplay. It establishes
`first-beacon-baseline-v1` in `assets/data/region.json`: 12 stable, visible
landmark records and 12 server-authoritative interaction records. The existing
`macroquad_toolkit::data_loader` content path validates it; `/v1/state`
projects it; and the client retains it for F1 presentation. The fixture is a
contract for later phases, not evidence that its currently non-rendered
interactions are usable.

Status totals below are generated from the 132 requirements in this matrix:

| Status | Count |
|---|---:|
| usable | 8 |
| partial | 51 |
| missing | 52 |
| conflicting | 8 |
| deliberately deferred | 13 |

## Evidence keys

- **Baseline:** `assets/data/region.json`, `protocol/src/foundation.rs`,
  `server/src/content.rs`, `server/src/content/region_validation.rs`,
  `server/src/repository/observability.rs`, `src/network/projection.rs`.
- **Authority:** `server/src/repository.rs`, `server/src/repository/persistence.rs`,
  `server/src/http.rs`, `protocol/src/lib.rs`.
- **Connected client:** `src/network.rs`, `src/network/projection.rs`,
  `src/ui_online.rs`, `src/ui_online/controls.rs`, `src/ui_regional.rs`.
- **Civic/region:** `server/src/repository/phase4.rs`,
  `server/src/repository/phase5.rs`, `assets/data/infrastructure.json`,
  `assets/data/settlements.json`.
- **Evidence:** the Phase 0–6 documents and runbooks linked from
  `docs/README.md`, especially `PHASE_4_PLAYTHROUGH.md`,
  `PHASE_5_PLAYTHROUGH.md`, and `PHASE_6_FOLLOW_UP_REGISTER.md`.

## Requirements matrix

The rows are atomic where the brief gives an independent acceptance rule.
Closely coupled examples are grouped only when they share one implementation
boundary, status, owner and risk.

### Core fantasy and foundational principles

| ID | Brief requirement | Status | Existing implementation evidence | Current player-facing evidence | Remaining owner | Dependency or risk |
|---|---|---|---|---|---|---|
| FP-01 | One persistent shared world. | usable | Authority; JSON/MySQL repository boundary; `/v1/state`. | Connected clients share server-owned state and cursors. | F1 regression | Deployment origin remains an external gate. |
| FP-02 | One permanent starting beacon shared by all first arrivals. | partial | Baseline `first-beacon`, permanent at `(8,6)`; guest spawn uses Hearth position. | Players spawn together, but no beacon is rendered or named. | F1 | Do not mistake a projected record for an arrival experience. |
| FP-03 | Early arrivals may take desirable nearby land; later players seek farther opportunity. | partial | Claims, three starting registry plots, regional vacancies and frontier travel. | Claim and regional opportunity summaries exist. | F8/F9 | Registry abstractions do not yet express physical nearby scarcity. |
| FP-04 | No permanent class or profession selection. | usable | Skill catalogue and profession capabilities have no character class field. | No class-selection screen; visible Practice and profession actions exist. | F7 regression | Later onboarding must not add a disguised class choice. |
| FP-05 | A newcomer can try farming, logging, mining, exploration and smithing before specialising. | partial | Farming and exploration work; skill roots exist; baseline identifies woodland, mine and forge. | Farming/exploration are playable; logging, mining and forge work are not. | F2/F4/F7 | Practice-menu entries are not world activity proofs. |
| FP-06 | A sufficiently committed player may eventually participate in every activity. | partial | Root skills have direct practice paths and no exclusive class gates. | Player can practise roots, but several foundational activities have no loop. | F7 | Preserve broad access while adding efficiency differences. |
| FP-07 | Interdependence comes from limited time and inconvenient self-supply, not arbitrary lockouts. | partial | Trade, orders, regional scarcity and material escrow create demand. | Trade/order controls exist; no fixed time-saving foundational comparison. | F5/F7 | Current credentials can feel like gating unless crude fallbacks are clear. |
| FP-08 | Specialists gain value from knowledge, tools, facilities, efficiency, quality and supply relationships. | partial | Skills, knowledge, tool condition, service quality, market and orders exist. | Several values are visible, but foundational activities do not connect them. | F4/F5/F7 | Advanced surfaces currently precede a legible basic chain. |
| FP-09 | Foundational activities are simple to begin; optional depth improves rather than invalidates the base action. | partial | Plant/tend/harvest and crude fallback concepts exist; weather/pests add depth. | Simple farming remains playable; other activities lack base actions. | F2–F4 | Avoid exposing advanced complexity before its purpose is felt. |
| FP-10 | Add deeper layers only after players experience the problem they solve. | conflicting | Phase 4–6 systems implement governance, schools, regional markets and operations. | Dense permanent controls expose advanced systems before the First Beacon loop. | F1/F7 | Re-sequence presentation without deleting advanced systems. |
| FP-11 | Progress is visible through tools, fields, shelter, storage, knowledge, trade, reputation, contributions, beacons and history rather than a global level. | partial | Tool condition, crops, skills, trades, reputation, claims, outpost and chronicle persist. | Many ledger values are visible; shelter/storage/building/beacons are not world-visible. | F2–F9 | Existing generic skill number may read like a level. |
| FP-12 | An ordinary session can feel worthwhile without a level or major unlock. | partial | Farming, trade, contract and travel produce small outcomes. | Automated role loop exists; human evidence is explicitly still desired. | F3/F7 | Product enjoyment cannot be inferred from deterministic endpoints. |

### Initial session experience

| ID | Brief requirement | Status | Existing implementation evidence | Current player-facing evidence | Remaining owner | Dependency or risk |
|---|---|---|---|---|---|---|
| IH-01 | Arrive at the First Beacon in a settlement primarily made of tents. | conflicting | Baseline has beacon/tent records; current Hearth content has population 36, town hall, services and mature infrastructure. | Spawn is at Hearth; neither beacon nor tent camp is shown. | F1 | Preserve mature systems while presenting an authored baseline state. |
| IH-02 | Meet the builder as the first foundational NPC. | partial | Baseline has stable `builder-mara`; fixed NPC household infrastructure exists. | Mara is not rendered, approachable or conversational. | F1 | Fixed-NPC identity must remain stable through persistence. |
| IH-03 | Learn what the settlement currently needs. | partial | Notice feed, settlement demand, vacancies and `read-local-needs` record. | Tavern/region text shows notices, but no visible world noticeboard at arrival. | F1 | Competing sources may obscure the one next need. |
| IH-04 | Walk through the surrounding world rather than operate through a command menu. | conflicting | Server movement, collision and map exist. | Movement is world-based, but foundational actions live in a large permanent command surface. | F1/F2 | Context selection must remain touch-first. |
| IH-05 | Try basic farming. | usable | `/v1/farming/actions`, persistent plots, deterministic tests. | Visible Plant, Tend, Harvest and Care controls. | F3 polish | Current fields are shared and server-authoritative. |
| IH-06 | Try logging and mining. | missing | Forestry/mining skills and baseline resource landmarks only. | No connected logging or mining action or yield. | F2 | Needs authoritative nodes, tools, depletion/recovery and inventory goods. |
| IH-07 | Try exploration and observe/use the rough forge. | partial | Movement, regional travel/map and baseline forge record. | Exploration/travel work; forge is neither rendered nor usable. | F2/F4 | Travel UI must not substitute for local discovery. |
| IH-08 | See cross-activity support and leave with a modest future-session goal. | missing | Server systems contain material/order connections. | No first-hour chain or journal goal makes the connection legible. | F7 | Requires earlier activity proofs and a visible non-class goal surface. |
| SS-01 | In about 15 minutes, inspect offline-grown crops, harvest, tend/water and replant. | partial | World-owned crop growth and full farming actions persist. | Controls exist, but no recorded 15-minute human acceptance scenario. | F3 | Offline elapsed-time semantics and crop cadence need explicit proof. |
| SS-02 | In a short visit, check tools, fencing or storage. | partial | Field-tool condition is visible; claims/infrastructure exist. | Tool condition can be read; physical fence and storage checks are absent. | F3/F8 | Keep maintenance useful rather than mandatory busywork. |
| LS-01 | In 60–90 minutes, discover/barter seeds, gather fencing timber, commission a tool, reorganise land, trade harvest and contribute surplus. | partial | Crops, direct trade, service orders, claims and projects exist separately. | Several controls work, but logging, fences, physical construction and the complete chain do not. | F7/F8 | Cross-system sequence is unproven by a human session. |
| LS-02 | Short sessions maintain a life; longer sessions create new possibilities. | missing | No acceptance fixture compares these rhythms. | No connected journal/life presentation demonstrates the distinction. | F3/F7 | Requires pacing evidence, not additional breadth. |

### Foundational activities and economic connections

| ID | Brief requirement | Status | Existing implementation evidence | Current player-facing evidence | Remaining owner | Dependency or risk |
|---|---|---|---|---|---|---|
| FA-01 | Farming is a small connected foundational activity. | usable | Three crops, shared plots, server clock, inventory and trade. | Full visible plant/tend/harvest loop. | F3/F7 | Needs short-session proof and clearer world context. |
| FA-02 | Logging produces timber, firewood, charcoal material, fence parts and handles. | missing | Timber exists in regional stock and orders; Forestry skill exists. | No logging action or foundational timber outputs. | F2/F4 | Item model currently lacks most named timber derivatives. |
| FA-03 | Mining produces stone, clay, coal and metal ore. | missing | Stone and iron salvage exist regionally; Mining skill exists. | No mine action or foundational mineral outputs. | F2/F4 | Define small initial set without building a full simulation. |
| FA-04 | Exploration discovers deposits, seeds, routes and settlement sites. | partial | Movement/travel, routes, regional sites and event projections exist. | Routes and locations are visible; discoveries are prelisted rather than found. | F2/F9 | Avoid turning exploration into a menu reveal. |
| FA-05 | Basic smithing combines ore, fuel and components into useful tools. | missing | Smithing root and generic service-order infrastructure exist. | No rough-forge recipe or smithing production chain. | F4 | Depends on F2 gatherable ore/fuel/components. |
| FA-06 | Barter/direct trade is basic and atomic. | usable | `/v1/trades`, escrow validation, replay-safe acceptance. | Visible review/accept/cancel path. | F5 | Fixed proof must show cooperation saves time. |
| FA-07 | Construction works through the builder NPC. | missing | Baseline builder/site records; civic projects exist. | No player-builder interaction or material contribution loop. | F6 | Reuse authoritative project ledgers; do not add parallel persistence. |
| EC-01 | Farmers produce food, seeds and useful plant material. | partial | Crops and seeds exist and trade. | Food crops/seeds are visible; other plant materials are absent. | F3/F7 | Minimal additions should serve a real production chain. |
| EC-02 | Loggers support fuel, fencing and tool components. | missing | Baseline woodland and timber market concept. | No connected source or conversion. | F2/F4 | Logging must precede forge comparison. |
| EC-03 | Miners support construction and smithing inputs. | missing | Stone/iron regional stock concepts. | No connected source or direct use at First Beacon. | F2/F4/F6 | Resource nodes need readable availability. |
| EC-04 | Blacksmiths make useful tools from ore, fuel and other components. | missing | Smithing skill only. | No blacksmith loop. | F4 | Must quantify improved-tool benefit. |
| EC-05 | Explorers reveal resources, routes and settlement opportunities. | partial | Regional map, events, locations and pioneer outpost exist. | Players can inspect known routes/sites; revealing them is absent. | F2/F9 | Stable discovery state must remain server-owned. |
| EC-06 | Builders consume money/materials and create visible structures. | partial | Governance/public projects consume treasury; baseline builder/site records. | Public actions are ledgers, not builder-led structures appearing in the world. | F6 | Project completion must transform authoritative map state. |
| EC-07 | Crude tools always provide fallback; improved tools measurably save time/actions/materials. | partial | New players receive field tools and weapons; baseline tool rack; service repair. | Some default equipment exists, but no crude-access choice or fixed comparison. | F2/F4 | Avoid specialist dependency dead ends. |

### First settlement and builder

| ID | Brief requirement | Status | Existing implementation evidence | Current player-facing evidence | Remaining owner | Dependency or risk |
|---|---|---|---|---|---|---|
| FS-01 | Permanent First Beacon. | partial | Stable validated baseline record; arrival shares `(8,6)`. | Not rendered or interactable. | F1 | Must never degrade in later beacon lifecycle work. |
| FS-02 | Tent settlement. | partial | Stable baseline record. | Current map does not show a tent camp. | F1/F2 | Mature Hearth projections conflict with visual starting state. |
| FS-03 | Communal fire/gathering place. | partial | Stable `first-beacon-fire` record; tavern chat/social feed exists. | Social text exists, but no world fire or gather interaction. | F1 | Keep social value soft rather than mandatory. |
| FS-04 | Builder NPC. | partial | Stable `builder-mara` record. | No visible NPC. | F1 | Builder dialogue/service must be fixed and durable. |
| FS-05 | Noticeboard or visible local-needs source. | partial | Stable board record plus notice/demand endpoints. | Notices are readable in sidebar/feed, not at a world board. | F1 | F1 should select a clear initial need. |
| FS-06 | Basic shared storage or collection point. | partial | Stable cache record; regional stock and escrow systems exist. | No player-facing shared container. | F2 | Authority, capacity, access and replay rules are needed. |
| FS-07 | Crude-tool access. | partial | Stable rack; default field tool/weapon. | No visible rack, borrow or choose action. | F2 | Tool identity is currently too coarse for all activities. |
| FS-08 | Nearby farmland. | usable | Manifest farm plots, map field tiles and farming endpoints. | Fields and crop interactions are visible. | F3 polish | Ensure First Beacon presentation points players there. |
| FS-09 | Nearby woodland. | partial | Stable woodland record and forest tiles. | Woodland is visible on the map; no logging interaction. | F2 | Node state and output persistence are missing. |
| FS-10 | Nearby mineable ground. | partial | Stable mine record and stone tile. | Ground is map-visible only; it cannot be mined. | F2 | Avoid conflating regional stock with a local node. |
| FS-11 | Rough forge. | partial | Stable forge record and Smithing root. | No forge art/context/control or recipe. | F4 | Depends on gathered inputs and tool comparison. |
| FS-12 | Visible space for future construction. | partial | Stable `storehouse-site` record. | Site is not rendered. | F6 | F1 may render it without enabling contribution early. |
| FS-13 | Early projects include storehouse, well, fenced growing area, smithing shelter, carpenter yard and first homes. | partial | Civic project types and infrastructure framework exist. | Named foundational projects are not visible construction choices. | F6/F8 | Keep initial scope to the storehouse before expanding. |
| FS-14 | The first communal project visibly shows what contributed materials will create. | missing | Baseline storehouse site and project ledgers are separate. | No unfinished structure or contribution forecast. | F6 | Needs staged authoritative state and replay-safe contribution. |
| BP-01 | Builder introduces construction and settlement development. | missing | Stable builder record only. | No dialogue or service. | F1/F6 | F1 introduction must not prematurely implement F6 contribution. |
| BP-02 | Builder accepts appropriate money/resources. | missing | Generic escrow/project cost systems exist. | No builder request. | F6 | Reuse existing transaction/idempotency infrastructure. |
| BP-03 | Builder builds or assists communal structures. | missing | Civic public actions exist. | No construction transformation. | F6 | Must produce visible persistent world change. |
| BP-04 | Builder may be hired for personal construction. | missing | Service orders exist but not builder housing. | No personal construction path. | F8 | Depends on tent/property placement and staged housing. |
| BP-05 | Builder is a dependable low-population fallback. | missing | Fixed-NPC fallback philosophy and other travelling services exist. | No builder service. | F6/F8 | Baseline must remain useful without eclipsing players. |
| BP-06 | Builder gives logging, mining, smithing and trade immediate purposes without replacing player builders. | missing | Systems exist separately. | No connected builder demand chain. | F6/F7 | Balance requires the earlier gather/forge proofs. |

### Housing and beacon network

| ID | Brief requirement | Status | Existing implementation evidence | Current player-facing evidence | Remaining owner | Dependency or risk |
|---|---|---|---|---|---|---|
| HB-01 | Permanent home is a major long-term goal following tent → camp → house → additions. | missing | Claims and infrastructure records exist. | No personal tent/home construction stages. | F8 | Avoid presenting a registry row as a home. |
| HB-02 | House construction requires suitable location, money, materials, transport, builder/skilled help and time/stages. | missing | Individual subsystems exist for claims, currency, stock, travel and orders. | No integrated housing project. | F8 | Cross-system scope is large; keep stages deterministic. |
| HB-03 | Stored items remain physically where stored; beacon travel does not teleport remote storage. | missing | Character inventory and regional stock are separate. | No personal world storage or beacon travel exists. | F2/F9 | Storage location identifiers must remain stable. |
| BN-01 | First Beacon always exists and never degrades. | partial | Permanent baseline record. | Not shown; no degradation rule currently exercises it. | F1/F9 | F9 lifecycle must explicitly exclude this ID. |
| BN-02 | First Beacon always remains a valid arrival and travel location. | partial | Arrival position is stable; Hearth route access exists. | Arrival works, but no beacon travel surface. | F1/F9 | Preserve rescue spawn under all state transitions. |
| BN-03 | Players can construct additional high-cost frontier beacons; solo is possible, cooperation easier. | partial | Pioneer expedition/outpost systems exist. | Pioneer control can create an outpost, not a beacon project. | F9 | Reuse expedition logistics without equating outpost to beacon. |
| BN-04 | A player-built beacon can become a new settlement centre. | partial | Whisperwood outpost and multi-settlement region exist. | Regional settlement exists, but no player-built beacon caused it. | F9 | Must use the First Beacon systems rather than bespoke regional data. |
| BN-05 | New arrivals may choose among active beacons with name, age, facilities, activity, land, resources and needs. | missing | Settlement comparisons expose several of these fields. | Account bootstrap has no arrival choice. | F9 | Needs safe default to First Beacon and touch selection. |
| BN-06 | Active beacons provide travel between one another. | missing | Route travel is server-authoritative and bidirectional. | Travel uses roads/ferry, not beacons. | F9 | Beacon travel must coexist with regional logistics. |
| BN-07 | Beacon travel carries personal inventory but leaves external storage in place. | missing | Travel carries character inventory; regional stock stays local. | No beacon travel or external player storage. | F9 | Requires F2 storage semantics first. |
| BN-08 | Bulk freight/animal restrictions are not required for the foundation. | deliberately deferred | Road market logistics exist but do not define beacon freight. | No beacon travel restriction is presented. | F9 guardrail | Do not block F9 on a bulk-freight simulation. |
| BN-09 | A player beacon needs nearby non-abandoned tent/qualifying settlement support. | missing | Outpost supply requirements exist; no tent support relation. | No beacon support projection. | F9 | Depends on F2 tents and F10 abandonment state. |
| BN-10 | Player beacon lifecycle is construction → active → unsupported → degrading → dormant → restored. | missing | Infrastructure condition and decline patterns exist. | No beacon lifecycle. | F9/F10 | First Beacon must remain exempt. |
| BN-11 | Dormant beacon cannot spawn or provide ordinary beacon travel but remains physical and restorable by ordinary arrival. | missing | No beacon lifecycle or physical beacon. | No player evidence. | F9/F10 | Avoid stranding restorers; ordinary routes must remain possible. |
| BN-12 | Degradation is gradual, visible and warned. | partial | Settlement/infrastructure decline has readable stages and clues. | No beacon-specific warning. | F10 | Reuse proven staged state rather than timers hidden from players. |

### Tents, fences, inactivity and public space

| ID | Brief requirement | Status | Existing implementation evidence | Current player-facing evidence | Remaining owner | Dependency or risk |
|---|---|---|---|---|---|---|
| TP-01 | A tent claims only its footprint and can be placed on valid clear terrain. | missing | Registry plots/leases exist, but no physical tent placement. | No tent item or placement preview. | F2 | Needs authoritative collision and persistence. |
| TP-02 | Tent cannot overlap another structure or active beacon. | missing | Movement collision and claim validation patterns exist. | No placement path. | F2 | Baseline landmark occupancy must feed placement guards. |
| TP-03 | Tent cannot block a building entrance, protected road or essential route. | missing | Routes/infrastructure have stable records. | No placement path. | F2/F8 | Requires protected geometry, not only IDs. |
| TP-04 | Tent cannot deliberately trap another player. | missing | Movement collision exists. | No placement path. | F2 | Automated escape-path proof required. |
| FC-01 | Individual fence segments claim nothing; one closed fence with at least one gate registers a bounded enclosure. | missing | Lease claims do not model fence geometry. | No fence placement. | F8 | Polygon/closure checks must remain deterministic. |
| FC-02 | Enclosure cannot contain another player's protected property or active beacon. | missing | Claim ownership exists separately. | No enclosure. | F8 | Needs spatial cross-reference validation. |
| FC-03 | Enclosure cannot block roads, entrances, essential public resources or the only reasonable route. | missing | Route/infrastructure records exist. | No enclosure. | F8 | Pathfinding guard is a critical anti-grief boundary. |
| FC-04 | Maximum area and perimeter cost prevent extreme land grabs. | missing | Claim capacities exist, but not physical area/perimeter. | No enclosure. | F8 | Choose a readable bound without over-equalising land. |
| FC-05 | Fences block ordinary passage except through owner-controlled gates. | missing | World collision supports static water only. | No fence/gate world objects. | F8 | Dynamic collision must remain authoritative and recoverable. |
| FC-06 | Fences protect crops, contain animals and define yards/work/storage. | missing | Crops, one animal and claim records exist separately. | No enclosure effects. | F8 | Do not expand animal simulation beyond the brief. |
| FC-07 | Every active beacon has an unclaimable public commons with protected access. | missing | Baseline beacon record; public infrastructure model. | No beacon commons boundary. | F8/F9 | Commons geometry must be shared by placement guards. |
| FC-08 | Communal town walls are public projects with public gates, never one private enclosure. | missing | Governance/public project authority exists. | No walls. | F8 | Later content; guard private-claim model now. |
| IA-01 | Property is protected for three real months of owner inactivity. | conflicting | Current claims use a 90-day lease from approval/renewal, not inactivity protection. | Lease countdown is visible, but semantics differ. | F10 | Requires migration/decision separating lease renewal from inactivity. |
| IA-02 | After that period property becomes abandoned and reclaimable. | partial | Claim expiry, grace, abandonment and reclamation exist. | Registry status/reclaim path exists. | F10 | Trigger is not owner inactivity and physical property is absent. |
| IA-03 | Structures, fences and stored goods degrade gradually rather than disappear immediately. | missing | Infrastructure condition provides a pattern. | No personal structures/fences/storage degradation. | F10 | Must preserve history and prevent abrupt loss. |
| IA-04 | Abandoned sites can become ruins, salvage or restoration projects. | partial | Chronicle, reclamation and infrastructure recovery exist. | No physical ruin/salvage/restoration loop. | F10 | Avoid reward duplication during reclamation. |
| IA-05 | An abandoned tent stops supporting a player-built beacon. | missing | Neither tent support nor player beacon lifecycle exists. | No player evidence. | F10 | Depends on F2 and F9 stable ownership records. |

### Interface and anti-griefing

| ID | Brief requirement | Status | Existing implementation evidence | Current player-facing evidence | Remaining owner | Dependency or risk |
|---|---|---|---|---|---|---|
| UI-01 | Interactions primarily arise by approaching world objects/people/sites. | conflicting | Position validation exists for many actions. | A broad permanent sidebar remains the main action surface. | F1/F2/F4/F6 | Preserve visible recovery controls while contextualising actions. |
| UI-02 | Farm context exposes inspect/plant/water/tend/harvest. | partial | Position-bound farming endpoints and visible actions. | Plant/tend/harvest are visible; inspect/context and watering vocabulary are incomplete. | F3 | Do not add redundant watering if tending remains the chosen simple action. |
| UI-03 | Woodland and mine contexts expose logging/mining. | missing | Baseline resource records only. | No controls or results. | F2 | Requires foundational node mechanics. |
| UI-04 | Forge, NPC and construction-site contexts expose work, talk/trade/help and needs/contribution. | missing | Baseline records plus separate trade/project endpoints. | None are connected to world proximity. | F1/F4/F6 | Multiple phases must share one context-selection model. |
| UI-05 | Journal carries ambitions/discoveries/work; noticeboard carries local needs/opportunities/projects. | partial | Chronicle, knowledge, notices, demands and contracts exist. | Panels contain information, but no personal journal or world board. | F1/F7 | Avoid duplicating authoritative data in local saves. |
| UI-06 | Only relevant contextual actions are prominent; every required action has a visible tap/click path. | conflicting | Existing controls satisfy touch access for implemented systems. | Too many permanent controls are simultaneously prominent. | F1/F7 | Contextualisation must not remove touch recovery paths. |
| AG-01 | Narrowly protect beacons, commons, routes, entrances, claims, shared resources and escape paths. | partial | Authority validates bounds/collision/claims and no-PvP ownership; baseline supplies stable spaces. | No construction placement exists to exercise most guards. | F2/F8/F9 | Spatial rules must be automated before player placement ships. |
| AG-02 | Protections prevent critical-space grief without erasing early land advantage. | missing | No physical placement/commons acceptance fixture. | No player evidence. | F8 | Avoid global no-build buffers that sterilise settlement growth. |
| AG-03 | PvP, theft and destructive crime do not threaten foundational property. | usable | `/v1/law` disables PvP/theft; support mutations are audited. | Client displays protected-law boundary. | F8–F10 regression | Future law work must be opt-in and separately designed. |

### Cohesive playable target and success criteria

| ID | Brief requirement | Status | Existing implementation evidence | Current player-facing evidence | Remaining owner | Dependency or risk |
|---|---|---|---|---|---|---|
| IS-01 | Persistent world, permanent beacon, tent settlement and builder NPC. | partial | World persistence plus F0 baseline records. | Only persistence is currently usable. | F1 | This is the immediate track dependency. |
| IS-02 | Personal tents and basic world storage. | missing | Claims/inventories exist. | No placeable shelter or physical storage. | F2 | Storage must remain server-authoritative and location-bound. |
| IS-03 | Farming, logging, mining and exploration. | partial | Farming/exploration usable; F0 resource records. | Logging/mining absent. | F2/F3 | Keep each first loop small. |
| IS-04 | Rough forge, basic smithing, crude and improved tools. | missing | Skills/default equipment/F0 records only. | No connected production or comparison. | F4 | Depends on F2 resources. |
| IS-05 | A few crops, trees and minerals. | partial | Three crops and forest/stone tiles. | Crops work; trees/minerals are not resource entities. | F2/F3 | Avoid content breadth beyond the fixed proof. |
| IS-06 | Basic barter/direct trade. | usable | Atomic direct trade. | Visible trade review/accept/cancel. | F5 | Add specialisation proof, not a second trade system. |
| IS-07 | One communal construction project. | partial | Generic civic projects and storehouse-site baseline. | No visible contribution/transformation. | F6 | Storehouse is the selected first proof. |
| IS-08 | Contextual world interactions. | conflicting | Server checks positions. | Permanent command presentation dominates. | F1/F7 | Shared context architecture should span later activities. |
| IS-09 | Persistent resources, property and world changes. | partial | Inventory/crops/claims/civic/region state persist. | Resources and abstract claims persist; foundational physical property does not. | F2/F6/F8 | Avoid local client truth. |
| IS-10 | Two players can save each other meaningful time through economic connection. | missing | Trade and orders exist. | No fixed comparative production goal demonstrates the saving. | F5 | Must measure actions or world time. |
| SC-01 | New player tries several activities without a class. | partial | No classes; farming/exploration/root practice. | Logging/mining/smithing absent. | F7 | Depends on F2–F4. |
| SC-02 | Different players naturally prefer different activities. | missing | Automated role labels do not establish preference. | No human observational evidence. | F7 | Requires playtest, not telemetry alone. |
| SC-03 | Players trade because cooperation saves time. | missing | Atomic trade exists. | Time-saving motivation is unproven. | F5/F7 | Fixed comparison precedes human evidence. |
| SC-04 | A 15-minute visit feels useful. | missing | Farming actions can be short. | No timed human acceptance record. | F3/F7 | Tune cadence from observed play. |
| SC-05 | A longer session creates memorable improvement or discovery. | partial | Knowledge, projects, combat, travel and chronicle changes exist. | Automated playthroughs record outcomes; human evidence remains desired. | F7 | Player meaning cannot be claimed from server state. |
| SC-06 | Collected resources have visible purposes. | partial | Recipes, orders, markets and projects describe uses. | Foundational timber/mineral chains are absent or hidden in panels. | F4/F6/F7 | World sites should show needs before collection. |
| SC-07 | Tent camp visibly develops through player effort. | missing | Storehouse baseline record plus abstract projects. | No visual tent camp or staged structure. | F6/F7 | Requires persistent render state. |
| SC-08 | A home feels achievable but genuinely long-term. | missing | 90-day claim and service concepts only. | No house path. | F8 | Do not block F7 on post-foundation housing proof. |
| SC-09 | Nearby scarcity encourages frontier expansion. | partial | Regional scarcity, travel and pioneer expedition exist. | Expansion is a command-driven outpost flow, not First Beacon scarcity. | F9 | Needs F2 resources and physical property pressure. |
| SC-10 | A new beacon creates credible settlement opportunity. | missing | Multi-settlement region exists without player beacon causation. | No beacon project/lifecycle. | F9 | Reuse F1–F6 systems at the second site. |
| SC-11 | The world feels inhabited rather than like a command menu. | conflicting | Persistent players, NPC projections, world history and map exist. | Dense sidebar commands and abstract ledgers dominate. | F7 | Central product risk for the entire track. |

### Explicit foundational deferrals

These rows are intentionally not defects in F0–F7. The named phase owns the
guardrail that keeps them out of the foundational release; implementation
requires a later, separate decision.

| ID | Brief deferral | Status | Existing implementation evidence | Current player-facing evidence | Remaining owner | Dependency or risk |
|---|---|---|---|---|---|---|
| DD-01 | Detailed fertilizer specialisations. | deliberately deferred | No foundational requirement. | None required. | F3/F7 guardrail | Add only after simple farming pacing proves a need. |
| DD-02 | Complex soil simulation. | deliberately deferred | Weather/pest depth is bounded. | Simple farming remains available. | F3/F7 guardrail | Do not turn ordinary crops into expert-only play. |
| DD-03 | Advanced animal husbandry. | deliberately deferred | One authored goat and care action. | Basic Care exists. | F7 guardrail | Breeding/herds stay outside this track. |
| DD-04 | Large profession trees. | deliberately deferred | Root/merger catalogue exists, but F track needs only representative loops. | Practice/school surfaces exist. | F7 guardrail | Advanced catalogue must not obscure first activities. |
| DD-05 | Player government. | deliberately deferred | Advanced bounded mayor/governance already exists and is retained. | Town hall controls exist. | F1/F7 presentation guardrail | Existing system is not a prerequisite for foundational onboarding. |
| DD-06 | Bulk-freight restrictions on beacon travel. | deliberately deferred | Regional logistics remain separate. | No beacon travel yet. | F9 guardrail | Carry inventory; leave storage physical. |
| DD-07 | Complex settlement law. | deliberately deferred | Protected no-PvP boundary is explicit. | Law summary visible. | F8/F9 guardrail | Do not mix ownership with punitive law. |
| DD-08 | Advanced beacon specialisations. | deliberately deferred | No beacon lifecycle yet. | None. | F9 guardrail | Prove one generic second beacon first. |
| DD-09 | Dynamic NPC populations. | deliberately deferred | Fixed/household simulations exist, but brief selects fixed builder foundation. | Household summaries exist. | F1/F7 guardrail | Do not make migration a dependency for essential services. |
| DD-10 | Large-scale regional economies. | deliberately deferred | Phase 5 regional market is retained as an advanced system. | Market panel exists. | F7 presentation guardrail | Foundational proof remains two-player local savings. |
| DD-11 | PvP or destructive criminal play. | deliberately deferred | Protected law boundary. | PvP/theft shown disabled. | F8–F10 guardrail | Reopen only with opt-in safety/recovery design. |
| DD-12 | Extensive combat progression. | deliberately deferred | Bounded combat and skills exist. | Combat controls work. | F7 presentation guardrail | Combat must not displace settlement-life proof. |

## F0 fixture contract

`first-beacon-baseline-v1` is part of the validated `hearthlands` region
content and reaches the client only through the authoritative world snapshot.
Its stable landmarks are:

| Stable ID | Required role | Position |
|---|---|---:|
| `first-beacon` | permanent First Beacon | `(8,6)` |
| `first-beacon-tents` | tent settlement | `(6,4)` |
| `first-beacon-fire` | communal gathering place | `(8,5)` |
| `builder-mara` | fixed builder NPC | `(7,5)` |
| `first-beacon-noticeboard` | visible local-needs source | `(9,5)` |
| `first-beacon-cache` | shared collection/storage point | `(7,6)` |
| `first-beacon-tool-rack` | crude-tool access | `(9,6)` |
| `first-beacon-fields` | nearby farmland | `(2,8)` |
| `whisperwood-edge` | nearby woodland | `(13,3)` |
| `first-beacon-mine` | nearby mineable ground | `(10,3)` |
| `first-beacon-forge` | rough forge | `(10,5)` |
| `storehouse-site` | visible construction space | `(6,7)` |

Each landmark has exactly one stable interaction record with `authority:
"server"`. That assertion intentionally says nothing about current usability;
F1, F2, F4 and F6 own the visible and executable interaction paths.

## Verification record

Run the focused F0 gate from the project root:

```powershell
.\scripts\verify_foundation_baseline.ps1
```

On 2026-09-02 it passed. The script runs four focused Rust tests across the
protocol, content, repository and client projection, then starts two isolated
JSON-backed authoritative servers. It verifies all 12 landmarks and 12
interactions through `/v1/state`, compares the complete baseline byte-for-byte
after repeated clean fixture creation, checks stable identifiers and the
permanent arrival position, proves a chat/player mutation from run one is
absent from run two, and requires `/v1/ops/health` to report both `ready` and
`integrity_ok`.

The same final tree also passed:

- `.\scripts\validate_content.ps1` (15 manifests, including the new validated
  region baseline);
- `cargo test --workspace` (762 tests);
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`; and
- `.\publish.ps1` with no parameters.

The publisher built and packaged the Windows and WebGL clients, included the
registered assets and catalogue thumbnail, and deployed the preview artifact
successfully.

## Risks entering F1

1. **Presentation conflict:** the authoritative regional fixture describes a
   mature Hearth while the foundational arrival must feel like an undeveloped
   tent settlement. F1 needs a clear projection/presentation rule rather than
   deleting Phase 4–6 state.
2. **Menu-first interaction:** current touch controls are available but dense
   and permanent. F1 must establish a reusable proximity/context surface while
   preserving visible recovery actions and browser touch access.
3. **Records without art or behavior:** F0 landmark records are intentionally
   not rendered. Their `partial` statuses must not be promoted until players
   can see and use them through the connected client.
4. **Builder identity:** Mara is now a stable authored identifier, but dialogue,
   service inventory, construction authority and persistence policy must be
   added without colliding with the existing Bellweather household model.
5. **Property semantics:** the brief's three-month inactivity protection and
   physical fence/tent claims differ from the existing renewable registry
   lease. F8/F10 need an explicit reconciliation and migration decision.
6. **Deployment evidence:** browser API origin, target MySQL and human regional
   play remain open Phase 6 gates. They do not block deterministic F0, but F1's
   connected acceptance must run against an available authority.
