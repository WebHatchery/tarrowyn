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
the regional stream drops its cache and restarts from cursor zero.

Travel and combat recovery are also an explicit boundary: a knocked-out
character cannot start, resume, or alter a regional journey until a recovery
choice clears the knockout. The server enforces this even when a client sends a
direct travel request outside the visible touch controls.

The three routes are a threatened north pack road, an operational Saltmere
ferry, and a delayed frontier watch trail. Repair, escort, and improvement are
server-owned logistics actions. An interruption preserves character, cargo,
and rewards. `Recover` and `Resume` move the existing journey forward; they do
not create a second journey.

## Economy and calendar

The regional market escrows goods at the origin and settles an order only at
the destination. Seeds and crops use the character inventory; timber, stone,
and bandages use location stock. Saltmere is abundant in stone and bandages;
Whisperwood is abundant in timber and iron salvage; The Hearth is abundant in
wheat and seeds. Scarcity, route risk, infrastructure, and settlement
condition alter the visible price index. Open orders, failed fulfilment, and
stock notes are exposed as telemetry. A failed order is marked `Failed` rather
than silently destroying the escrow record; support can reconcile it.

The real-time calendar is locked before leases or crop promises depend on it:
one game day is 80 real minutes. The 14-day season and four-season year (56
game days) remain development fixtures pending pacing validation. Phase 5 names
the seasons thaw, greenrise, harvest, and
deepwinter. Seasons change opportunity and route pressure but do not close
essential services. Phase 6 owns the long-session calendar compatibility check.

## Event lifecycle

The seeded `river-thaw` fixture follows signal, escalation, intervention,
resolution, and aftermath. It crosses all three locations and records effects
on travel risk, farming supply, market prices, and household confidence. The
server accepts only an exact option from the event's visible intervention list;
arbitrary client text cannot manufacture a regional choice or effect. Every
cause, intervention, and outcome is recorded through the existing chronicle
cursor, so a later player can search what the region remembers.

## Law boundary

Tarrowyn does not select PvP for this release. `GET /v1/law` is explicit:
`pvp_enabled` and `theft_enabled` are false, while claims, trades, travel, and
recovery are protected. There is no accidental mixture of player ownership
and theft rules. Character defeat remains a bounded recovery state, and support
repair is audited. Any future PvP proposal must replace this decision record
and add consent, evidence, protected spaces, consequences, reporting, and
recovery fixtures before exposing ownership mechanics.
