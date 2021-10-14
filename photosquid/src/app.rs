use crate::{
    color_scheme::ColorScheme,
    context_menu::ContextMenu,
    mesh::{MeshXyz, MeshXyzUv},
    ocean::{Ocean, Selection},
    smooth::Smooth,
    squid::SquidRef,
    tool::{Capture, Interaction},
    toolbox::ToolBox,
};
use glium::{
    glutin::{dpi::LogicalPosition, event::VirtualKeyCode, window::CursorIcon},
    Display, Program,
};
use glium_text_rusttype::{FontTexture, TextSystem};
use nalgebra_glm as glm;
use std::{collections::btree_set::BTreeSet, rc::Rc, time::Instant};

pub const MULTISAMPLING_COUNT: u16 = 4;

pub struct ApplicationState {
    pub display: Display,
    pub color_scheme: ColorScheme,
    pub toolbox: ToolBox,
    pub ribbon_mesh: MeshXyz,
    pub ring_mesh: MeshXyz,
    pub square_xyzuv: MeshXyzUv,
    pub color_shader_program: Program,
    pub hue_value_picker_shader_program: Program,
    pub saturation_picker_shader_program: Program,
    pub rounded_rectangle_shader_program: Program,
    pub mouse_position: Option<LogicalPosition<f32>>,
    pub scale_factor: f64,
    pub ocean: Ocean,
    pub dimensions: Option<(f32, f32)>,
    pub projection: Option<glm::Mat4>,
    pub view: Option<glm::Mat4>,
    pub frame_start_time: Instant,
    pub camera: Smooth<glm::Vec2>,
    pub dragging: Option<Dragging>,
    pub selections: Vec<Selection>,
    pub keys_held: BTreeSet<VirtualKeyCode>,
    pub text_system: TextSystem,
    pub font: Rc<FontTexture>,
    pub context_menu: Option<ContextMenu>,
}

impl ApplicationState {
    // Tries to interact with any already selected squids
    // Returns whether interaction was captured
    pub fn try_interact_with_selections(&mut self, interaction: &Interaction) -> Capture {
        for (reference, squid) in self.ocean.get_squids_newest_mut() {
            if selection_contains(&self.selections, reference) {
                squid.interact(interaction, &self.camera.get_animated())?;
            }
        }

        Capture::Miss
    }

    pub fn press_key(&mut self, key: &VirtualKeyCode) {
        match key {
            VirtualKeyCode::Key1 => self.toolbox.select(0),
            VirtualKeyCode::Key2 => self.toolbox.select(1),
            VirtualKeyCode::Key3 => self.toolbox.select(2),
            VirtualKeyCode::X => self.delete_selected(),
            VirtualKeyCode::D => {
                if self.keys_held.contains(&VirtualKeyCode::LShift) {
                    self.duplicate_selected();
                }
            }
            _ => (),
        }
    }

    #[allow(dead_code)]
    pub fn set_cursor_icon(&self, cursor: CursorIcon) {
        self.display.gl_window().window().set_cursor_icon(cursor);
    }

    pub fn handle_captured(&mut self, capture: &Capture) {
        match capture {
            Capture::Miss => (),
            Capture::AllowDrag => (),
            Capture::NoDrag => (),
            Capture::MoveSelectedSquids { delta } => {
                for squid_id in self.get_selected_squids() {
                    self.ocean.squids[squid_id].translate(&delta);
                }
            }
            Capture::RotateSelectedSquids { delta_theta } => {
                for squid_id in self.get_selected_squids() {
                    self.ocean.squids[squid_id].rotate(*delta_theta);
                }
            }
        }
    }

    pub fn delete_selected(&mut self) {
        for squid_id in self.get_selected_squids() {
            self.ocean.squids.remove(squid_id);
        }
        self.selections.clear();
    }

    pub fn duplicate_selected(&mut self) {
        let offset = glm::vec2(10.0, 10.0);
        let created: Vec<SquidRef> = self
            .get_selected_squids()
            .iter()
            .map(|squid_id| self.ocean.squids.insert(self.ocean.squids[*squid_id].duplicate(&offset, &self.display)))
            .collect();

        self.selections.clear();
        self.selections = created.iter().map(|squid_id| Selection::new(*squid_id, None)).collect();
    }

    pub fn get_selected_squids<'a>(&self) -> Vec<SquidRef> {
        self.selections.iter().filter(|x| x.limb_id.is_none()).map(|x| x.squid_id).collect()
    }
}

pub struct Dragging {
    pub down: glm::Vec2,
    pub current: glm::Vec2,
    pub last: glm::Vec2,
}

impl Dragging {
    pub fn new(mouse_position: LogicalPosition<f32>) -> Self {
        let position: glm::Vec2 = glm::vec2(mouse_position.x, mouse_position.y);

        Self {
            down: position,
            current: position,
            last: position,
        }
    }

    pub fn update(&mut self, mouse_position: glm::Vec2) {
        self.last = self.current;
        self.current = mouse_position;
    }

    pub fn get_delta(&self) -> glm::Vec2 {
        self.current - self.last
    }

    pub fn to_interaction(&self) -> Interaction {
        Interaction::Drag {
            delta: self.get_delta(),
            start: self.down,
            current: self.current,
        }
    }
}

pub fn selection_contains(selections: &Vec<Selection>, squid_reference: SquidRef) -> bool {
    for selection in selections.iter() {
        if selection.squid_id == squid_reference {
            return true;
        }
    }
    false
}
