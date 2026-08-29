use super::*;

impl Phase4Client {
    pub(super) fn queue_combat(&mut self, request_id: String) {
        let action = match self.combat.as_ref().map(|combat| combat.status) {
            Some(tarrowyn_protocol::LocalCombatStatus::Engaged) => LocalCombatAction::Strike,
            Some(tarrowyn_protocol::LocalCombatStatus::KnockedOut) => LocalCombatAction::Retreat,
            _ => LocalCombatAction::Prepare,
        };
        super::super::queue::try_push(
            &mut self.commands,
            Phase4Command::Combat(LocalCombatRequest {
                request_id,
                action,
                weapon: next_combat_weapon(self.combat.as_ref().map(|combat| combat.weapon)),
            }),
        );
    }

    pub(super) fn queue_combat_action(&mut self, request_id: String, action: LocalCombatAction) {
        super::super::queue::try_push(
            &mut self.commands,
            Phase4Command::Combat(LocalCombatRequest {
                request_id,
                action,
                weapon: self
                    .combat
                    .as_ref()
                    .map(|combat| combat.weapon)
                    .unwrap_or(WeaponKind::IronSword),
            }),
        );
    }
}

pub(super) fn advance_crafting(challenge: &mut Option<CraftingChallenge>, dt: f32) {
    let Some(challenge) = challenge else {
        return;
    };
    challenge.progress += dt.max(0.0) * 0.45 * challenge.direction;
    if challenge.progress >= 1.0 {
        challenge.progress = 1.0;
        challenge.direction = -1.0;
    } else if challenge.progress <= 0.0 {
        challenge.progress = 0.0;
        challenge.direction = 1.0;
    }
}

pub(super) fn next_combat_weapon(current: Option<WeaponKind>) -> WeaponKind {
    match current {
        None => WeaponKind::IronSword,
        Some(WeaponKind::IronSword) => WeaponKind::Spear,
        Some(WeaponKind::Spear) => WeaponKind::Axe,
        Some(WeaponKind::Axe) => WeaponKind::Bow,
        Some(WeaponKind::Bow) => WeaponKind::Shield,
        Some(WeaponKind::Shield) => WeaponKind::ImprovisedClub,
        Some(WeaponKind::ImprovisedClub) => WeaponKind::IronSword,
    }
}
