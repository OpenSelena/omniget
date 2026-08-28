use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VoiceState {
    #[default]
    Idle,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VoiceEvent {
    Join,
    Connected,
    Disconnected { recoverable: bool },
    Reconnected,
    Leave,
    GiveUp,
}

#[derive(Debug, Default, Clone)]
pub struct VoiceStateMachine {
    state: VoiceState,
    retries: u8,
}

impl VoiceStateMachine {
    pub const MAX_RETRIES: u8 = 5;

    pub fn state(&self) -> VoiceState {
        self.state
    }

    pub fn apply(&mut self, ev: &VoiceEvent) -> VoiceState {
        use VoiceEvent as E;
        use VoiceState as S;
        self.state = match (self.state, ev) {
            (S::Idle, E::Join) => S::Connecting,
            (S::Connecting, E::Connected) => {
                self.retries = 0;
                S::Connected
            }
            (S::Connecting, E::Disconnected { recoverable: true })
            | (S::Connected, E::Disconnected { recoverable: true }) => {
                self.retries += 1;
                if self.retries > Self::MAX_RETRIES {
                    S::Failed
                } else {
                    S::Reconnecting
                }
            }
            (S::Connecting, E::Disconnected { recoverable: false })
            | (S::Connected, E::Disconnected { recoverable: false }) => S::Failed,
            (S::Reconnecting, E::Reconnected) | (S::Reconnecting, E::Connected) => {
                self.retries = 0;
                S::Connected
            }
            (S::Reconnecting, E::Disconnected { recoverable: true }) => {
                self.retries += 1;
                if self.retries > Self::MAX_RETRIES {
                    S::Failed
                } else {
                    S::Reconnecting
                }
            }
            (S::Reconnecting, E::Disconnected { recoverable: false })
            | (S::Reconnecting, E::GiveUp) => S::Failed,
            (_, E::Leave) => {
                self.retries = 0;
                S::Idle
            }
            (S::Failed, E::Join) => S::Connecting,
            (s, _) => s,
        };
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path() {
        let mut m = VoiceStateMachine::default();
        assert_eq!(m.apply(&VoiceEvent::Join), VoiceState::Connecting);
        assert_eq!(m.apply(&VoiceEvent::Connected), VoiceState::Connected);
        assert_eq!(m.apply(&VoiceEvent::Leave), VoiceState::Idle);
    }

    #[test]
    fn gives_up_after_max_retries() {
        let mut m = VoiceStateMachine::default();
        m.apply(&VoiceEvent::Join);
        m.apply(&VoiceEvent::Connected);
        for _ in 0..VoiceStateMachine::MAX_RETRIES {
            assert_eq!(
                m.apply(&VoiceEvent::Disconnected { recoverable: true }),
                VoiceState::Reconnecting
            );
        }
        assert_eq!(
            m.apply(&VoiceEvent::Disconnected { recoverable: true }),
            VoiceState::Failed
        );
        assert_eq!(m.apply(&VoiceEvent::Join), VoiceState::Connecting);
    }

    #[test]
    fn leave_always_returns_to_idle() {
        let mut m = VoiceStateMachine::default();
        m.apply(&VoiceEvent::Join);
        m.apply(&VoiceEvent::Disconnected { recoverable: true });
        assert_eq!(m.apply(&VoiceEvent::Leave), VoiceState::Idle);
    }
}
