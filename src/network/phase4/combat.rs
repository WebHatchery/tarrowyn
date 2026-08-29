use super::*;

impl Phase4Client {
    pub(super) fn queue_combat(&mut self, request_id: String) {
        let action = match self.combat.as_ref().map(|combat| combat.status) {
            Some(tarrowyn_protocol::LocalCombatStatus::Engaged) => LocalCombatAction::Strike,
            Some(tarrowyn_protocol::LocalCombatStatus::KnockedOut) => LocalCombatAction::Retreat,
            _ => LocalCombatAction::Prepare,
        };
        self.commands
            .push_back(Phase4Command::Combat(LocalCombatRequest {
                request_id,
                action,
                weapon: next_combat_weapon(self.combat.as_ref().map(|combat| combat.weapon)),
            }));
    }

    pub(super) fn queue_combat_action(&mut self, request_id: String, action: LocalCombatAction) {
        self.commands
            .push_back(Phase4Command::Combat(LocalCombatRequest {
                request_id,
                action,
                weapon: self
                    .combat
                    .as_ref()
                    .map(|combat| combat.weapon)
                    .unwrap_or(WeaponKind::IronSword),
            }));
    }
}
