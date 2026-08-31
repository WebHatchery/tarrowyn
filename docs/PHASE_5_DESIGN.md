# Phase 5 decision record — the Hearthlands

## Region and topology

The first regional proof is the bounded `hearthlands` region. It contains the
original settlement, The Hearth; the Phase 3 frontier site, Whisperwood Watch;
and the additional Saltmere Landing settlement. The three locations have
different roles and resource profiles. The current topology is one authoritative
server process with region-owned records and location-aware projections. This
is intentionally a routing boundary that can later become a worker boundary;
the prototype does not pretend to provide global seamless-world guarantees.

`GET /v1/region` returns only locations, routes, and settlement projections in
the player's interest radius. `GET /v1/settlements` remains the comparison view
for the small regional fixture. A player retains the origin location while a
travel record is `Travelling` or `Interrupted`; only the server tick may move
the character to the destination. A reconnect reads the same durable travel
record and event cursor. Repeating a request ID returns the original response.
The client advances the regional event stream from the returned cursor and
merges changed event records by stable event ID, so a stage transition does
not erase earlier regional history. If a restore moves the cursor backward,
the regional stream drops its cache and restarts from cursor zero. If a cursor
falls before the retained event window, the server returns `cursor_stale`; the
client uses the same bounded-history reset rather than accepting a silent gap.
The frontier chronicle and opportunity refreshes report ordinary failures and
send the same reset signal when their chronicle cursor crosses that boundary.

Each settlement projection retains its newest 64 local chronicle entries. Older
entries remain available through the shared durable chronicle archive and
authenticated history search rather than growing the regional projection
without bound.

Travel and combat recovery are also an explicit boundary: a knocked-out
character cannot start, resume, or alter a regional journey until a recovery
choice clears the knockout. The server enforces this even when a client sends a
direct travel request outside the visible touch controls.

The three routes are a threatened north pack road, an operational Saltmere
ferry, and a delayed frontier watch trail. Repair, escort, and improvement are
server-owned logistics actions. An interruption preserves character, cargo,
and rewards. `Recover` and `Resume` move the existing journey forward; they do
not create a second journey. Player travel may use each recorded road or ferry
from either endpoint, while market orders keep their directional origin and
destination for logistics and fulfilment.

## Economy and calendar

The regional market escrows goods at the origin and settles an order only at
the destination. Seeds and crops use the character inventory; timber, stone,
and bandages use location stock. Saltmere is abundant in stone and bandages;
Whisperwood is abundant in timber and iron salvage; The Hearth is abundant in
wheat and seeds. Scarcity, route risk, infrastructure, and settlement
condition alter the visible price index. Open orders, failed fulfilment, and
stock notes are exposed as telemetry, and failed-fulfilment history reaches the
order's recorded origin and destination. A failed order is marked `Failed`
rather than silently destroying the escrow record; support can reconcile it.
If an essential crop or bandage shipment lacks player escrow, a bounded
travelling-service fallback can carry at most two goods, twice per world day.
It adds a surcharge, arrives after a short delay, and marks `fallback_used`;
because no player goods entered escrow, cancellation and support reconciliation
never create a refund for that order. Timber and stone remain player or
regional-stock shipments.

The regional market keeps at most 128 order records. Fulfilled and cancelled
history makes room for new shipments, while open and failed orders remain
addressable for fulfilment or support escrow recovery; creation fails closed if
all retained records are still live. A rejected create request also leaves
retained history unchanged when its requested goods are unavailable.

The real-time calendar is locked before leases or crop promises depend on it:
one game day is 80 real minutes. The 14-day season and four-season year (56
game days) remain development fixtures pending pacing validation. Phase 5 names
the seasons thaw, greenrise, harvest, and
deepwinter. Seasons change opportunity and route pressure but do not close
essential services. The online header shows the current server-projected season
beside the calendar day, while Phase 6 owns the long-session calendar
compatibility check and final pacing decision.

Settlement activity is local rather than a regional broadcast: active character
presence supports the nearest settlement, and open market orders pulse activity
at their origin or destination. Each decision interval removes one activity
point when nothing is supporting a settlement. Low activity combined with weak
industry or governance exposes a strained or quiet condition, while the
existing recovery opportunity remains visible for a later player.

The touch client places the current settlement's condition beside travel status
and marks a recovery opportunity as open. The comparison data remains
available in the same projection so a player can see the wider region without
leaving the shared-road sidebar; the local line also carries compact claim,
free-plot, and public-work counts.

The shared-road sidebar also exposes compact road availability/risk, open market
orders, and the protected-law boundary. A visible Inspect control opens the
authoritative route names, status, condition, and risk beside the first stock
and price notes without squeezing them into the touch summary. The same panel
offers visible Escort and Improve controls for the first open route connected
to the current location, while Repair remains in the sidebar recovery row.
The latest regional event stage is shown beside that compact telemetry so
signals and escalations remain visible before intervention. The map draws the region's
server-owned location positions and route status colours; offline play keeps its
local-only landmark fixture.
The client polls the regional household endpoint and shows the current travelling
service status in the same compact line, while the household's reason and history
remain server-owned projection data. The movement history keeps its latest 64
entries during live updates, snapshot loading, and support merges so a long-lived
household cannot make the regional projection grow without limit.

Each settlement projection also rolls up its nearest recognised claims, free
plots, and public works. The bounded content fixture therefore gives Hearth,
Whisperwood, and Saltmere distinct facility signals while the Phase 4 registry
and infrastructure records remain the authoritative mutation surfaces.
Route status moves safety toward a readable local target, public-work condition
moves infrastructure toward its authoritative average, local support moves
industry and remote governance, and Hearth governance follows the Phase 4
administration-quality record. These changes are deliberately bounded to one
step per decision interval.

## Event lifecycle

The seeded `river-thaw` fixture follows signal, escalation, intervention,
resolution, and aftermath. Its manifest names the locations it affects, and it
crosses all three launch locations while recording effects on travel risk,
farming supply, market prices, and household confidence. The server accepts
only an exact option from the event's visible intervention list;
arbitrary client text cannot manufacture a regional choice or effect. The
current thaw choices have bounded distinct consequences: ferry repair steadies
the landing route, caravan escort restores food movement while leaving the road
watched, and the frontier storehouse opens seed reserve at Whisperwood. Every
cause, intervention, and outcome is recorded through the existing chronicle
cursor, so a later player can search what the region remembers.
All settlement, route, and fixed-household event consequences now follow the
event's affected-location scope, so a future localized event cannot mutate
unlisted settlements while its history remains correctly scoped.
Content validation also requires each intervention to include every location
its implemented effect targets.

## Law boundary

Tarrowyn does not select PvP for this release. `GET /v1/law` is explicit:
`pvp_enabled` and `theft_enabled` are false, while claims, trades, travel, and
recovery are protected. There is no accidental mixture of player ownership
and theft rules. Character defeat remains a bounded recovery state, and support
repair is audited. Any future PvP proposal must replace this decision record
and add consent, evidence, protected spaces, consequences, reporting, and
recovery fixtures before exposing ownership mechanics.
