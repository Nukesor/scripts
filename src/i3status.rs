use serde::Serialize;

#[derive(Serialize, Default)]
pub struct CustomBarStatus {
    pub text: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub tooltip: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub class: String,
}

impl CustomBarStatus {
    pub fn new(text: String) -> Self {
        Self {
            text,
            ..Default::default()
        }
    }
}
