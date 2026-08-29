# Phase 5 regional playthrough report

The deterministic regional fixture was exercised with one guest character.
The character travelled from The Hearth to Whisperwood Watch, manually
interrupted the journey, recovered it, and arrived once. The same character
then created a two-seed order for Saltmere, travelled over the watch trail,
and settled the order at the destination. The market response changed from
`Open` to `Fulfilled`, the character received the useful good and carrier
payment, and the order remained idempotent when its request was repeated.

The settlements expose different conditions, vacancies, demands, resources,
public works, and free-plot projections. The regional event fixture moved from signal to
escalation, accepted a ferry-marker intervention, and resolved with recorded
effects on route safety, prices, and settlement confidence. The Maren household
changed from considering to travelling or arrived as the bounded household
interval advanced. `/v1/law` returned the protected no-PvP boundary.

The settlement projection also moved Saltmere's safety, industry, governance,
and infrastructure signals as its unattended regional support faded, while the
route and public-work targets remained visible for recovery.

The settlement regression also leaves the Hearth supported by an active
character while Saltmere loses unattended activity locally. After the session
expires, the Hearth activity signal declines and Saltmere exposes a strained
condition through its low activity and weak industry, without removing access
to the regional projection.

The server tests cover travel interruption/recovery, market settlement,
regional event cursor recovery, household history, OIDC-style guest linking,
session refresh, and revocation. The browser-facing touch pass uses the
visible Travel, Recover, Repair, Market, Inspect, Event, Account, Logout,
Report, Delete, and Reconnect controls; an owned open market order also shows
the visible Cancel recovery control, and linked-account deletion requires two
visible taps before returning to Reconnect. No keyboard command is required.
