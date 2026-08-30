use super::*;
use crate::network::WorldProjection;

impl Phase4Client {
    #[cfg(test)]
    pub fn update(
        &mut self,
        dt: f32,
        api: &mut HttpClient,
        projection: &mut WorldProjection,
        online: bool,
        another_mutation_pending: bool,
        notices: &mut Vec<NetworkNotice>,
    ) {
        self.update_with_mode(
            dt,
            api,
            projection,
            MutationContext {
                online,
                another_mutation_pending,
                session_only: false,
            },
            notices,
        );
    }

    pub(crate) fn update_with_mode(
        &mut self,
        dt: f32,
        api: &mut HttpClient,
        projection: &mut WorldProjection,
        context: MutationContext,
        notices: &mut Vec<NetworkNotice>,
    ) {
        if !context.online {
            return;
        }
        self.command_retry_timer = (self.command_retry_timer - dt.max(0.0)).max(0.0);
        if !context.session_only {
            advance_crafting(&mut self.crafting, dt);
        }
        poll_projection(
            &mut self.pending_governance,
            dt,
            |response| {
                if projection
                    .accept_response_version(response.meta.server_tick, response.meta.cursor)
                    && accept_projection_cursor(&mut self.projection_cursor, response.meta.cursor)
                {
                    self.governance = Some(response.data.governance);
                }
            },
            notices,
            "town hall",
        );
        poll_projection(
            &mut self.pending_claims,
            dt,
            |response| {
                if projection
                    .accept_response_version(response.meta.server_tick, response.meta.cursor)
                    && accept_projection_cursor(&mut self.projection_cursor, response.meta.cursor)
                {
                    self.claims = Some(response.data);
                }
            },
            notices,
            "land registry",
        );
        poll_projection(
            &mut self.pending_professions,
            dt,
            |response| {
                if projection
                    .accept_response_version(response.meta.server_tick, response.meta.cursor)
                    && accept_projection_cursor(&mut self.projection_cursor, response.meta.cursor)
                {
                    self.professions = Some(response.data);
                }
            },
            notices,
            "profession ledger",
        );
        poll_projection(
            &mut self.pending_knowledge,
            dt,
            |response| {
                if projection
                    .accept_response_version(response.meta.server_tick, response.meta.cursor)
                    && accept_projection_cursor(&mut self.projection_cursor, response.meta.cursor)
                {
                    self.knowledge = Some(response.data);
                }
            },
            notices,
            "knowledge archive",
        );
        poll_projection(
            &mut self.pending_skills,
            dt,
            |response| {
                if projection
                    .accept_response_version(response.meta.server_tick, response.meta.cursor)
                    && accept_projection_cursor(&mut self.projection_cursor, response.meta.cursor)
                {
                    self.skills = Some(response.data);
                }
            },
            notices,
            "skill ledger",
        );
        poll_projection(
            &mut self.pending_households,
            dt,
            |response| {
                if projection
                    .accept_response_version(response.meta.server_tick, response.meta.cursor)
                    && accept_projection_cursor(&mut self.projection_cursor, response.meta.cursor)
                {
                    self.households = Some(response.data);
                }
            },
            notices,
            "household ledger",
        );
        poll_projection(
            &mut self.pending_combat,
            dt,
            |response| {
                if projection
                    .accept_response_version(response.meta.server_tick, response.meta.cursor)
                    && accept_projection_cursor(&mut self.projection_cursor, response.meta.cursor)
                {
                    self.combat = Some(response.data);
                }
            },
            notices,
            "local combat ledger",
        );
        if projection
            .player
            .as_ref()
            .is_some_and(|player| player.knocked_out)
        {
            self.combat = None;
            self.pending_combat = None;
        }
        if let Some(result) = self
            .pending_command
            .as_mut()
            .and_then(|pending| pending.poll_timed(dt, REQUEST_TIMEOUT_SECONDS))
        {
            self.pending_command = None;
            let in_flight_command = self.in_flight_command.take();
            match result {
                Ok(response) => {
                    self.command_retry_timer = 0.0;
                    self.command_retry_count = 0;
                    let projection_current = projection
                        .accept_response_version(response.meta.server_tick, response.meta.cursor);
                    self.apply_command(
                        response.data,
                        response.meta.cursor,
                        projection_current,
                        in_flight_command.as_ref(),
                        notices,
                    );
                }
                Err(error)
                    if is_transient_transport_error(&error)
                        && self.command_retry_count < MAX_COMMAND_RETRIES
                        && in_flight_command.is_some() =>
                {
                    self.commands
                        .push_front(in_flight_command.expect("command exists"));
                    self.command_retry_count += 1;
                    self.command_retry_timer = COMMAND_RETRY_DELAY_SECONDS;
                    notices.push(NetworkNotice::Warning(format!(
                        "The Phase 4 action could not be confirmed; retrying the same request ({}/{}). {}",
                        self.command_retry_count,
                        MAX_COMMAND_RETRIES,
                        short_error(&error)
                    )));
                }
                Err(error) => {
                    self.command_retry_count = 0;
                    notices.push(NetworkNotice::Warning(format!(
                        "The Phase 4 action could not be confirmed: {}",
                        short_error(&error)
                    )));
                }
            }
        }
        self.regional.update_with_mode(
            dt,
            api,
            projection,
            MutationContext {
                online: context.online,
                another_mutation_pending: self.pending_command.is_some()
                    || context.another_mutation_pending,
                session_only: context.session_only,
            },
            notices,
        );
        self.dispatch(
            api,
            context.another_mutation_pending || self.regional.command_pending(),
            projection
                .player
                .as_ref()
                .is_some_and(|player| player.knocked_out),
            context.session_only,
        );
    }

    fn dispatch(
        &mut self,
        api: &mut HttpClient,
        another_mutation_pending: bool,
        player_knocked_out: bool,
        session_only: bool,
    ) {
        if !session_only && !self.regional.dispatch_blocked() {
            if self.pending_governance.is_none() {
                self.pending_governance = Some(api.get("/v1/settlement/governance"));
            }
            if self.pending_claims.is_none() {
                self.pending_claims = Some(api.get("/v1/claims"));
            }
            if self.pending_professions.is_none() {
                self.pending_professions = Some(api.get("/v1/professions"));
            }
            if self.pending_knowledge.is_none() {
                self.pending_knowledge = Some(api.get("/v1/knowledge"));
            }
            if self.pending_skills.is_none() {
                self.pending_skills = Some(api.get("/v1/skills"));
            }
            if self.pending_households.is_none() {
                self.pending_households = Some(api.get("/v1/households"));
            }
            if !player_knocked_out && self.pending_combat.is_none() {
                self.pending_combat = Some(api.get("/v1/combat/local"));
            }
        }
        if self.pending_command.is_none()
            && !another_mutation_pending
            && !session_only
            && !self.regional.dispatch_blocked()
            && self.command_retry_timer <= 0.0
        {
            if let Some(command) = self.commands.pop_front() {
                self.pending_command = Some(match &command {
                    Phase4Command::Governance(request) => {
                        api.post_json("/v1/settlement/governance", &request)
                    }
                    Phase4Command::Claim(request) => {
                        api.post_json("/v1/claims/lifecycle", &request)
                    }
                    Phase4Command::Profession(request) => {
                        api.post_json("/v1/professions/orders", &request)
                    }
                    Phase4Command::Knowledge(request) => api.post_json("/v1/knowledge", &request),
                    Phase4Command::Combat(request) => api.post_json("/v1/combat/local", &request),
                    Phase4Command::Skill(request) => api.post_json("/v1/skills", &request),
                });
                self.in_flight_command = Some(command);
            }
        }
    }
}
