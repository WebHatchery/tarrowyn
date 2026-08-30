use super::*;

impl Phase4Client {
    pub(crate) fn claim_command_pending(&self) -> bool {
        self.in_flight_command
            .as_ref()
            .is_some_and(|command| matches!(command, Phase4Command::Claim(_)))
            || self
                .commands
                .iter()
                .any(|command| matches!(command, Phase4Command::Claim(_)))
    }

    fn owned_claim(&self) -> Option<&tarrowyn_protocol::ClaimRecord> {
        let own_account_id = self.own_account_id.as_deref()?;
        self.claims
            .as_ref()?
            .claims
            .iter()
            .rev()
            .find(|claim| claim.owner_account_id.as_deref() == Some(own_account_id))
    }

    pub(super) fn can_abandon_claim(&self) -> bool {
        self.owned_claim().is_some_and(|claim| {
            matches!(
                claim.status,
                tarrowyn_protocol::ClaimLifecycleStatus::Requested
                    | tarrowyn_protocol::ClaimLifecycleStatus::Active
                    | tarrowyn_protocol::ClaimLifecycleStatus::Renewed
                    | tarrowyn_protocol::ClaimLifecycleStatus::Transferred
                    | tarrowyn_protocol::ClaimLifecycleStatus::Inherited
            )
        })
    }

    pub(super) fn can_transfer_claim(&self) -> bool {
        self.owned_claim().is_some_and(|claim| {
            matches!(
                claim.status,
                tarrowyn_protocol::ClaimLifecycleStatus::Active
                    | tarrowyn_protocol::ClaimLifecycleStatus::Renewed
                    | tarrowyn_protocol::ClaimLifecycleStatus::Transferred
                    | tarrowyn_protocol::ClaimLifecycleStatus::Inherited
            )
        })
    }

    pub(super) fn queue_claim_action(
        &mut self,
        request_id: String,
        action: ClaimLifecycleAction,
        target_account_id: Option<String>,
    ) -> bool {
        if self.claim_command_pending() {
            return false;
        }
        let claim_id = self.owned_claim().map(|claim| claim.claim_id.clone());
        let eligible = match action {
            ClaimLifecycleAction::Abandon => self.can_abandon_claim(),
            ClaimLifecycleAction::Transfer | ClaimLifecycleAction::Inherit => {
                self.can_transfer_claim() && target_account_id.is_some()
            }
            _ => false,
        };
        if !eligible {
            return false;
        }
        super::super::queue::try_push(
            &mut self.commands,
            Phase4Command::Claim(ClaimLifecycleRequest {
                request_id,
                action,
                claim_id,
                target_account_id,
            }),
        )
    }

    pub(super) fn queue_claim(&mut self, request_id: String) {
        let own_account_id = self.own_account_id.as_deref();
        let is_steward = own_account_id.is_some_and(|account_id| {
            self.governance.as_ref().is_some_and(|governance| {
                governance.offices.iter().any(|office| {
                    office.kind == tarrowyn_protocol::OfficeKind::Steward
                        && office.holder_account_id.as_deref() == Some(account_id)
                })
            })
        });
        let claim = self.claims.as_ref().and_then(|claims| {
            is_steward
                .then(|| {
                    claims.claims.iter().rev().find(|claim| {
                        claim.status == tarrowyn_protocol::ClaimLifecycleStatus::Requested
                    })
                })
                .flatten()
                .or_else(|| {
                    claims.claims.iter().rev().find(|claim| {
                        own_account_id.is_some_and(|account_id| {
                            claim.owner_account_id.as_deref() == Some(account_id)
                        })
                    })
                })
        });
        let (action, claim_id) = match claim {
            None => (ClaimLifecycleAction::Request, None),
            Some(claim) => {
                let action = match claim.status {
                    tarrowyn_protocol::ClaimLifecycleStatus::Requested => {
                        ClaimLifecycleAction::Approve
                    }
                    tarrowyn_protocol::ClaimLifecycleStatus::Active
                    | tarrowyn_protocol::ClaimLifecycleStatus::Renewed
                    | tarrowyn_protocol::ClaimLifecycleStatus::Transferred
                    | tarrowyn_protocol::ClaimLifecycleStatus::Inherited => {
                        ClaimLifecycleAction::Renew
                    }
                    tarrowyn_protocol::ClaimLifecycleStatus::Abandoned
                    | tarrowyn_protocol::ClaimLifecycleStatus::Expired => {
                        ClaimLifecycleAction::Reclaim
                    }
                    tarrowyn_protocol::ClaimLifecycleStatus::Reclaimed => {
                        ClaimLifecycleAction::Request
                    }
                };
                (action, Some(claim.claim_id.clone()))
            }
        };
        super::super::queue::try_push(
            &mut self.commands,
            Phase4Command::Claim(ClaimLifecycleRequest {
                request_id,
                action,
                claim_id,
                target_account_id: None,
            }),
        );
    }
}
