use super::*;

impl OnlineClient {
    pub(crate) fn queue_phase5(&mut self, id: &str) {
        let ready = if is_session_action(id) {
            self.session_mutations_ready()
        } else {
            self.mutations_ready()
        };
        if ready && !self.phase4.queue_region_cycle(id) {
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

    pub(crate) fn travel_pending(&self) -> bool {
        self.phase4.travel_command_pending()
    }

    pub(crate) fn identity_pending(&self) -> bool {
        self.phase4.identity_command_pending()
    }

    pub(crate) fn report_pending(&self) -> bool {
        self.phase4.report_command_pending()
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

fn is_session_action(id: &str) -> bool {
    matches!(id, "account" | "logout" | "delete-account")
}
