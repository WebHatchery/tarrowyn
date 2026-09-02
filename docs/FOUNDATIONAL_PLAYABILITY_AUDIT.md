# Foundational playability audit

Status: project-27 F5 connected cooperation implemented and accepted on
2026-09-02; subjective enjoyment and personal tent placement remain explicitly
unproven or missing.

This audit treats the attached **Tarrowyn Foundational Game Design Brief** as
the design authority and uses the GDD plus the historical Phase 0–6 records as
implementation evidence. `Usable` has a strict meaning here: a connected
player can complete the experience through the current client. A protocol
type, endpoint, content row, repository rule, or offline-only presentation is
not enough by itself.

## Readiness assessment

The release candidate is technically deep but not yet a cohesive foundational
settlement experience. Its connected client now opens on an undeveloped tent
camp with a visible First Beacon, Mara's builder introduction, a world
noticeboard, and touch-first nearby context. Nearby woodland and mine actions
now produce persistent timber, stone, and iron ore through shared crude tools;
their nodes deplete and recover, and the capacity-bounded shared cache keeps
goods at its physical camp location. The rough forge now consumes those same
goods to prepare charcoal and a handle, then makes a six-action iron field tool
whose fixed benefit is twice the three-action crude fallback. Personal shelter
and construction are still absent. Mature registry-lease and property
assumptions must be reconciled without removing the advanced systems.

Shared plots now present their authoritative crop outlook in the nearby deck
and expose touch-first plant, optional tend/water, and harvest actions. A
connected accelerated scenario proves 15 minutes of restart-safe offline crop
growth, untended maturity, harvest, replant, useful maintenance, replay, and
persistence. This proves the fixed short visit works; it does not substitute
for human evidence that the visit feels enjoyable.

The fixed First Beacon iron-tool goal now proves local interdependence without
a class gate. An uncommitted player can self-supply in six accepted
gather/forge actions. A player who voluntarily practises Mining to mastery two
extracts both required ore in one action, trades them atomically to a
logger/smith, and the two players complete the same tool in five actions. The
durable result records both contributions, the accepted trade, and one saved
action; touch-first nearby controls expose offer, acceptance, next forge work,
the measured result, and the unchanged solo fallback.

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
| usable | 39 |
| partial | 43 |
| missing | 36 |
| conflicting | 1 |
| deliberately deferred | 13 |

## Evidence keys

- **Baseline:** `assets/data/region.json`, `protocol/src/foundation.rs`,
  `server/src/content.rs`, `server/src/content/region_validation.rs`,
  `server/src/repository/observability.rs`, `src/network/projection.rs`.
- **Authority:** `server/src/repository.rs`, `server/src/repository/persistence.rs`,
  `server/src/http.rs`, `protocol/src/lib.rs`.
- **Connected client:** `src/network.rs`, `src/network/projection.rs`,
  `src/ui_online.rs`, `src/ui_online/controls.rs`, `src/ui_regional.rs`,
  `src/ui_foundation.rs`, `src/network/foundation.rs`.
- **F2 resources:** `server/src/repository/foundation.rs`,
  `server/src/repository/foundation/tests.rs`,
  `scripts/verify_foundation_resources.ps1`, and
  `FOUNDATIONAL_PLAYABILITY_F2_RUNBOOK.md`.
- **F3 farming:** `server/src/repository/world.rs`,
  `server/src/repository/tests/offline_crop_growth.rs`,
  `src/ui_foundation.rs`, `scripts/verify_foundation_farming.ps1`, and
  `FOUNDATIONAL_PLAYABILITY_F3_RUNBOOK.md`.
- **F4 forge:** `server/src/repository/foundation/forge.rs`,
  `src/network/foundation.rs`, `src/ui_foundation.rs`,
  `scripts/verify_foundation_forge.ps1`, and
  `FOUNDATIONAL_PLAYABILITY_F4_RUNBOOK.md`.
- **F5 cooperation:** `server/src/repository/foundation/cooperation.rs`,
  `server/src/repository/trades.rs`, `src/ui_foundation.rs`,
  `scripts/verify_foundation_cooperation.ps1`, and
  `FOUNDATIONAL_PLAYABILITY_F5_RUNBOOK.md`.
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
| FP-02 | One permanent starting beacon shared by all first arrivals. | usable | Baseline `first-beacon`, permanent at `(8,6)`; guest spawn and context use the same record. | The named beacon is rendered at every fresh arrival. | F1 regression | Later beacon lifecycle work must preserve its permanence. |
| FP-03 | Early arrivals may take desirable nearby land; later players seek farther opportunity. | partial | Claims, three starting registry plots, regional vacancies and frontier travel. | Claim and regional opportunity summaries exist. | F8/F9 | Registry abstractions do not yet express physical nearby scarcity. |
| FP-04 | No permanent class or profession selection. | usable | Skill catalogue and profession capabilities have no character class field. | No class-selection screen; visible Practice and profession actions exist. | F7 regression | Later onboarding must not add a disguised class choice. |
| FP-05 | A newcomer can try farming, logging, mining, exploration and smithing before specialising. | partial | Farming, exploration, persistent logging/mining and unrestricted forge work exist. | Each activity is independently playable without a class; the cohesive first-hour sequence remains unproven. | F7 | Preserve broad access while integrating the first-hour path. |
| FP-06 | A sufficiently committed player may eventually participate in every activity. | partial | Root skills have direct practice paths and no exclusive class gates. | Player can practise roots, but several foundational activities have no loop. | F7 | Preserve broad access while adding efficiency differences. |
| FP-07 | Interdependence comes from limited time and inconvenient self-supply, not arbitrary lockouts. | partial | Trade, orders, regional scarcity and material escrow create demand. | Trade/order controls exist; no fixed time-saving foundational comparison. | F5/F7 | Current credentials can feel like gating unless crude fallbacks are clear. |
| FP-08 | Specialists gain value from knowledge, tools, facilities, efficiency, quality and supply relationships. | partial | Skills, knowledge, tool condition, service quality, market and orders exist. | Several values are visible, but foundational activities do not connect them. | F4/F5/F7 | Advanced surfaces currently precede a legible basic chain. |
| FP-09 | Foundational activities are simple to begin; optional depth improves rather than invalidates the base action. | partial | Plant/tend/harvest, shared crude gathering, and unrestricted forge work; weather, pests, and improved tools add depth. | Nearby controls lead each base action and the crude tool remains viable; the broader first-hour hierarchy remains. | F7 | Avoid exposing advanced complexity before its purpose is felt. |
| FP-10 | Add deeper layers only after players experience the problem they solve. | partial | Phase 4–6 systems remain intact behind **All tools**. | Arrival now promotes one nearby world action; broader first-hour sequencing remains. | F7 | Preserve recovery access while reducing later-system prominence. |
| FP-11 | Progress is visible through tools, fields, shelter, storage, knowledge, trade, reputation, contributions, beacons and history rather than a global level. | partial | Tool condition, crops, cache contents, skills, trades, reputation, claims, outpost and chronicle persist. | Nearby cache choices expose storage progress; personal shelter, building stages and player beacons remain absent. | F6–F9 | Existing generic skill number may read like a level. |
| FP-12 | An ordinary session can feel worthwhile without a level or major unlock. | partial | Farming, trade, contract and travel produce small outcomes. | Automated role loop exists; human evidence is explicitly still desired. | F3/F7 | Product enjoyment cannot be inferred from deterministic endpoints. |

### Initial session experience

| ID | Brief requirement | Status | Existing implementation evidence | Current player-facing evidence | Remaining owner | Dependency or risk |
|---|---|---|---|---|---|---|
| IH-01 | Arrive at the First Beacon in a settlement primarily made of tents. | usable | The stable beacon/tent fixture is projected while mature regional overlays are presentation-suppressed at arrival. | The connected client opens on the rendered tent camp. | F1 regression | Advanced state must remain available without replacing the arrival view. |
| IH-02 | Meet the builder as the first foundational NPC. | usable | Stable `builder-mara` landmark plus proximity-checked interaction endpoint. | Mara is rendered, labelled and reachable through **Talk to Mara**. | F6 extension | Construction contributions remain deliberately later. |
| IH-03 | Learn what the settlement currently needs. | usable | Stable board record and authoritative `read-local-needs` response. | **Read local need** names timber and stone for the first storehouse. | F6 regression | Keep one clear initial need as project systems expand. |
| IH-04 | Walk through the surrounding world rather than operate through a command menu. | usable | Server movement, collision and reusable nearest-landmark context selection. | Press-and-hold map movement and one nearby action form the primary path; **All tools** is secondary. | F2 regression | New gathering contexts must reuse this touch-first surface. |
| IH-05 | Try basic farming. | usable | `/v1/farming/actions`, persistent plots, deterministic tests and the connected F3 harness. | Nearby **Plant crop**, **Tend / water**, and **Harvest crop** controls follow the crop state. | F3 regression | Current fields are shared and server-authoritative. |
| IH-06 | Try logging and mining. | usable | Persistent timber/stone/ore nodes, shared crude tools, proximity checks and replay-safe resource commands. | Nearby **Gather timber** and **Mine stone** controls grant authoritative inventory yields. | F2 regression | Keep both actions touch-first and readable when a node is depleted. |
| IH-07 | Try exploration and observe/use the rough forge. | usable | Movement, regional travel/map, rendered forge, and authoritative typed forge recipes. | The local world path reaches a touch-first rough forge that exposes and executes its production chain. | F4 regression | Travel UI must not substitute for local discovery. |
| IH-08 | See cross-activity support and leave with a modest future-session goal. | missing | Server systems contain material/order connections. | No first-hour chain or journal goal makes the connection legible. | F7 | Requires earlier activity proofs and a visible non-class goal surface. |
| SS-01 | In about 15 minutes, inspect offline-grown crops, harvest, tend/water and replant. | usable | The connected F3 harness models a 15-minute absence, proves untended maturity, and completes harvest, replant, optional tending, replay and restart. | The nearby deck exposes crop stage and the matching touch action at every shared plot. | F3 regression | Human pacing observation remains desirable but is not required for the deterministic action proof. |
| SS-02 | In a short visit, check tools, fencing or storage. | partial | Field-tool condition and authoritative shared-cache contents are visible and restart-safe. | Crop outlook names tool condition; nearby storage can be checked, but physical fencing is absent. | F8 | Keep maintenance useful rather than mandatory busywork. |
| LS-01 | In 60–90 minutes, discover/barter seeds, gather fencing timber, commission a tool, reorganise land, trade harvest and contribute surplus. | partial | Crops, logging, direct trade, service orders, claims and projects exist separately. | Timber gathering now works, but fences, physical construction and the complete chain do not. | F7/F8 | Cross-system sequence is unproven by a human session. |
| LS-02 | Short sessions maintain a life; longer sessions create new possibilities. | missing | No acceptance fixture compares these rhythms. | No connected journal/life presentation demonstrates the distinction. | F3/F7 | Requires pacing evidence, not additional breadth. |

### Foundational activities and economic connections

| ID | Brief requirement | Status | Existing implementation evidence | Current player-facing evidence | Remaining owner | Dependency or risk |
|---|---|---|---|---|---|---|
| FA-01 | Farming is a small connected foundational activity. | usable | Three crops, shared plots, offline server growth, inventory and connected F3 acceptance. | The nearby plot outlook provides the full visible plant/tend/harvest loop. | F3/F7 regression | Keep tending valuable and optional as later production connects. |
| FA-02 | Logging produces timber, firewood, charcoal material, fence parts and handles. | partial | Authoritative logging produces persistent timber; forge recipes convert it into charcoal and tool handles. | **Gather timber**, **Burn charcoal**, and **Shape tool handle** form one connected path; firewood and fence parts remain absent. | F6 | Add further derivatives only when construction consumes them. |
| FA-03 | Mining produces stone, clay, coal and metal ore. | partial | Authoritative mining produces persistent stone and iron ore from bounded deposits, and the forge consumes that ore. | **Mine stone** supplies the iron-tool recipe; clay and coal remain absent. | F6 | Keep the mineral set small until construction requires more. |
| FA-04 | Exploration discovers deposits, seeds, routes and settlement sites. | partial | Movement/travel, routes, regional sites and event projections exist. | Routes and locations are visible; discoveries are prelisted rather than found. | F2/F9 | Avoid turning exploration into a menu reveal. |
| FA-05 | Basic smithing combines ore, fuel and components into useful tools. | usable | Proximity-checked forge atomically combines two ore, charcoal, and a timber handle into a typed iron tool. | The nearby deck exposes every preparation and crafting step plus the resulting `6/6` tool. | F4 regression | Preserve exact recipe accounting and replay safety. |
| FA-06 | Barter/direct trade is basic and atomic. | usable | `/v1/trades` atomically moves the fixed two-ore offer and returns stable retry/restart responses. | Nearby **Offer 2 ore** and **Accept 2 ore** actions share the existing review/accept/cancel ledger. | F5 regression | Preserve the canonical trade authority; do not add a cooperation inventory. |
| FA-07 | Construction works through the builder NPC. | missing | Baseline builder/site records; civic projects exist. | No player-builder interaction or material contribution loop. | F6 | Reuse authoritative project ledgers; do not add parallel persistence. |
| EC-01 | Farmers produce food, seeds and useful plant material. | partial | Crops and seeds exist and trade. | Food crops/seeds are visible; other plant materials are absent. | F3/F7 | Minimal additions should serve a real production chain. |
| EC-02 | Loggers support fuel, fencing and tool components. | partial | Connected timber supplies forge charcoal and handles through atomic recipes. | A logger can supply both forge derivatives; fencing remains absent. | F6 | Reuse timber for only the selected construction proof. |
| EC-03 | Miners support construction and smithing inputs. | partial | Connected iron ore is consumed by the rough forge; stone remains durable and stored. | Mining now supplies smithing directly; construction consumption remains absent. | F6 | Reuse these same goods for construction. |
| EC-04 | Blacksmiths make useful tools from ore, fuel and other components. | usable | The unrestricted forge consumes gathered ore, prepared fuel, and a handle for an iron field tool. | Touch controls complete the whole recipe and show the resulting capacity. | F4 regression | Later specialisation may improve efficiency, not gate the base loop. |
| EC-05 | Explorers reveal resources, routes and settlement opportunities. | partial | Regional map, events, locations and pioneer outpost exist. | Players can inspect known routes/sites; revealing them is absent. | F2/F9 | Stable discovery state must remain server-owned. |
| EC-06 | Builders consume money/materials and create visible structures. | partial | Governance/public projects consume treasury; baseline builder/site records. | Public actions are ledgers, not builder-led structures appearing in the world. | F6 | Project completion must transform authoritative map state. |
| EC-07 | Crude tools always provide fallback; improved tools measurably save time/actions/materials. | usable | Every identity defaults to crude access; iron tools double useful field actions, and voluntary Mining practice reduces the iron-tool production path from six to five actions through barter. | Connected acceptance measures both benefits while fresh solo players retain the complete crude self-supply path. | F4/F5 regression | Later balance changes must retain measured benefits and non-blocking fallback. |

### First settlement and builder

| ID | Brief requirement | Status | Existing implementation evidence | Current player-facing evidence | Remaining owner | Dependency or risk |
|---|---|---|---|---|---|---|
| FS-01 | Permanent First Beacon. | usable | Stable validated baseline record and permanent arrival at `(8,6)`. | Rendered beacon exposes **Inspect beacon**. | F1 regression | Must never degrade in later beacon lifecycle work. |
| FS-02 | Tent settlement. | usable | Stable tent-camp record owns the foundational presentation. | Canvas shelters are visible west of the beacon. | F2 regression | Player tents must remain visually distinct. |
| FS-03 | Communal fire/gathering place. | usable | Stable fire record and authoritative gather inspection. | Visible fire exposes **Warm by fire** without making it mandatory. | F1 regression | Keep its social value soft. |
| FS-04 | Builder NPC. | usable | Stable Mara record and proximity-checked dialogue. | **MARA** is a visible spatial anchor with a touch action. | F6 extension | Her construction service remains later. |
| FS-05 | Noticeboard or visible local-needs source. | usable | Stable board record and authoritative local-need copy. | **NEEDS** is visible in the world and readable nearby. | F6 regression | Project expansion must preserve a clear first need. |
| FS-06 | Basic shared storage or collection point. | usable | Stable cache record, 64-item authority-owned inventory, atomic transfers, replay protection and restart persistence. | Nearby **Store**, **Collect**, and **Inspect cache** controls expose the shared goods. | F2 regression | Preserve physical proximity and owner-safe accounting. |
| FS-07 | Crude-tool access. | usable | The stable rack projects a shared hand axe and stone pick available to all identities. | Logging and mining work without a profession gate or private specialist tool. | F4 regression | Improved tools may save work but must not remove this fallback. |
| FS-08 | Nearby farmland. | usable | Manifest farm plots, map field tiles and farming endpoints. | Fields and crop interactions are visible. | F3 polish | Ensure First Beacon presentation points players there. |
| FS-09 | Nearby woodland. | usable | Stable woodland and timber-node records with bounded depletion, recovery, replay and persistence. | The map-visible woodland exposes **Gather timber** within one tile. | F2 regression | Node recovery must remain server-timed and capacity bounded. |
| FS-10 | Nearby mineable ground. | usable | Stable shallow-seam node with bounded stone/ore deposits, recovery, replay and persistence. | The map-visible seam exposes **Mine stone** within one tile. | F2 regression | Keep local deposits distinct from regional market stock. |
| FS-11 | Rough forge. | usable | Stable rendered landmark, typed recipes, proximity authority, persistence, and replay ledger. | Nearby touch controls inspect, prepare, and forge at the physical site. | F4 regression | Keep the forge connected to gathered inventory. |
| FS-12 | Visible space for future construction. | partial | Stable `storehouse-site` record. | Site is not rendered. | F6 | F1 may render it without enabling contribution early. |
| FS-13 | Early projects include storehouse, well, fenced growing area, smithing shelter, carpenter yard and first homes. | partial | Civic project types and infrastructure framework exist. | Named foundational projects are not visible construction choices. | F6/F8 | Keep initial scope to the storehouse before expanding. |
| FS-14 | The first communal project visibly shows what contributed materials will create. | missing | Baseline storehouse site and project ledgers are separate. | No unfinished structure or contribution forecast. | F6 | Needs staged authoritative state and replay-safe contribution. |
| BP-01 | Builder introduces construction and settlement development. | usable | Mara's authoritative dialogue introduces the first storehouse and points to its need. | Players meet her through the nearby context action. | F6 extension | Introduction must lead into, not duplicate, the later contribution loop. |
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
| HB-03 | Stored items remain physically where stored; beacon travel does not teleport remote storage. | partial | The First Beacon cache has a stable landmark ID and server-owned inventory separate from carried goods. | Cache access requires physical proximity and survives restart; beacon travel isolation is unproven. | F9 | Preserve cache location identity when beacon travel is added. |
| BN-01 | First Beacon always exists and never degrades. | usable | Permanent validated baseline record is excluded from mutable beacon state. | It is rendered as the shared arrival anchor. | F9 regression | F9 lifecycle must explicitly continue excluding this ID. |
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
| UI-01 | Interactions primarily arise by approaching world objects/people/sites. | partial | Reusable nearest-landmark selection and authoritative proximity validation now cover F1. | Arrival promotes one nearby world action; later activities still use broader tools. | F2/F4/F6 | Extend the same context boundary rather than adding new permanent rows. |
| UI-02 | Farm context exposes inspect/plant/water/tend/harvest. | usable | Position-bound farming endpoints plus deterministic nearby crop-choice tests. | Approaching any shared plot shows its outlook and **Plant crop**, **Tend / water**, or **Harvest crop** without keyboard input. | F3 regression | Tending intentionally carries the watering vocabulary instead of adding a redundant action. |
| UI-03 | Woodland and mine contexts expose logging/mining. | usable | Nearest-landmark context selects typed resource commands against authoritative node records. | **Gather timber** and **Mine stone** appear beside their sites and report accepted yields or depletion. | F2 regression | Do not duplicate these actions in a permanent command row. |
| UI-04 | Forge, NPC and construction-site contexts expose work, talk/trade/help and needs/contribution. | partial | Shared landmark context drives Mara, board, and dynamic forge work. | NPC talk and forge work are touch-first; construction needs/contribution remain absent. | F6 | Extend the same context model for contribution. |
| UI-05 | Journal carries ambitions/discoveries/work; noticeboard carries local needs/opportunities/projects. | partial | Chronicle, knowledge, notices, demands, contracts and world noticeboard context exist. | The board carries the first local need; a cohesive personal journal remains absent. | F7 | Avoid duplicating authoritative data in local saves. |
| UI-06 | Only relevant contextual actions are prominent; every required action has a visible tap/click path. | partial | The F1–F5 nearby deck selects physical work plus eligible barter while **All tools** remains available. | Gathering, farming, forge preparation, ore offer/acceptance and recovery are touch-capable without a permanent command wall; construction and the cohesive first hour remain incomplete. | F6/F7 | Apply the same hierarchy to construction and the complete journey. |
| AG-01 | Narrowly protect beacons, commons, routes, entrances, claims, shared resources and escape paths. | partial | Authority validates bounds/collision/claims and no-PvP ownership; baseline supplies stable spaces. | No construction placement exists to exercise most guards. | F2/F8/F9 | Spatial rules must be automated before player placement ships. |
| AG-02 | Protections prevent critical-space grief without erasing early land advantage. | missing | No physical placement/commons acceptance fixture. | No player evidence. | F8 | Avoid global no-build buffers that sterilise settlement growth. |
| AG-03 | PvP, theft and destructive crime do not threaten foundational property. | usable | `/v1/law` disables PvP/theft; support mutations are audited. | Client displays protected-law boundary. | F8–F10 regression | Future law work must be opt-in and separately designed. |

### Cohesive playable target and success criteria

| ID | Brief requirement | Status | Existing implementation evidence | Current player-facing evidence | Remaining owner | Dependency or risk |
|---|---|---|---|---|---|---|
| IS-01 | Persistent world, permanent beacon, tent settlement and builder NPC. | usable | Persistent world plus stable rendered records and authoritative Mara interaction. | The complete F1 arrival slice is connected and restartable. | F1 regression | Preserve it as later systems grow. |
| IS-02 | Personal tents and basic world storage. | partial | The authoritative, location-bound shared cache now supplies basic world storage. | Storage is touch-usable; personal tent placement remains absent. | Tent-placement follow-up | Do not mistake shared storage for personal shelter completion. |
| IS-03 | Farming, logging, mining and exploration. | usable | Farming/exploration plus persistent logging and mining use server-owned actions and state. | Each activity has a connected, visible client path. | F2/F3 regression | Keep each first loop small and spatial. |
| IS-04 | Rough forge, basic smithing, crude and improved tools. | usable | Physical forge recipes consume F2 materials; typed crude and iron tools have three- and six-action capacities. | The connected touch path prepares inputs, forges iron, measures its benefit, and preserves crude access. | F4 regression | Keep condition and recipe state durable. |
| IS-05 | A few crops, trees and minerals. | usable | Three crops plus bounded timber, stone and iron-ore deposits are authoritative resource state. | Crops, woodland timber and shallow-seam minerals are productively reachable. | F2/F3 regression | Avoid content breadth beyond the fixed proof. |
| IS-06 | Basic barter/direct trade. | usable | Atomic direct trade now carries all fixed foundation materials and anchors the durable cooperation result. | Exact ore offer/accept quick actions plus visible general review/accept/cancel. | F5 regression | Keep trade canonical and replay-safe. |
| IS-07 | One communal construction project. | partial | Generic civic projects and storehouse-site baseline. | No visible contribution/transformation. | F6 | Storehouse is the selected first proof. |
| IS-08 | Contextual world interactions. | partial | Server checks stable interaction IDs and authoritative distance; client selects the nearest landmark. | F1 interactions are contextual; later activities still need migration. | F2/F4/F6/F7 | Reuse the shared architecture. |
| IS-09 | Persistent resources, property and world changes. | partial | Inventory/crops/claims/civic/region state persist. | Resources and abstract claims persist; foundational physical property does not. | F2/F6/F8 | Avoid local client truth. |
| IS-10 | Two players can save each other meaningful time through economic connection. | usable | The connected F5 harness records two contributors completing the iron-tool goal in five accepted actions versus the six-action solo baseline. | Nearby status reports `5/6 accepted actions` and `1 saved through barter`. | F5 regression | Human preference remains separate evidence; preserve the measurable saving. |
| SC-01 | New player tries several activities without a class. | partial | No classes; farming, exploration, logging, mining, smithing, and root practice work. | All selected activities are independently available without selection; one cohesive first-hour pass remains unproven. | F7 | Integrate rather than add a class gate. |
| SC-02 | Different players naturally prefer different activities. | missing | Automated role labels do not establish preference. | No human observational evidence. | F7 | Requires playtest, not telemetry alone. |
| SC-03 | Players trade because cooperation saves time. | usable | The fixed two-ore trade is the causal bridge between one-action specialist extraction and the five-action shared result. | Both clients see five together versus six solo before offering/accepting and one saved afterward. | F5/F7 regression | Automation proves the incentive and path, not subjective preference. |
| SC-04 | A 15-minute visit feels useful. | partial | The connected F3 scenario fits inspect, offline growth, harvest, replant and optional upkeep into one modeled 15-minute return. | The fixed useful actions are visible, but subjective value has not been observed with a human player. | F7 | Tune cadence from observed play; automation cannot establish enjoyment. |
| SC-05 | A longer session creates memorable improvement or discovery. | partial | Knowledge, projects, combat, travel and chronicle changes exist. | Automated playthroughs record outcomes; human evidence remains desired. | F7 | Player meaning cannot be claimed from server state. |
| SC-06 | Collected resources have visible purposes. | partial | Noticeboard needs, cache storage, and forge recipes use the same gathered inventory. | Timber and ore now expose a physical production purpose; settlement contribution remains absent. | F6/F7 | World sites should show needs before collection. |
| SC-07 | Tent camp visibly develops through player effort. | missing | Storehouse baseline record plus abstract projects. | No visual tent camp or staged structure. | F6/F7 | Requires persistent render state. |
| SC-08 | A home feels achievable but genuinely long-term. | missing | 90-day claim and service concepts only. | No house path. | F8 | Do not block F7 on post-foundation housing proof. |
| SC-09 | Nearby scarcity encourages frontier expansion. | partial | Regional scarcity, travel and pioneer expedition exist. | Expansion is a command-driven outpost flow, not First Beacon scarcity. | F9 | Needs F2 resources and physical property pressure. |
| SC-10 | A new beacon creates credible settlement opportunity. | missing | Multi-settlement region exists without player beacon causation. | No beacon project/lifecycle. | F9 | Reuse F1–F6 systems at the second site. |
| SC-11 | The world feels inhabited rather than like a command menu. | partial | Persistent players and the authored camp now lead the arrival presentation. | Mara, the board, tents and nearby sites occupy the world; advanced ledgers remain secondary. | F7 | The full first hour still needs the same treatment. |

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

The F0 tree also passed:

- `.\scripts\validate_content.ps1` (15 manifests, including the new validated
  region baseline);
- `cargo test --workspace` (762 tests);
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`; and
- `.\publish.ps1` with no parameters.

The publisher built and packaged the Windows and WebGL clients, included the
registered assets and catalogue thumbnail, and deployed the preview artifact
successfully.

The project-27 F2 resource proof is documented in
[`FOUNDATIONAL_PLAYABILITY_F2_RUNBOOK.md`](FOUNDATIONAL_PLAYABILITY_F2_RUNBOOK.md)
and can be repeated with:

```powershell
.\scripts\verify_foundation_resources.ps1
```

On 2026-09-02 that harness passed 11 focused client tests, 16 focused server
tests, and its live HTTP walk through mining, timber depletion/recovery,
shared-cache transfer, replay, server restart, and durable state recovery. The
integrated tree passed 788 workspace tests across protocol, server, client and
integration targets, clippy with warnings denied, formatting and source-size
checks, then Windows/WebGL preview publication.

The eight rows promoted to `usable` are `IH-06`, `FS-06`, `FS-07`, `FS-09`,
`FS-10`, `UI-03`, `IS-03`, and `IS-05`. Six broader production or property
requirements moved from `missing` to `partial`: `FA-02`, `FA-03`, `EC-02`,
`EC-03`, `HB-03`, and `IS-02`. The matrix remains exactly 132 rows.

The project-27 F3 farming proof is documented in
[`FOUNDATIONAL_PLAYABILITY_F3_RUNBOOK.md`](FOUNDATIONAL_PLAYABILITY_F3_RUNBOOK.md)
and can be repeated with:

```powershell
.\scripts\verify_foundation_farming.ps1
```

On 2026-09-02 it passed 8 client nearby-field tests, 11 farming-authority
tests, 5 offline-growth tests, and the live 15-minute modeled-absence scenario
through two server restarts. The full tree passed 797 tests, clippy, formatting,
diff and source-size checks, followed by Windows/WebGL preview publication.

F3 promotes `SS-01` and `UI-02` from `partial` to `usable`. It moves `SC-04`
from `missing` to `partial`, because the fixed visit now produces a concrete
outcome while human enjoyment remains unobserved. No other audit row advances;
the matrix remains exactly 132 rows.

The project-27 F4 production proof is documented in
[`FOUNDATIONAL_PLAYABILITY_F4_RUNBOOK.md`](FOUNDATIONAL_PLAYABILITY_F4_RUNBOOK.md)
and can be repeated with:

```powershell
.\scripts\verify_foundation_forge.ps1
```

On 2026-09-02 it passed 10 client nearby-context tests, 3 forge-authority
tests, and a connected two-identity scenario covering gathering, every recipe,
same-request retries, an exact 3-versus-6 useful-action comparison, restart,
replay persistence, and repository integrity. The full tree passed 810 tests,
clippy, formatting, diff and source-size checks, followed by Windows/WebGL
preview publication.

F4 promotes `IH-07`, `FA-05`, `EC-04`, `EC-07`, `FS-11`, and `IS-04` to
`usable`. The first and last two were previously `partial`; `FA-05`, `EC-04`,
and `IS-04` were `missing`. Broader rows remain conservative where fencing,
construction, first-hour cohesion, or human experience is still absent. The
matrix remains exactly 132 rows.

The project-27 F5 cooperation proof is documented in
[`FOUNDATIONAL_PLAYABILITY_F5_RUNBOOK.md`](FOUNDATIONAL_PLAYABILITY_F5_RUNBOOK.md)
and can be repeated with:

```powershell
.\scripts\verify_foundation_cooperation.ps1
```

On 2026-09-02 it passed focused touch and authority tests plus a live
three-identity HTTP scenario covering voluntary practice, specialist yield,
exact atomic barter, contribution attribution, five-versus-six work, retries,
restart persistence, integrity, and unrestricted solo completion. The full
tree passed 823 tests before the documentation-only reconciliation, clippy,
formatting, diff and source-size checks, followed by Windows/WebGL preview
publication.

F5 promotes `IS-10` and `SC-03` from `missing` to `usable`. `FA-06`, `EC-07`,
`UI-06`, and `IS-06` gain stronger implementation/player evidence without
changing status. Construction, first-hour cohesion, and subjective player
preference remain conservative. The matrix remains exactly 132 rows.

## Risks after the F5 cooperation boundary

1. **Placement safety:** personal tents still need visible touch placement plus
   collision, commons, route, entrance and escape-path rejection without
   prematurely implementing F8's permanent enclosures.
2. **Construction connection:** timber, stone and iron ore now feed storage,
   barter and the rough forge, but F6 must connect the same inventory to the
   storehouse without a parallel material source.
3. **Mature-state compatibility:** foundational nodes and cache state must
   continue to coexist with Phase 4–6 regional stock and claims without
   becoming a second source of truth.
4. **Deployment evidence:** browser API origin, target MySQL and human regional
   play remain open Phase 6 gates. They do not block the isolated F1 acceptance
   or F2/F3 connected fixtures but still require target-environment proof.
