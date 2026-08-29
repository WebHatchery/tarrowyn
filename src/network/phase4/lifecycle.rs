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
        self.commands.clear();
        self.governance = None;
        self.claims = None;
        self.professions = None;
        self.knowledge = None;
        self.skills = None;
        self.combat = None;
        self.crafting = None;
        self.own_account_id = None;
        self.regional.clear();
    }
}
