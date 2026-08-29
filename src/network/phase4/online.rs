use super::CraftingView;
use crate::network::{ConnectionState, OnlineClient};
use tarrowyn_protocol::{ClaimLifecycleAction, LocalCombatState};

impl OnlineClient {
    pub(crate) fn queue_phase4(&mut self, id: &str) {
        if self.state == ConnectionState::Online {
            let request_id = self.next_request_id("phase4");
            if !self.phase4.queue_cycle(id, request_id) {
                self.status_message =
                    "That settlement action is not ready; wait for its ledger or queue to clear."
                        .to_owned();
            }
        }
    }

    pub(crate) fn phase4_summary(&self) -> String {
        self.phase4.summary()
    }

    pub(crate) fn can_abandon_claim(&self) -> bool {
        self.phase4.can_abandon_claim()
    }

    pub(crate) fn can_transfer_claim(&self) -> bool {
        self.phase4.can_transfer_claim()
    }

    pub(crate) fn queue_claim_action(
        &mut self,
        action: ClaimLifecycleAction,
        target_account_id: Option<String>,
    ) {
        if self.state == ConnectionState::Online {
            let request_id = self.next_request_id("claim");
            if !self
                .phase4
                .queue_claim_action(request_id, action, target_account_id)
            {
                self.status_message =
                    "That lease action is not ready; inspect the registry or wait for queue space."
                        .to_owned();
            }
        }
    }

    pub(crate) fn crafting_view(&self) -> Option<CraftingView> {
        self.phase4
            .crafting_view()
            .map(|(progress, target_start, target_end)| CraftingView {
                progress,
                target_start,
                target_end,
            })
    }

    pub(crate) fn combat_state(&self) -> Option<&LocalCombatState> {
        self.phase4.combat.as_ref()
    }

    pub(crate) fn storm_magic_unlocked(&self) -> bool {
        self.phase4.storm_magic_unlocked()
    }

    pub(crate) fn queue_crafting_timing(&mut self) {
        if self.state != ConnectionState::Online {
            return;
        }
        let request_id = self.next_request_id("craft");
        if self.phase4.submit_crafting(request_id) {
            self.status_message =
                "Crafting result sent; waiting for the workshop ledger…".to_owned();
        } else {
            self.status_message =
                "The workshop action is not ready; keep the timing challenge open and try again."
                    .to_owned();
        }
    }

    pub(crate) fn queue_skill_teach(&mut self, target_account_id: &str) -> bool {
        if self.state != ConnectionState::Online {
            return false;
        }
        let request_id = self.next_request_id("school");
        let queued = self
            .phase4
            .queue_school(request_id, target_account_id.to_owned());
        if !queued {
            self.status_message =
                "The school action is not ready; wait for a mastered discipline or queue space."
                    .to_owned();
        }
        queued
    }
}
