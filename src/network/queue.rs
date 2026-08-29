use std::collections::VecDeque;

pub(super) const MAX_PENDING_COMMANDS: usize = 32;

pub(super) fn try_push<T>(queue: &mut VecDeque<T>, value: T) -> bool {
    if queue.len() >= MAX_PENDING_COMMANDS {
        return false;
    }
    queue.push_back(value);
    true
}
