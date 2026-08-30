use super::Phase4Client;

impl Phase4Client {
    pub(crate) fn discard_stale_knockout_combat(&mut self) {
        if self
            .combat
            .as_ref()
            .is_some_and(|combat| combat.status == tarrowyn_protocol::LocalCombatStatus::KnockedOut)
        {
            self.combat = None;
            self.pending_combat = None;
        }
    }

    pub(crate) fn recover_cursor_boundary(&mut self) {
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
        self.projection_cursor = 0;
        self.regional.reset_event_cursor();
    }
}
