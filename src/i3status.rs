use serde::Serialize;

#[derive(Serialize, Default)]
pub struct CustomI3Status {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub state: I3State,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_text: Option<String>,
}

impl CustomI3Status {
    pub fn new(state: I3State, text: String) -> Self {
        Self {
            state,
            text,
            ..Default::default()
        }
    }
}

#[derive(Serialize, Default)]
#[serde(rename_all = "PascalCase")]
pub enum I3State {
    #[default]
    Idle,
    Info,
    Good,
    Warning,
    Critical,
}
