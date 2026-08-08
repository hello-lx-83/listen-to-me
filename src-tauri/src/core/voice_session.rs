use std::time::Duration;

use crate::core::models::VoiceSessionState;

#[derive(Debug, Default)]
pub struct VoiceSession {
    state: VoiceSessionState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HoldAction {
    Begin,
    Finish,
    Cancel,
}

#[derive(Debug)]
pub struct HoldController {
    threshold: Duration,
    pressed_at: Option<Duration>,
    active: bool,
}

impl HoldController {
    pub fn new(threshold: Duration) -> Self {
        Self {
            threshold,
            pressed_at: None,
            active: false,
        }
    }

    pub fn press(&mut self, now: Duration) {
        if self.pressed_at.is_none() {
            self.pressed_at = Some(now);
        }
    }

    pub fn poll(&mut self, now: Duration) -> Option<HoldAction> {
        let pressed_at = self.pressed_at?;
        if !self.active && now.saturating_sub(pressed_at) >= self.threshold {
            self.active = true;
            return Some(HoldAction::Begin);
        }

        None
    }

    pub fn release(&mut self) -> Option<HoldAction> {
        self.pressed_at?;
        self.pressed_at = None;

        if self.active {
            self.active = false;
            Some(HoldAction::Finish)
        } else {
            None
        }
    }

    pub fn cancel(&mut self) -> Option<HoldAction> {
        let had_session = self.pressed_at.take().is_some() || self.active;
        self.active = false;
        had_session.then_some(HoldAction::Cancel)
    }

    pub fn time_until_activation(&self, now: Duration) -> Option<Duration> {
        let pressed_at = self.pressed_at?;
        if self.active {
            return None;
        }

        Some(
            self.threshold
                .saturating_sub(now.saturating_sub(pressed_at)),
        )
    }

    pub fn is_pending(&self) -> bool {
        self.pressed_at.is_some()
    }
}

impl VoiceSession {
    pub fn state(&self) -> VoiceSessionState {
        self.state
    }

    pub fn transition_to(&mut self, next: VoiceSessionState) {
        self.state = next;
    }

    pub fn reset(&mut self) {
        self.state = VoiceSessionState::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_can_return_to_idle() {
        let mut session = VoiceSession::default();
        session.transition_to(VoiceSessionState::Recording);
        session.reset();

        assert!(matches!(session.state(), VoiceSessionState::Idle));
    }

    #[test]
    fn short_tap_does_not_begin() {
        let mut controller = HoldController::new(Duration::from_millis(220));
        controller.press(Duration::ZERO);

        assert_eq!(controller.poll(Duration::from_millis(100)), None);
        assert_eq!(controller.release(), None);
    }

    #[test]
    fn long_hold_begins_and_release_finishes() {
        let mut controller = HoldController::new(Duration::from_millis(220));
        controller.press(Duration::ZERO);

        assert_eq!(
            controller.poll(Duration::from_millis(220)),
            Some(HoldAction::Begin)
        );
        assert_eq!(controller.release(), Some(HoldAction::Finish));
    }

    #[test]
    fn repeated_press_does_not_reset_threshold() {
        let mut controller = HoldController::new(Duration::from_millis(220));
        controller.press(Duration::ZERO);
        controller.press(Duration::from_millis(180));

        assert_eq!(
            controller.poll(Duration::from_millis(220)),
            Some(HoldAction::Begin)
        );
    }

    #[test]
    fn cancel_resets_pending_session() {
        let mut controller = HoldController::new(Duration::from_millis(220));
        controller.press(Duration::ZERO);

        assert_eq!(controller.cancel(), Some(HoldAction::Cancel));
        assert!(!controller.is_pending());
    }
}
