use super::*;

impl Phase5Client {
    pub(super) fn queue_route_action(&mut self, request_id: String, action: RouteAction) -> bool {
        let route_id = self.region.as_ref().and_then(|region| {
            region
                .routes
                .iter()
                .find(|route| {
                    route.origin_location_id == region.player_location_id
                        && route_action_selectable(route.status, action)
                })
                .or_else(|| {
                    region.routes.iter().find(|route| {
                        route.destination_location_id == region.player_location_id
                            && route_action_selectable(route.status, action)
                    })
                })
                .map(|route| route.route_id.clone())
        });
        let Some(route_id) = route_id else {
            return false;
        };
        super::super::queue::try_push(
            &mut self.commands,
            Phase5Command::Route(RouteRequest {
                request_id,
                route_id,
                action,
            }),
        )
    }
}

fn route_action_selectable(status: tarrowyn_protocol::RouteStatus, action: RouteAction) -> bool {
    match action {
        RouteAction::Repair => status != tarrowyn_protocol::RouteStatus::Operational,
        RouteAction::Escort => true,
        RouteAction::Improve => status != tarrowyn_protocol::RouteStatus::Closed,
    }
}
