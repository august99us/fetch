use std::time::Instant;

#[derive(Debug, Clone, Default)]
pub struct State {
    pub last_click: Option<Instant>,
    pub click_count: u8,
    pub status: Status,
}

// Needs to be public because the runtime needs to view it for styling
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Status {
    #[default] Default,
    Hovered,
    Selected,
}