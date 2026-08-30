use super::*;

impl OnlineClient {
    pub(crate) fn queue_phase5(&mut self, id: &str) {
        if self.state == super::ConnectionState::Online && !self.phase4.queue_region_cycle(id) {
            self.status_message =
                "That regional action is not ready; wait for its projection or queue to clear."
                    .to_owned();
        }
    }

    pub(crate) fn phase5_summary(&self) -> String {
        self.phase4.region_summary()
    }

    pub(crate) fn account_summary(&self) -> String {
        self.phase4.account_summary()
    }

    pub(crate) fn phase5_travel_control(&self) -> (&'static str, bool, bool) {
        self.phase4.regional_travel_control()
    }

    pub(crate) fn has_open_market_order(&self) -> bool {
        self.phase4.has_open_market_order()
    }

    pub(crate) fn market_pending(&self) -> bool {
        self.phase4.market_command_pending()
    }

    pub(crate) fn event_pending(&self) -> bool {
        self.phase4.event_command_pending()
    }

    pub(crate) fn route_pending(&self) -> bool {
        self.phase4.route_command_pending()
    }

    pub(crate) fn phase5_inspection(&self) -> String {
        self.phase4.regional_inspection()
    }

    pub(crate) fn phase5_season(&self) -> Option<&str> {
        self.phase4.regional_season()
    }

    pub(crate) fn phase5_region(&self) -> Option<&RegionSnapshot> {
        self.phase4.regional_region()
    }

    pub(crate) fn account_deletion_armed(&self) -> bool {
        self.phase4.deletion_armed()
    }

    pub(crate) fn account_link_available(&self) -> bool {
        self.phase4.account_link_available()
    }

    pub(crate) fn account_deletion_available(&self) -> bool {
        self.phase4.account_deletion_available()
    }
}
