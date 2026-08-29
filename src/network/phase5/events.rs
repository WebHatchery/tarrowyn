use super::*;

impl Phase5Client {
    pub(super) fn queue_event(&mut self, request_id: String) {
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
