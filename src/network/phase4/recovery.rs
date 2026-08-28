use super::Phase4Client;

impl Phase4Client {
    pub(crate) fn recover_regional_cursor(&mut self) {
        self.regional.reset_event_cursor();
    }
}
