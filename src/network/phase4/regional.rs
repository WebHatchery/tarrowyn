use super::Phase4Client;
use tarrowyn_protocol::{AuthSession, GuestSessionResponse, SkillStatus};

impl Phase4Client {
    pub(crate) fn queue_region_cycle(&mut self, id: &str) -> bool {
        self.regional.queue_cycle(id)
    }

    pub(crate) fn auth_refresh_pending(&self) -> bool {
        self.regional.auth_refresh_pending()
    }

    pub(crate) fn account_link_available(&self) -> bool {
        self.regional.account_link_available()
    }

    pub(crate) fn queue_region_report(
        &mut self,
        request_id: String,
        target_account_id: Option<String>,
        message_id: Option<u64>,
    ) -> bool {
        self.regional
            .queue_report(request_id, target_account_id, message_id)
    }

    pub(crate) fn region_summary(&self) -> String {
        self.regional.summary()
    }

    pub(crate) fn regional_travel_control(&self) -> (&'static str, bool, bool) {
        self.regional.travel_control()
    }

    pub(crate) fn has_open_market_order(&self) -> bool {
        self.regional.has_open_market_order()
    }

    pub(crate) fn regional_inspection(&self) -> String {
        self.regional.inspection()
    }

    pub(crate) fn regional_season(&self) -> Option<&str> {
        self.regional.season()
    }

    pub(crate) fn regional_region(&self) -> Option<&tarrowyn_protocol::RegionSnapshot> {
        self.regional.region_snapshot()
    }

    pub(crate) fn take_linked_account(
        &mut self,
        client_key: Option<&str>,
    ) -> Option<GuestSessionResponse> {
        self.regional.take_linked_account(client_key)
    }

    pub(crate) fn take_logged_out(&mut self) -> bool {
        self.regional.take_logged_out()
    }

    pub(crate) fn deletion_armed(&self) -> bool {
        self.regional.deletion_armed()
    }

    pub(crate) fn take_refreshed_session(&mut self) -> Option<AuthSession> {
        self.regional.take_refreshed_session()
    }

    pub(crate) fn storm_magic_unlocked(&self) -> bool {
        self.skills.as_ref().is_some_and(|skills| {
            skills.skills.iter().any(|skill| {
                skill.skill_id == "storm-magic" && skill.status == SkillStatus::Discovered
            })
        })
    }
}
