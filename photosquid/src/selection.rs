use crate::{
    color::Color,
    squid::{SquidLimbRef, SquidRef},
};

#[derive(Copy, Clone)]
pub struct Selection {
    pub squid_id: SquidRef,
    pub limb_id: Option<SquidLimbRef>,
}

impl Selection {
    pub fn new(squid_id: SquidRef, limb_id: Option<SquidLimbRef>) -> Self {
        Self { squid_id, limb_id }
    }
}

pub struct NewSelectionInfo {
    pub color: Option<Color>,
}

pub struct NewSelection {
    pub selection: Selection,
    pub info: NewSelectionInfo,
}

pub enum TrySelectResult {
    New(NewSelection),
    Preserve,
    Discard,
}

pub fn selection_contains(selections: &[Selection], squid_reference: SquidRef) -> bool {
    for selection in selections.iter() {
        if selection.squid_id == squid_reference {
            return true;
        }
    }
    false
}
