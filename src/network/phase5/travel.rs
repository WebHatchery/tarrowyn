use super::*;

impl Phase5Client {
    pub(super) fn travel_control_details(&self) -> (&'static str, bool, bool) {
        let Some(region) = self.region.as_ref() else {
            return ("Travel", false, false);
        };
        match region.travel.as_ref().map(|travel| travel.status) {
            Some(TravelStatus::Interrupted) => ("Travel", false, true),
            Some(TravelStatus::Travelling) => ("Interrupt", true, false),
            Some(TravelStatus::Recovering) => ("Recovering", false, false),
            Some(TravelStatus::Idle | TravelStatus::Arrived) | None => {
                ("Travel", has_travel_route(region), false)
            }
        }
    }

    pub(super) fn queue_travel(&mut self, request_id: String) {
        let Some(region) = self.region.as_ref() else {
            return;
        };
        match region.travel.as_ref().map(|travel| travel.status) {
            Some(TravelStatus::Interrupted) => {
                self.queue_travel_action(request_id, TravelAction::Recover);
                return;
            }
            Some(TravelStatus::Travelling) => {
                self.queue_travel_action(request_id, TravelAction::Interrupt);
                return;
            }
            Some(TravelStatus::Recovering) => return,
            Some(TravelStatus::Idle | TravelStatus::Arrived) | None => {}
        }
        let route = region
            .routes
            .iter()
            .find(|route| {
                route.origin_location_id == region.player_location_id && route_is_open(route)
            })
            .or_else(|| {
                region.routes.iter().find(|route| {
                    route.destination_location_id == region.player_location_id
                        && route_is_open(route)
                })
            });
        let Some(route) = route else {
            return;
        };
        super::super::queue::try_push(
            &mut self.commands,
            Phase5Command::Travel(TravelRequest {
                request_id,
                action: TravelAction::Start,
                route_id: Some(route.route_id.clone()),
                travel_id: None,
            }),
        );
    }

    pub(super) fn queue_travel_action(&mut self, request_id: String, action: TravelAction) {
        let travel_id = self
            .region
            .as_ref()
            .and_then(|region| region.travel.as_ref())
            .map(|travel| travel.travel_id.clone());
        super::super::queue::try_push(
            &mut self.commands,
            Phase5Command::Travel(TravelRequest {
                request_id,
                action,
                route_id: None,
                travel_id,
            }),
        );
    }
}

fn has_travel_route(region: &RegionSnapshot) -> bool {
    region
        .routes
        .iter()
        .any(|route| route_is_travelable(route, &region.player_location_id))
}

fn route_is_travelable(route: &tarrowyn_protocol::RouteRecord, location_id: &str) -> bool {
    (route.origin_location_id == location_id || route.destination_location_id == location_id)
        && route_is_open(route)
}

fn route_is_open(route: &tarrowyn_protocol::RouteRecord) -> bool {
    route.status != tarrowyn_protocol::RouteStatus::Closed
}
