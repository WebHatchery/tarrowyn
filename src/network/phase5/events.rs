use super::*;

impl Phase5Client {
    pub(super) fn queue_event(&mut self, request_id: String) {
        if self.event_command_pending() {
            return;
        }
        let event = self.active_event();
        let request = match event {
            None => RegionalEventRequest {
                request_id,
                action: RegionalEventAction::Seed,
                event_id: None,
                intervention: None,
            },
            Some(event)
                if matches!(
                    event.stage,
                    tarrowyn_protocol::RegionalEventStage::Signal
                        | tarrowyn_protocol::RegionalEventStage::Escalation
                ) =>
            {
                RegionalEventRequest {
                    request_id,
                    action: RegionalEventAction::Intervene,
                    event_id: Some(event.event_id.clone()),
                    intervention: event.intervention_options.first().cloned(),
                }
            }
            Some(event) if event.stage == tarrowyn_protocol::RegionalEventStage::Intervention => {
                RegionalEventRequest {
                    request_id,
                    action: RegionalEventAction::Resolve,
                    event_id: Some(event.event_id.clone()),
                    intervention: None,
                }
            }
            Some(_) => return,
        };
        super::super::queue::try_push(&mut self.commands, Phase5Command::Event(request));
    }

    pub(crate) fn queue_event_intervention(
        &mut self,
        request_id: String,
        intervention: String,
    ) -> bool {
        if self.event_command_pending() {
            return false;
        }
        let Some(event) = self.active_event().filter(|event| {
            matches!(
                event.stage,
                tarrowyn_protocol::RegionalEventStage::Signal
                    | tarrowyn_protocol::RegionalEventStage::Escalation
            )
        }) else {
            return false;
        };
        let event_id = event.event_id.clone();
        if !event
            .intervention_options
            .iter()
            .any(|option| option == &intervention)
        {
            return false;
        }
        super::super::queue::try_push(
            &mut self.commands,
            Phase5Command::Event(RegionalEventRequest {
                request_id,
                action: RegionalEventAction::Intervene,
                event_id: Some(event_id),
                intervention: Some(intervention),
            }),
        )
    }

    pub(crate) fn event_choices(&self) -> &[String] {
        self.active_event()
            .filter(|event| {
                matches!(
                    event.stage,
                    tarrowyn_protocol::RegionalEventStage::Signal
                        | tarrowyn_protocol::RegionalEventStage::Escalation
                )
            })
            .map(|event| event.intervention_options.as_slice())
            .unwrap_or(&[])
    }

    fn active_event(&self) -> Option<&tarrowyn_protocol::RegionalEvent> {
        self.events.as_ref().and_then(|events| {
            events.events.iter().rev().find(|event| {
                !matches!(
                    event.stage,
                    tarrowyn_protocol::RegionalEventStage::Aftermath
                )
            })
        })
    }
}

pub(super) fn merge_regional_events(
    current: &mut Option<RegionalEventsResponse>,
    incoming: RegionalEventsResponse,
) {
    let Some(current) = current else {
        let mut incoming = incoming;
        incoming.events.sort_by_key(|event| event.cursor);
        let excess = incoming
            .events
            .len()
            .saturating_sub(MAX_CACHED_REGIONAL_EVENTS);
        if excess > 0 {
            incoming.events.drain(..excess);
        }
        *current = Some(incoming);
        return;
    };
    let RegionalEventsResponse {
        events,
        cursor: incoming_cursor,
    } = incoming;
    let existing_cursor = current.cursor;
    for event in events {
        if event.cursor <= existing_cursor {
            continue;
        }
        if let Some(existing) = current
            .events
            .iter_mut()
            .find(|existing| existing.event_id == event.event_id)
        {
            if event.cursor > existing.cursor {
                *existing = event;
            }
        } else {
            current.events.push(event);
        }
    }
    current.cursor = current.cursor.max(incoming_cursor).max(
        current
            .events
            .iter()
            .map(|event| event.cursor)
            .max()
            .unwrap_or(0),
    );
    current.events.sort_by_key(|event| event.cursor);
    let excess = current
        .events
        .len()
        .saturating_sub(MAX_CACHED_REGIONAL_EVENTS);
    if excess > 0 {
        current.events.drain(..excess);
    }
}
