use super::{CraftingView, Phase4Client};
use crate::network::{ConnectionState, OnlineClient};
use macroquad_toolkit::grid::TilePos;
use tarrowyn_protocol::{ClaimLifecycleAction, LocalCombatState};

impl Phase4Client {
    pub(crate) fn sync_regional_player_location(&mut self, position: TilePos) {
        self.regional.sync_player_location(position);
    }

    pub(super) fn queue_region_intervention(
        &mut self,
        request_id: String,
        intervention: String,
    ) -> bool {
        self.regional
            .queue_event_intervention(request_id, intervention)
    }

    pub(super) fn regional_event_choices(&self) -> &[String] {
        self.regional.event_choices()
    }
}

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

    pub(crate) fn queue_region_intervention(&mut self, intervention: String) {
        if self.state == ConnectionState::Online {
            let request_id = self.next_request_id("event");
            if !self
                .phase4
                .queue_region_intervention(request_id, intervention)
            {
                self.status_message =
                    "That event choice is no longer available; refresh the regional ledger."
                        .to_owned();
            }
        }
    }

    pub(crate) fn phase4_summary(&self) -> String {
        self.phase4.summary()
    }

    pub(crate) fn knowledge_cycle_label(&self, has_target: bool) -> &'static str {
        self.phase4.knowledge_cycle_label(has_target)
    }

    pub(crate) fn queue_knowledge_cycle(&mut self, target_account_id: Option<String>) {
        if self.state == ConnectionState::Online {
            let request_id = self.next_request_id("knowledge");
            if !self.phase4.queue_knowledge(request_id, target_account_id) {
                self.status_message =
                    "The knowledge archive is busy; wait for its ledger or queue space.".to_owned();
            }
        }
    }

    pub(crate) fn queue_report(&mut self) {
        if self.state != ConnectionState::Online {
            return;
        }
        let own_account_id = self
            .account
            .as_ref()
            .map(|account| account.account_id.as_str());
        let evidence = self
            .projection
            .chat
            .iter()
            .rev()
            .find(|message| Some(message.account_id.as_str()) != own_account_id)
            .map(|message| {
                (
                    Some(message.account_id.clone()),
                    Some(message.message_id),
                    Some(message.display_name.clone()),
                )
            })
            .or_else(|| {
                self.projection
                    .players
                    .iter()
                    .find(|player| {
                        Some(player.account_id.as_str()) != own_account_id
                            && !player.stale(self.projection.server_tick)
                    })
                    .map(|player| {
                        (
                            Some(player.account_id.clone()),
                            None,
                            Some(player.display_name.clone()),
                        )
                    })
            })
            .unwrap_or((None, None, None));
        let request_id = self.next_request_id("report");
        if self
            .phase4
            .queue_region_report(request_id, evidence.0, evidence.1)
        {
            self.status_message = match evidence.2 {
                Some(name) => {
                    format!("Report prepared for {name}; waiting for the moderation ledger…")
                }
                None => "General report prepared; waiting for the moderation ledger…".to_owned(),
            };
        } else {
            self.status_message =
                "The moderation queue is busy; wait before submitting another report.".to_owned();
        }
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

    pub(crate) fn queue_skill_practice(&mut self, skill_id: String) {
        if self.state != ConnectionState::Online {
            return;
        }
        let request_id = self.next_request_id("practice");
        if !self.phase4.queue_skill_practice_for(request_id, skill_id) {
            self.status_message =
                "That discipline is no longer open for practice; refresh the skill ledger."
                    .to_owned();
        }
    }

    pub(crate) fn phase5_event_choices(&self) -> &[String] {
        self.phase4.regional_event_choices()
    }

    pub(crate) fn phase4_skills(&self) -> &[tarrowyn_protocol::SkillView] {
        self.phase4
            .skills
            .as_ref()
            .map(|skills| skills.skills.as_slice())
            .unwrap_or(&[])
    }
}
