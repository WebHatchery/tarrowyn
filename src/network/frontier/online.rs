use super::super::OnlineClient;
use tarrowyn_protocol::{
    ClaimAction, CombatAction, ContractAction, ExpeditionAction, ExpeditionRequest, ExpeditionRole,
    RecoveryChoice, WeaponKind,
};

impl OnlineClient {
    pub fn queue_contract_cycle(&mut self) {
        let action = match self.frontier.contracts.first() {
            Some(contract) if contract.status == tarrowyn_protocol::ContractStatus::Cooldown => {
                self.status_message =
                    "The frontier contract is cooling down; return after its visible availability tick."
                        .to_owned();
                return;
            }
            Some(contract)
                if contract.status == tarrowyn_protocol::ContractStatus::Accepted
                    && contract.progress < 3 =>
            {
                ContractAction::Progress
            }
            Some(contract) if contract.status == tarrowyn_protocol::ContractStatus::Accepted => {
                ContractAction::Report
            }
            _ => ContractAction::Accept,
        };
        self.queue_contract(action);
    }

    pub fn queue_claim_cycle(&mut self) {
        let action = if self.projection.claim.as_ref().is_some_and(|claim| {
            self.account.as_ref().is_some_and(|account| {
                claim.owner_account_id == account.account_id
                    && claim.status == tarrowyn_protocol::ClaimStatus::Active
            })
        }) {
            ClaimAction::Renew
        } else {
            ClaimAction::Request
        };
        self.queue_claim(action);
    }

    pub fn queue_expedition_cycle(&mut self) {
        let (action, role) = match self.projection.expedition.as_ref() {
            None => (ExpeditionAction::Announce, Some(ExpeditionRole::Scout)),
            Some(expedition)
                if expedition.status == tarrowyn_protocol::ExpeditionStatus::Launched =>
            {
                (ExpeditionAction::Resolve, None)
            }
            Some(expedition)
                if matches!(
                    expedition.status,
                    tarrowyn_protocol::ExpeditionStatus::Succeeded
                        | tarrowyn_protocol::ExpeditionStatus::Retreated
                ) =>
            {
                (ExpeditionAction::Announce, Some(ExpeditionRole::Scout))
            }
            Some(expedition) => {
                let requirements = self.projection.expedition_requirements;
                let own = self
                    .account
                    .as_ref()
                    .map(|account| account.account_id.as_str());
                if !expedition
                    .members
                    .iter()
                    .any(|member| Some(member.account_id.as_str()) == own)
                {
                    let role = [
                        ExpeditionRole::Scout,
                        ExpeditionRole::Farmer,
                        ExpeditionRole::Builder,
                    ]
                    .into_iter()
                    .find(|role| expedition.members.iter().all(|member| member.role != *role))
                    .unwrap_or(ExpeditionRole::Builder);
                    (ExpeditionAction::Join, Some(role))
                } else if expedition.food < requirements.food
                    || expedition.tools < requirements.tools
                    || expedition.materials < requirements.materials
                    || expedition.safety < requirements.safety
                {
                    (ExpeditionAction::Supply, None)
                } else {
                    (ExpeditionAction::Launch, None)
                }
            }
        };
        self.queue_expedition(action, role);
    }

    pub fn queue_contract(&mut self, action: ContractAction) {
        if self.mutations_ready() {
            let request_id = self.next_request_id("contract");
            if !self.frontier.queue_contract(request_id, action) {
                self.status_message =
                    "That frontier action is not ready; wait for its ledger or queue to clear."
                        .to_owned();
            }
        }
    }

    pub fn queue_combat(&mut self, action: CombatAction, weapon: WeaponKind) {
        if self.mutations_ready() {
            let request_id = self.next_request_id("combat");
            if !self.frontier.queue_combat(request_id, action, weapon) {
                self.status_message =
                    "That frontier action is not ready; wait for its ledger or queue to clear."
                        .to_owned();
            }
        }
    }

    pub fn queue_recovery(&mut self, choice: RecoveryChoice) {
        if self.mutations_ready() {
            let request_id = self.next_request_id("recovery");
            if !self.frontier.queue_recovery(request_id, choice) {
                self.status_message =
                    "That frontier action is not ready; wait for its ledger or queue to clear."
                        .to_owned();
            }
        }
    }

    pub(crate) fn recovery_pending(&self) -> bool {
        self.frontier.recovery_command_pending()
    }

    pub(crate) fn contract_pending(&self) -> bool {
        self.frontier.contract_command_pending()
    }

    pub(crate) fn expedition_pending(&self) -> bool {
        self.frontier.expedition_command_pending()
    }

    pub(crate) fn frontier_combat_pending(&self) -> bool {
        self.frontier.combat_command_pending()
    }

    pub(crate) fn frontier_claim_pending(&self) -> bool {
        self.frontier.claim_command_pending()
    }

    pub fn queue_claim(&mut self, action: ClaimAction) {
        if self.mutations_ready() {
            let request_id = self.next_request_id("claim");
            if !self.frontier.queue_claim(request_id, action) {
                self.status_message =
                    "That frontier action is not ready; wait for its ledger or queue to clear."
                        .to_owned();
            }
        }
    }

    pub fn queue_expedition(&mut self, action: ExpeditionAction, role: Option<ExpeditionRole>) {
        if self.mutations_ready() {
            let request_id = self.next_request_id("expedition");
            if !self.frontier.queue_expedition(ExpeditionRequest {
                request_id,
                action,
                expedition_id: Some("pioneer-1".to_owned()),
                role,
                food: if action == ExpeditionAction::Supply {
                    6
                } else {
                    0
                },
                tools: if action == ExpeditionAction::Supply {
                    3
                } else {
                    0
                },
                materials: if action == ExpeditionAction::Supply {
                    8
                } else {
                    0
                },
                safety: if action == ExpeditionAction::Supply {
                    3
                } else {
                    0
                },
                outpost_name: Some("Lantern Rest".to_owned()),
            }) {
                self.status_message =
                    "That frontier action is not ready; wait for its ledger or queue to clear."
                        .to_owned();
            }
        }
    }
}
