//! Safe-mode confirmation data.

use crate::action::Action;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ConfirmChoice {
    Cancel,
    Confirm,
}

impl ConfirmChoice {
    pub fn toggle(self) -> Self {
        match self {
            Self::Cancel => Self::Confirm,
            Self::Confirm => Self::Cancel,
        }
    }

    pub fn is_confirm(self) -> bool {
        matches!(self, Self::Confirm)
    }
}

#[derive(Clone, Debug)]
pub struct PendingCommand {
    pub description: String,
    pub actions: Vec<Action>,
    pub destructive: bool,
}

impl PendingCommand {
    pub fn preview_lines(&self, max_actions: usize) -> Vec<String> {
        let mut lines = vec![self.description.clone()];
        for action in self.actions.iter().take(max_actions) {
            lines.push(action.describe());
        }
        if self.actions.len() > max_actions {
            lines.push(format!(
                "… {} more action(s)",
                self.actions.len() - max_actions
            ));
        }
        if self.destructive {
            lines.push("WARNING: destructive command".to_string());
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choice_toggles() {
        assert_eq!(ConfirmChoice::Cancel.toggle(), ConfirmChoice::Confirm);
        assert_eq!(ConfirmChoice::Confirm.toggle(), ConfirmChoice::Cancel);
    }

    #[test]
    fn preview_lists_all_actions_and_a_destructive_warning() {
        let pending = PendingCommand {
            description: "test".to_string(),
            actions: vec![
                Action::Publish {
                    topic: "a".to_string(),
                    payload: "1".to_string(),
                    retain: false,
                },
                Action::Publish {
                    topic: "b".to_string(),
                    payload: "2".to_string(),
                    retain: false,
                },
            ],
            destructive: true,
        };
        let lines = pending.preview_lines(1);
        assert_eq!(lines[0], "test");
        assert!(lines[1].contains("MQTT a"));
        assert!(lines.iter().any(|line| line.contains("more action")));
        assert!(lines.iter().any(|line| line.contains("WARNING")));
    }
}
