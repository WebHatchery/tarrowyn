use super::Phase4Client;

impl Phase4Client {
    pub(in crate::network) fn clear(&mut self) {
        self.pending_governance = None;
        self.pending_claims = None;
        self.pending_professions = None;
        self.pending_knowledge = None;
        self.pending_skills = None;
        self.pending_households = None;
        self.pending_combat = None;
        self.pending_command = None;
        self.in_flight_command = None;
        self.commands.clear();
        self.command_retry_timer = 0.0;
        self.command_retry_count = 0;
        self.governance = None;
        self.claims = None;
        self.professions = None;
        self.knowledge = None;
        self.skills = None;
        self.households = None;
        self.combat = None;
        self.crafting = None;
        self.own_account_id = None;
        self.projection_cursor = 0;
        self.regional.clear();
    }
}
