use crate::models::PlateStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tool {
    AskUserQuestion,
    ExitPlanMode,
    Other,
}

impl Tool {
    pub fn from_name(name: &str) -> Self {
        match name {
            "AskUserQuestion" => Self::AskUserQuestion,
            "ExitPlanMode" => Self::ExitPlanMode,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    SessionStart,
    PromptSubmit,
    ToolStart(Tool),
    ToolCall,
    Stop { has_error: bool },
    HealthCheckRecovery,
}

impl Event {
    pub fn from_hook(event_type: &str, tool_name: Option<&str>, error: Option<&str>) -> Self {
        match event_type {
            "session_start" => Self::SessionStart,
            "prompt_submit" => Self::PromptSubmit,
            "tool_start" => Self::ToolStart(Tool::from_name(tool_name.unwrap_or(""))),
            "tool_call" => Self::ToolCall,
            "stop" => Self::Stop {
                has_error: error.is_some(),
            },
            _ => Self::ToolCall,
        }
    }
}

impl PlateStatus {
    pub fn transition(self, event: &Event) -> PlateStatus {
        match (self, event) {
            (_, Event::SessionStart) => PlateStatus::Running,
            (_, Event::PromptSubmit) => PlateStatus::Running,

            (_, Event::ToolStart(Tool::AskUserQuestion)) => PlateStatus::AwaitingInput,
            (_, Event::ToolStart(Tool::ExitPlanMode)) => PlateStatus::AwaitingApproval,
            (_, Event::ToolStart(Tool::Other)) => PlateStatus::Running,

            (_, Event::ToolCall) => PlateStatus::Running,

            (_, Event::Stop { has_error: true }) => PlateStatus::Error,
            (_, Event::Stop { has_error: false }) => PlateStatus::Idle,

            (PlateStatus::AwaitingInput, Event::HealthCheckRecovery) => PlateStatus::Idle,
            (PlateStatus::AwaitingApproval, Event::HealthCheckRecovery) => PlateStatus::Idle,
            (PlateStatus::Error, Event::HealthCheckRecovery) => PlateStatus::Idle,
            (state, Event::HealthCheckRecovery) => state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_start_transitions_to_running() {
        assert_eq!(
            PlateStatus::Starting.transition(&Event::SessionStart),
            PlateStatus::Running
        );
    }

    #[test]
    fn tool_start_ask_user_question_transitions_to_awaiting_input() {
        assert_eq!(
            PlateStatus::Running.transition(&Event::ToolStart(Tool::AskUserQuestion)),
            PlateStatus::AwaitingInput
        );
    }

    #[test]
    fn tool_start_exit_plan_mode_transitions_to_awaiting_approval() {
        assert_eq!(
            PlateStatus::Running.transition(&Event::ToolStart(Tool::ExitPlanMode)),
            PlateStatus::AwaitingApproval
        );
    }

    #[test]
    fn tool_call_transitions_to_running() {
        assert_eq!(
            PlateStatus::AwaitingInput.transition(&Event::ToolCall),
            PlateStatus::Running
        );
        assert_eq!(
            PlateStatus::AwaitingApproval.transition(&Event::ToolCall),
            PlateStatus::Running
        );
    }

    #[test]
    fn stop_with_error_transitions_to_error() {
        assert_eq!(
            PlateStatus::Running.transition(&Event::Stop { has_error: true }),
            PlateStatus::Error
        );
    }

    #[test]
    fn stop_without_error_transitions_to_idle() {
        assert_eq!(
            PlateStatus::Running.transition(&Event::Stop { has_error: false }),
            PlateStatus::Idle
        );
    }

    #[test]
    fn health_check_recovery_from_attention_states() {
        assert_eq!(
            PlateStatus::AwaitingInput.transition(&Event::HealthCheckRecovery),
            PlateStatus::Idle
        );
        assert_eq!(
            PlateStatus::AwaitingApproval.transition(&Event::HealthCheckRecovery),
            PlateStatus::Idle
        );
        assert_eq!(
            PlateStatus::Error.transition(&Event::HealthCheckRecovery),
            PlateStatus::Idle
        );
    }

    #[test]
    fn health_check_recovery_no_op_for_other_states() {
        assert_eq!(
            PlateStatus::Running.transition(&Event::HealthCheckRecovery),
            PlateStatus::Running
        );
        assert_eq!(
            PlateStatus::Idle.transition(&Event::HealthCheckRecovery),
            PlateStatus::Idle
        );
    }

    #[test]
    fn prompt_submit_reactivates_idle() {
        assert_eq!(
            PlateStatus::Idle.transition(&Event::PromptSubmit),
            PlateStatus::Running
        );
    }

    #[test]
    fn event_from_hook_parses_correctly() {
        assert_eq!(
            Event::from_hook("session_start", None, None),
            Event::SessionStart
        );
        assert_eq!(
            Event::from_hook("tool_start", Some("AskUserQuestion"), None),
            Event::ToolStart(Tool::AskUserQuestion)
        );
        assert_eq!(
            Event::from_hook("stop", None, Some("error message")),
            Event::Stop { has_error: true }
        );
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    fn arb_plate_status() -> impl Strategy<Value = PlateStatus> {
        prop_oneof![
            Just(PlateStatus::Starting),
            Just(PlateStatus::Running),
            Just(PlateStatus::Idle),
            Just(PlateStatus::AwaitingInput),
            Just(PlateStatus::AwaitingApproval),
            Just(PlateStatus::Error),
            Just(PlateStatus::Closed),
        ]
    }

    fn arb_event() -> impl Strategy<Value = Event> {
        prop_oneof![
            Just(Event::SessionStart),
            Just(Event::PromptSubmit),
            Just(Event::ToolStart(Tool::AskUserQuestion)),
            Just(Event::ToolStart(Tool::ExitPlanMode)),
            Just(Event::ToolStart(Tool::Other)),
            Just(Event::ToolCall),
            Just(Event::Stop { has_error: false }),
            Just(Event::Stop { has_error: true }),
            Just(Event::HealthCheckRecovery),
        ]
    }

    proptest! {
        #[test]
        fn transition_is_deterministic(
            state in arb_plate_status(),
            event in arb_event()
        ) {
            let result1 = state.transition(&event);
            let result2 = state.transition(&event);
            prop_assert_eq!(result1, result2);
        }

        #[test]
        fn transition_produces_valid_state(
            state in arb_plate_status(),
            event in arb_event()
        ) {
            let result = state.transition(&event);
            let _ = result.as_str();
        }

        #[test]
        fn attention_states_recover_on_health_check(state in arb_plate_status()) {
            if state.needs_attention() && state != PlateStatus::Idle {
                let recovered = state.transition(&Event::HealthCheckRecovery);
                prop_assert_eq!(recovered, PlateStatus::Idle);
            }
        }

        #[test]
        fn event_sequence_maintains_valid_state(
            initial in arb_plate_status(),
            events in prop::collection::vec(arb_event(), 0..50)
        ) {
            let mut state = initial;
            for event in events {
                state = state.transition(&event);
                let _ = state.as_str();
            }
        }

        #[test]
        fn prompt_submit_always_activates(state in arb_plate_status()) {
            let result = state.transition(&Event::PromptSubmit);
            prop_assert_eq!(result, PlateStatus::Running);
        }

        #[test]
        fn session_start_always_activates(state in arb_plate_status()) {
            let result = state.transition(&Event::SessionStart);
            prop_assert_eq!(result, PlateStatus::Running);
        }
    }
}
