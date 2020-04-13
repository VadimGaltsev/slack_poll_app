use crate::ui_poll_view::{DialogViewVariant, SingleVariant};

#[derive(Default, Debug)]
pub struct PollData {
    pub ts: String,
    pub poll_channel: String,
    pub poll_variants: Vec<SingleVariant>,
    pub dialog_variants: Vec<DialogViewVariant>,
}
