use crate::{
    app::selection_contains,
    color::Color,
    color_scheme::ColorScheme,
    context_menu::ContextMenu,
    squid::{Squid, SquidLimbRef, SquidRef},
};
use nalgebra_glm as glm;
use slotmap::SlotMap;

pub struct Selection {
    pub squid_id: SquidRef,
    pub limb_id: Option<SquidLimbRef>,
}

impl Selection {
    pub fn new(squid_id: SquidRef, limb_id: Option<SquidLimbRef>) -> Self {
        Self {
            squid_id: squid_id,
            limb_id: limb_id,
        }
    }
}

pub struct NewSelectionInfo {
    pub color: Option<Color>,
}

// A world that objects (aka squids) live in
pub struct Ocean {
    pub squids: SlotMap<SquidRef, Box<dyn Squid>>,
}

impl Ocean {
    pub fn new() -> Self {
        Self { squids: SlotMap::with_key() }
    }

    // Tries to find a squid/squid-limb underneath a point to select
    pub fn try_select(&mut self, underneath: &glm::Vec2, camera: &glm::Vec2, existing_selections: &Vec<Selection>) -> TrySelectResult {
        for (self_reference, value) in self.get_squids_newest_mut() {
            if let Some(result) = value.try_select(underneath, camera, self_reference) {
                if !selection_contains(existing_selections, self_reference) {
                    // Found selection to append-to/replace existing ones
                    return TrySelectResult::New(result);
                } else {
                    // Not new selection found, but preserve existing selection(s)
                    return TrySelectResult::Preserve;
                }
            }
        }

        // No new selection found, and don't preserve existing selection(s)
        TrySelectResult::Discard
    }

    pub fn get_squids_newest<'a>(&'a self) -> Vec<(SquidRef, &'a Box<dyn Squid>)> {
        let mut squids: Vec<(SquidRef, &Box<dyn Squid>)> = self.squids.iter().collect();
        squids.sort_by(|a, b| b.cmp(a));
        squids
    }

    pub fn get_squids_newest_mut<'a>(&'a mut self) -> Vec<(SquidRef, &'a mut Box<dyn Squid>)> {
        let mut squids: Vec<(SquidRef, &mut Box<dyn Squid>)> = self.squids.iter_mut().collect();
        squids.sort_by(|a, b| b.cmp(a));
        squids
    }

    #[allow(dead_code)]
    pub fn get_squids_oldest<'a>(&'a self) -> Vec<(SquidRef, &'a Box<dyn Squid>)> {
        let mut squids: Vec<(SquidRef, &Box<dyn Squid>)> = self.squids.iter().collect();
        squids.sort_by(|a, b| a.cmp(b));
        squids
    }

    #[allow(dead_code)]
    pub fn get_squids_oldest_mut<'a>(&'a mut self) -> Vec<(SquidRef, &'a mut Box<dyn Squid>)> {
        let mut squids: Vec<(SquidRef, &mut Box<dyn Squid>)> = self.squids.iter_mut().collect();
        squids.sort_by(|a, b| a.cmp(b));
        squids
    }

    // Tries to get a context menu for a squid underneath a point
    pub fn try_context_menu(&self, underneath: &glm::Vec2, camera: &glm::Vec2, color_scheme: &ColorScheme) -> Option<ContextMenu> {
        for (self_reference, value) in self.get_squids_newest() {
            if let Some(new_context_menu) = value.try_context_menu(underneath, camera, self_reference, color_scheme) {
                return Some(new_context_menu);
            }
        }

        None
    }
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
