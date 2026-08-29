use super::*;

impl Phase5Client {
    pub(super) fn queue_travel(&mut self, request_id: String) {
        let Some(region) = self.region.as_ref() else {
            return;
        };
        if let Some(travel) = region.travel.as_ref() {
            let action = if travel.status == TravelStatus::Interrupted {
                TravelAction::Recover
            } else {
                TravelAction::Interrupt
            };
            self.queue_travel_action(request_id, action);
            return;
        }
        let route = region
            .routes
            .iter()
            .find(|route| {
                route.origin_location_id == region.player_location_id
                    && route.status != tarrowyn_protocol::RouteStatus::Closed
            })
            .or_else(|| {
                region.routes.iter().find(|route| {
                    route.destination_location_id == region.player_location_id
                        && route.status != tarrowyn_protocol::RouteStatus::Closed
                })
            });
        let Some(route) = route else {
            return;
        };
        self.commands
            .push_back(Phase5Command::Travel(TravelRequest {
                request_id,
                action: TravelAction::Start,
                route_id: Some(route.route_id.clone()),
                travel_id: None,
            }));
    }

    pub(super) fn queue_travel_action(&mut self, request_id: String, action: TravelAction) {
        let travel_id = self
            .region
            .as_ref()
            .and_then(|region| region.travel.as_ref())
            .map(|travel| travel.travel_id.clone());
        self.commands
            .push_back(Phase5Command::Travel(TravelRequest {
                request_id,
                action,
                route_id: None,
                travel_id,
            }));
    }
}
