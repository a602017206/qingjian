use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, PartialEq)]
pub enum ExitDecision {
    Allow,
    Save,
    Cancel,
}
pub fn exit_decision(
    exiting: bool,
    approved: &BTreeSet<String>,
    current: &BTreeSet<String>,
) -> ExitDecision {
    if current.is_empty() || exiting && current.is_subset(approved) {
        ExitDecision::Allow
    } else if exiting {
        ExitDecision::Cancel
    } else {
        ExitDecision::Save
    }
}

#[derive(Default)]
pub struct ActionQueue {
    next_id: u64,
    requests: BTreeMap<u64, (String, BTreeSet<String>, BTreeSet<String>)>,
}
impl ActionQueue {
    pub fn push(&mut self, action: &str, windows: Vec<String>) -> u64 {
        let windows: BTreeSet<_> = windows.into_iter().collect();
        if action == "quit" {
            if let Some((&id, (_, waiting, participants))) = self
                .requests
                .iter_mut()
                .find(|(_, (name, _, _))| name == "quit")
            {
                for window in windows {
                    if participants.insert(window.clone()) {
                        waiting.insert(window);
                    }
                }
                return id;
            }
        } else if let Some((&id, _)) = self
            .requests
            .iter()
            .find(|(_, (name, waiting, _))| name == action && waiting == &windows)
        {
            return id;
        }
        self.next_id += 1;
        self.requests
            .insert(self.next_id, (action.into(), windows.clone(), windows));
        self.next_id
    }
    pub fn participants(&self, id: u64) -> BTreeSet<String> {
        self.requests
            .get(&id)
            .map(|(_, _, windows)| windows.clone())
            .unwrap_or_default()
    }
    pub fn window_closed(&mut self, window: &str) -> bool {
        let quitting = self
            .requests
            .values()
            .any(|(action, waiting, _)| action == "quit" && waiting.contains(window));
        self.requests.retain(|_, (action, waiting, _)| {
            !(waiting.contains(window) || quitting && action == "quit")
        });
        quitting
    }
    pub fn action_for(&self, window: &str, id: u64) -> Option<String> {
        self.requests
            .get(&id)
            .filter(|(_, waiting, _)| waiting.contains(window))
            .map(|(action, _, _)| action.clone())
    }
    pub fn pending(&self, window: &str) -> Vec<(u64, String)> {
        self.requests
            .iter()
            .filter(|(_, (_, waiting, _))| waiting.contains(window))
            .map(|(&id, (action, _, _))| (id, action.clone()))
            .collect()
    }
    pub fn complete(&mut self, window: &str, id: u64, success: bool) -> Option<String> {
        let (action, waiting, _) = self.requests.get_mut(&id)?;
        if !waiting.remove(window) {
            return None;
        }
        if !success {
            self.requests.remove(&id);
            return None;
        }
        if !waiting.is_empty() {
            return None;
        }
        let action = action.clone();
        self.requests.remove(&id);
        Some(action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn system_exit_allows_an_empty_application_and_checks_approved_windows() {
        let empty = BTreeSet::new();
        let main = BTreeSet::from(["main".to_string()]);
        assert_eq!(exit_decision(false, &empty, &empty), ExitDecision::Allow);
        assert_eq!(exit_decision(false, &empty, &main), ExitDecision::Save);
        assert_eq!(exit_decision(true, &main, &main), ExitDecision::Allow);
        assert_eq!(exit_decision(true, &empty, &main), ExitDecision::Cancel);
    }
    #[test]
    fn close_waits_for_save_and_rejects_stale_ack() {
        let mut queue = ActionQueue::default();
        let id = queue.push("hide", vec!["main".into()]);
        assert_eq!(queue.pending("main")[0].0, id);
        assert_eq!(queue.complete("note-other", id, true), None);
        assert_eq!(queue.complete("main", id + 1, true), None);
        assert_eq!(queue.pending("main").len(), 1);
        assert_eq!(queue.complete("main", id, false), None);
        assert!(queue.pending("main").is_empty());
        let next = queue.push("hide", vec!["main".into()]);
        assert_eq!(queue.complete("main", next, true), Some("hide".into()));
    }
    #[test]
    fn quit_waits_for_every_window_and_cancels_on_failure() {
        let mut queue = ActionQueue::default();
        let id = queue.push("quit", vec!["main".into(), "note-one".into()]);
        assert_eq!(queue.complete("main", id, true), None);
        assert_eq!(
            queue.push("quit", vec!["main".into(), "note-one".into()]),
            id,
            "重复退出不能创建第二次事务"
        );
        assert_eq!(queue.pending("note-one").len(), 1);
        assert_eq!(queue.complete("note-one", id, false), None);
        let next = queue.push("quit", vec!["main".into(), "note-one".into()]);
        assert_eq!(queue.complete("note-one", next, true), None);
        assert_eq!(queue.complete("main", next, true), Some("quit".into()));
    }
    #[test]
    fn later_window_joins_the_existing_quit_transaction() {
        let mut queue = ActionQueue::default();
        let id = queue.push("quit", vec!["main".into()]);
        assert_eq!(
            queue.push("quit", vec!["main".into(), "note-new".into()]),
            id
        );
        assert_eq!(queue.pending("note-new").len(), 1);
        assert_eq!(queue.complete("main", id, true), None);
        assert_eq!(queue.complete("note-new", id, true), Some("quit".into()));
    }
    #[test]
    fn destroyed_window_cancels_pending_quit() {
        let mut queue = ActionQueue::default();
        queue.push("quit", vec!["main".into(), "note-one".into()]);
        assert!(queue.window_closed("note-one"));
        assert!(queue.pending("main").is_empty());
    }
    #[test]
    fn early_actions_remain_ordered_and_duplicate_requests_are_coalesced() {
        let mut queue = ActionQueue::default();
        let first = queue.push("search", vec!["main".into()]);
        assert_eq!(queue.push("search", vec!["main".into()]), first);
        queue.push("new", vec!["main".into()]);
        assert_eq!(
            queue
                .pending("main")
                .iter()
                .map(|(_, a)| a.as_str())
                .collect::<Vec<_>>(),
            vec!["search", "new"]
        );
        assert!(queue.pending("note-one").is_empty());
    }
}
