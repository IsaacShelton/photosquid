use crate::{
    app::selection_contains,
    color::Color,
    color_scheme::ColorScheme,
    context_menu::ContextMenu,
    layer::Layer,
    squid::{self, Squid, SquidLimbRef, SquidRef},
};
use nalgebra_glm as glm;
use slotmap::SlotMap;

#[derive(Copy, Clone)]
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
#[derive(Clone)]
pub struct Ocean {
    current_layer: usize,
    layers: Vec<Layer>,
    squids: SlotMap<SquidRef, Box<dyn Squid>>,
}

impl Ocean {
    pub fn new() -> Self {
        Self {
            current_layer: 0,
            layers: vec![Default::default()],
            squids: SlotMap::with_key(),
        }
    }

    pub fn insert(&mut self, value: Box<dyn Squid>) -> SquidRef {
        let reference = self.squids.insert(value);

        self.force_valid_layer();

        self.layers[self.current_layer].add(reference);
        reference
    }

    fn force_valid_layer(&mut self) {
        if self.layers.len() == 0 {
            self.layers.push(Default::default());
        }

        if self.current_layer >= self.layers.len() {
            self.current_layer = self.layers.len() - 1;
        }
    }

    pub fn remove(&mut self, reference: SquidRef) {
        self.squids.remove(reference);
    }

    pub fn get(&self, reference: SquidRef) -> Option<&Box<dyn Squid>> {
        self.squids.get(reference)
    }

    pub fn get_mut(&mut self, reference: SquidRef) -> Option<&mut Box<dyn Squid>> {
        self.squids.get_mut(reference)
    }

    // Tries to find a squid/squid-limb underneath a point to select
    pub fn try_select(&mut self, underneath: &glm::Vec2, camera: &glm::Vec2, existing_selections: &Vec<Selection>) -> TrySelectResult {
        let highest_squids: Vec<SquidRef> = self.get_squids_highest().collect();
        let world_mouse = underneath - camera;

        for self_reference in highest_squids {
            if let Some(squid) = self.get_mut(self_reference) {
                let already_selected = selection_contains(existing_selections, self_reference);

                // If the squid is already selected, and we are trying to select over on-top of one
                // of its handles, then return to just preserve the existing selection
                if already_selected {
                    for region in squid.get_opaque_handles().iter() {
                        if glm::distance(region, &world_mouse) < 2.0 * squid::HANDLE_RADIUS {
                            return TrySelectResult::Preserve;
                        }
                    }
                }

                if let Some(result) = squid.try_select(underneath, camera, self_reference) {
                    if !already_selected {
                        // Found selection to append-to/replace existing ones
                        return TrySelectResult::New(result);
                    } else {
                        // Not new selection found, but preserve existing selection(s)
                        return TrySelectResult::Preserve;
                    }
                }
            }
        }

        // No new selection found, and don't preserve existing selection(s)
        TrySelectResult::Discard
    }

    pub fn get_squids_unordered<'a>(&'a self) -> impl Iterator<Item = SquidRef> + '_ {
        self.get_squids_highest()
    }

    pub fn get_squids_highest<'a>(&'a self) -> impl Iterator<Item = SquidRef> + '_ {
        self.layers.iter().flat_map(|layer| layer.get_highest())
    }

    pub fn get_squids_lowest<'a>(&'a self) -> impl Iterator<Item = SquidRef> + '_ {
        self.layers.iter().flat_map(|layer| layer.get_lowest())
    }

    // Tries to get a context menu for a squid underneath a point
    pub fn try_context_menu(&self, underneath: &glm::Vec2, camera: &glm::Vec2, color_scheme: &ColorScheme) -> Option<ContextMenu> {
        for self_reference in self.get_squids_highest() {
            if let Some(value) = self.get(self_reference) {
                if let Some(new_context_menu) = value.try_context_menu(underneath, camera, self_reference, color_scheme) {
                    return Some(new_context_menu);
                }
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
