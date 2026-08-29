use super::*;

impl WorldRepository {
    pub fn chat(
        &self,
        token: &str,
        request: ChatRequest,
    ) -> Result<ApiResponse<ChatResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        expire_sessions(&mut state, &self.config);
        let key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        if let Some(previous) = state
            .identities
            .get(&key)
            .and_then(|identity| identity.chat_results.get(&request.request_id))
            .cloned()
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: previous,
            });
        }
        let text = request.text.trim().to_owned();
        let channel = if request.channel.trim().is_empty() {
            "settlement"
        } else {
            request.channel.trim()
        };
        let mut response = ChatResponse {
            request_id: request.request_id.clone(),
            accepted: false,
            message: None,
            reason: None,
        };
        let fast = state
            .sessions
            .get(token)
            .and_then(|session| session.last_chat_tick)
            .is_some_and(|last| last == state.tick);
        if text.is_empty() {
            response.reason = Some("A message cannot be empty.".to_owned());
        } else if text.chars().count() > self.config.chat_max_length.min(MAX_CHAT_MESSAGE_LENGTH) {
            response.reason = Some(format!(
                "Messages are limited to {} characters.",
                self.config.chat_max_length
            ));
        } else if text.chars().any(char::is_control) {
            response.reason = Some("Messages cannot contain control characters.".to_owned());
        } else if channel.len() > 24 {
            response.reason = Some("That channel name is too long.".to_owned());
        } else if channel.chars().any(char::is_control) {
            response.reason = Some("Channel names cannot contain control characters.".to_owned());
        } else if fast {
            response.reason = Some("Give the channel a moment before sending again.".to_owned());
        } else {
            let identity = state.identities.get(&key).expect("identity exists");
            let mut message = ChatMessage {
                message_id: state.next_message,
                account_id: identity.account_id.clone(),
                display_name: identity.display_name.clone(),
                channel: channel.to_owned(),
                text,
                cursor: 0,
            };
            state.next_message += 1;
            let cursor = push_event(&mut state, WorldEvent::Chat(message.clone()));
            message.cursor = cursor;
            if let Some(EventRecord {
                event: WorldEvent::Chat(stored),
                ..
            }) = state.events.back_mut()
            {
                *stored = message.clone();
            }
            state.chat_history.push_back(message.clone());
            trim_back(&mut state.chat_history, MAX_CHAT_HISTORY);
            response.accepted = true;
            response.message = Some(message);
            state
                .sessions
                .get_mut(token)
                .expect("session exists")
                .last_chat_tick = Some(state.tick);
        }
        state
            .identities
            .get_mut(&key)
            .expect("identity exists")
            .chat_results
            .insert(request.request_id.clone(), response.clone());
        let actor = state
            .identities
            .get(&key)
            .expect("identity exists")
            .account_id
            .clone();
        phase6::audit_command(
            &mut state,
            &actor,
            "chat.send",
            channel,
            response.accepted,
            "Chat metadata was recorded without retaining message text in the audit stream.",
        );
        record_command_outcome(&mut state, response.accepted);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }
}
