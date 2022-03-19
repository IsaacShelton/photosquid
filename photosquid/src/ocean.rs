use crate::{
    annotations::UnsafeTemporary,
    camera::Camera,
    color_scheme::ColorScheme,
    context_menu::ContextMenu,
    layer::Layer,
    selection::{selection_contains, Selection, TrySelectResult},
    squid::{self, Squid, SquidRef},
};
use nalgebra_glm as glm;
use slotmap::SlotMap;

// A world that objects (aka squids) live in
// NOTE: Only use direct access if you know what you're doing
// Use #[allow(deprecated)] to silence warnings when using internal fields
#[derive(Clone)]
pub struct Ocean {
    #[deprecated]
    pub current_layer: usize,

    #[deprecated]
    pub layers: Vec<Layer>,

    #[deprecated]
    pub squids: SlotMap<SquidRef, Squid>,
}

// Using #[allow(deprecated)] to silence warnings about manually accessing internal
// fields of 'Ocean' struct
#[allow(deprecated)]
impl Default for Ocean {
    fn default() -> Self {
        Self {
            current_layer: 0,
            layers: vec![Default::default()],
            squids: SlotMap::with_key(),
        }
    }
}

// Using #[allow(deprecated)] to silence warnings about manually accessing internal
// fields of 'Ocean' struct
#[allow(deprecated)]
impl Ocean {
    pub fn insert(&mut self, value: Squid) -> SquidRef {
        let reference = self.squids.insert(value);

        self.force_valid_layer();

        self.layers[self.current_layer].add(reference);
        reference
    }

    fn force_valid_layer(&mut self) {
        if self.layers.is_empty() {
            self.layers.push(Default::default());
        }

        if self.current_layer >= self.layers.len() {
            self.current_layer = self.layers.len() - 1;
        }
    }

    pub fn remove(&mut self, reference: SquidRef) {
        for layer in &mut self.layers {
            layer.remove_mention(reference);
        }

        self.squids.remove(reference);
    }

    pub fn get(&self, reference: SquidRef) -> Option<UnsafeTemporary<&Squid>> {
        self.squids.get(reference)
    }

    pub fn get_mut(&mut self, reference: SquidRef) -> Option<UnsafeTemporary<&mut Squid>> {
        self.squids.get_mut(reference)
    }

    pub fn get_layers(&self) -> &[Layer] {
        &self.layers
    }

    // Tries to find a squid/squid-limb underneath a point to select
    pub fn try_select(&mut self, underneath: glm::Vec2, camera: &Camera, existing_selections: &[Selection]) -> TrySelectResult {
        let highest_squids: Vec<SquidRef> = self.get_squids_highest().collect();
        let world_mouse = camera.apply_reverse(&underneath);

        for self_reference in highest_squids {
            if let Some(squid) = self.get_mut(self_reference) {
                let already_selected = selection_contains(existing_selections, self_reference);

                // If the squid is already selected, and we are trying to select over on-top of one
                // of its handles, then return to just preserve the existing selection
                if already_selected {
                    for region in &squid.get_opaque_handles() {
                        if glm::distance(region, &world_mouse) < 2.0 * squid::HANDLE_RADIUS {
                            return TrySelectResult::Preserve;
                        }
                    }
                }

                if let Some(result) = squid.try_select(underneath, camera, self_reference) {
                    return if !already_selected {
                        // Found selection to append-to/replace existing ones
                        TrySelectResult::New(result)
                    } else {
                        // Not new selection found, but preserve existing selection(s)
                        TrySelectResult::Preserve
                    };
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
    pub fn try_context_menu(&self, underneath: glm::Vec2, camera: &Camera, color_scheme: &ColorScheme) -> Option<ContextMenu> {
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
