use super::*;

impl Phase5Client {
    pub(crate) fn market_command_pending(&self) -> bool {
        self.in_flight_command
            .as_ref()
            .is_some_and(|command| matches!(command, Phase5Command::Market(_)))
            || self
                .commands
                .iter()
                .any(|command| matches!(command, Phase5Command::Market(_)))
    }

    pub(crate) fn event_command_pending(&self) -> bool {
        self.in_flight_command
            .as_ref()
            .is_some_and(|command| matches!(command, Phase5Command::Event(_)))
            || self
                .commands
                .iter()
                .any(|command| matches!(command, Phase5Command::Event(_)))
    }

    pub(crate) fn route_command_pending(&self) -> bool {
        self.in_flight_command
            .as_ref()
            .is_some_and(|command| matches!(command, Phase5Command::Route(_)))
            || self
                .commands
                .iter()
                .any(|command| matches!(command, Phase5Command::Route(_)))
    }

    pub(crate) fn travel_command_pending(&self) -> bool {
        self.in_flight_command
            .as_ref()
            .is_some_and(|command| matches!(command, Phase5Command::Travel(_)))
            || self
                .commands
                .iter()
                .any(|command| matches!(command, Phase5Command::Travel(_)))
    }

    pub(crate) fn identity_command_pending(&self) -> bool {
        self.in_flight_command.as_ref().is_some_and(|command| {
            matches!(
                command,
                Phase5Command::Link(_) | Phase5Command::Revoke(_) | Phase5Command::Delete(_)
            )
        }) || self.commands.iter().any(|command| {
            matches!(
                command,
                Phase5Command::Link(_) | Phase5Command::Revoke(_) | Phase5Command::Delete(_)
            )
        })
    }
}
