use crate::{
    color_scheme::ColorScheme,
    context_menu::ContextMenu,
    mesh::{MeshXyz, MeshXyzUv},
    ocean::{Ocean, Selection},
    smooth::Smooth,
    squid::{Initiation, Squid, SquidRef},
    tool::{Capture, Interaction, Tool, ToolKey},
    toolbox::ToolBox,
};
use glium::{
    glutin::{
        dpi::LogicalPosition,
        event::{ModifiersState, VirtualKeyCode},
        window::CursorIcon,
    },
    Display, Program,
};
use glium_text_rusttype::{FontTexture, TextSystem};
use nalgebra_glm as glm;
use slotmap::SlotMap;
use std::{
    collections::{btree_set::BTreeSet, HashMap},
    rc::Rc,
    time::Instant,
};

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
    pub history: History,
    pub dimensions: Option<(f32, f32)>,
    pub projection: Option<glm::Mat4>,
    pub view: Option<glm::Mat4>,
    pub frame_start_time: Instant,
    pub camera: Smooth<glm::Vec2>,
    pub dragging: Option<Dragging>,
    pub selections: Vec<Selection>,
    pub keys_held: BTreeSet<VirtualKeyCode>,
    pub modifiers_held: ModifiersState,
    pub text_system: TextSystem,
    pub font: Rc<FontTexture>,
    pub context_menu: Option<ContextMenu>,
    pub numeric_mappings: HashMap<VirtualKeyCode, char>,
    pub interaction_options: InteractionOptions,
    pub wait_for_stop_drag: bool,
    pub operation: Option<Operation>,
}

pub enum Operation {
    Rotation { point: glm::Vec2, rotation: f32 },
    Scale { point: glm::Vec2, origin: glm::Vec2 },
}

pub struct InteractionOptions {
    pub translation_snapping: f32,
    pub rotation_snapping: f32,
    pub duplication_offset: glm::Vec2,
}

impl InteractionOptions {
    pub fn new() -> Self {
        Self {
            translation_snapping: 1.0,
            rotation_snapping: 0.0,
            duplication_offset: glm::zero(),
        }
    }
}

trait ControlOrCommand {
    fn control_or_command(&self) -> bool;
}

impl ControlOrCommand for ModifiersState {
    fn control_or_command(&self) -> bool {
        if cfg!(target_os = "macos") {
            self.logo()
        } else {
            self.ctrl()
        }
    }
}

impl ApplicationState {
    // Tries to interact with any already selected squids
    // Returns whether interaction was captured
    pub fn preclick(&mut self) {
        for (_, squid) in self.ocean.squids.iter_mut() {
            squid.interact(&Interaction::PreClick, &self.camera.get_animated(), &self.interaction_options);
        }
    }

    pub fn try_interact_with_selections(&mut self, interaction: &Interaction) -> Capture {
        for (reference, squid) in self.ocean.get_squids_newest_mut() {
            if selection_contains(&self.selections, reference) {
                squid.interact(interaction, &self.camera.get_animated(), &self.interaction_options)?;
            }
        }

        Capture::Miss
    }

    pub fn press_key(&mut self, key: &VirtualKeyCode, tools: &mut SlotMap<ToolKey, Box<dyn Tool>>) {
        if self.modifiers_held.control_or_command() && key == &VirtualKeyCode::Z {
            if self.modifiers_held.shift() {
                self.redo();
            } else {
                self.undo();
            }
            return;
        }

        if let Some(tool_key) = self.toolbox.get_selected() {
            if tools[tool_key].interact(Interaction::Key { virtual_keycode: *key }, self) != Capture::Miss {
                return;
            }

            if tools[tool_key].interact_options(Interaction::Key { virtual_keycode: *key }, self) != Capture::Miss {
                return;
            }
        }

        match key {
            VirtualKeyCode::Key1 => self.toolbox.select(0),
            VirtualKeyCode::Key2 => self.toolbox.select(1),
            VirtualKeyCode::Key3 => self.toolbox.select(2),
            VirtualKeyCode::Key4 => self.toolbox.select(3),
            VirtualKeyCode::Key5 => self.toolbox.select(4),
            VirtualKeyCode::Key6 => self.toolbox.select(5),
            VirtualKeyCode::Key7 => self.toolbox.select(6),
            VirtualKeyCode::Key8 => self.toolbox.select(7),
            VirtualKeyCode::Key9 => self.toolbox.select(8),
            VirtualKeyCode::Key0 => self.toolbox.select(9),
            VirtualKeyCode::X => self.delete_selected(),
            VirtualKeyCode::Escape => self.context_menu = None,
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
            Capture::Keyboard(..) => (),
            Capture::MoveSelectedSquids { delta } => {
                for squid_id in self.get_selected_squids() {
                    if let Some(squid) = self.ocean.squids.get_mut(squid_id) {
                        squid.translate(&delta, &self.interaction_options);
                    }
                }
            }
            Capture::RotateSelectedSquids { delta_theta } => {
                for squid_id in self.get_selected_squids() {
                    if let Some(squid) = self.ocean.squids.get_mut(squid_id) {
                        squid.rotate(*delta_theta, &self.interaction_options);
                    }
                }
            }
            Capture::ScaleSelectedSquids { total_scale_factor } => {
                for squid_id in self.get_selected_squids() {
                    if let Some(squid) = self.ocean.squids.get_mut(squid_id) {
                        squid.scale(*total_scale_factor, &self.interaction_options);
                    }
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
        let offset = self.interaction_options.duplication_offset;
        let created: Vec<SquidRef> = self
            .get_selected_squids()
            .iter()
            .filter(|squid_id| self.ocean.squids.get(**squid_id).is_some())
            .map(|x| *x)
            .collect();
        let created: Vec<SquidRef> = created
            .iter()
            .map(|squid_id| self.insert(self.ocean.squids[*squid_id].duplicate(&offset)))
            .collect();

        self.selections.clear();
        self.selections = created.iter().map(|squid_id| Selection::new(*squid_id, None)).collect();
    }

    pub fn get_selected_squids<'a>(&self) -> Vec<SquidRef> {
        self.selections.iter().filter(|x| x.limb_id.is_none()).map(|x| x.squid_id).collect()
    }

    pub fn initiate(&mut self, initiation: Initiation) {
        self.dragging = Some(Dragging::new(self.mouse_position.unwrap_or_default()));
        self.wait_for_stop_drag = true;

        match initiation {
            Initiation::Rotation => {
                let mouse = self.mouse_position.unwrap();
                let camera = self.camera.get_animated();
                let position = glm::vec2(mouse.x, mouse.y) - camera;

                if let Some(rotate_point) = self.get_closest_selection_center(&position) {
                    let point = rotate_point + camera;
                    let rotation = (rotate_point.y - position.y).atan2(position.x - rotate_point.x) - std::f32::consts::FRAC_PI_2;
                    self.operation = Some(Operation::Rotation { point, rotation });
                }
            }
            Initiation::Scale => {
                let mouse = self.mouse_position.unwrap();
                let camera = self.camera.get_animated();
                let point = glm::vec2(mouse.x, mouse.y) - camera;

                if let Some(origin) = self.get_closest_selection_center(&point) {
                    self.operation = Some(Operation::Scale { point, origin });
                }
            }
            _ => (),
        }

        for squid_id in self.get_selected_squids() {
            if let Some(squid) = self.ocean.squids.get_mut(squid_id) {
                squid.initiate(initiation);
            }
        }
    }

    pub fn get_closest_selection_center(&self, position: &glm::Vec2) -> Option<glm::Vec2> {
        let mut least_distance = f32::INFINITY;
        let mut closest_center: Option<glm::Vec2> = None;
        for (_, squid) in self.ocean.squids.iter() {
            let center = squid.get_center();
            let distance = glm::distance(position, &center);

            if distance < least_distance {
                least_distance = distance;
                closest_center = Some(center);
            }
        }
        closest_center
    }

    pub fn add_history_marker(&mut self) {
        self.history.push(self.ocean.clone());
    }

    pub fn undo(&mut self) {
        if let Some(previous) = self.history.undo() {
            self.ocean = previous;
        }
    }

    pub fn redo(&mut self) {
        if let Some(previous) = self.history.redo() {
            self.ocean = previous;
        }
    }

    pub fn insert(&mut self, value: Box<dyn Squid>) -> SquidRef {
        self.prune_selection();
        self.ocean.squids.insert(value)
    }

    pub fn prune_selection(&mut self) {
        self.selections = self
            .selections
            .iter()
            .filter(|x| self.ocean.squids.get(x.squid_id).is_some())
            .map(|x| *x)
            .collect();
    }
}

pub struct History {
    history: Vec<Ocean>,
    time_travel: usize,
}

impl History {
    const MAX_HISTORY: usize = 100;

    pub fn new() -> Self {
        Self {
            history: vec![],
            time_travel: 0,
        }
    }

    pub fn push(&mut self, value: Ocean) {
        if self.history.len() == 0 {
            self.history.push(Ocean::new());
        } else {
            while self.time_travel < self.history.len() - 1 {
                self.history.pop();
            }
        }

        while self.history.len() >= Self::MAX_HISTORY {
            self.history.remove(0);
            self.time_travel -= 1;
        }

        self.history.push(value);
        self.time_travel = self.history.len() - 1;
    }

    pub fn undo(&mut self) -> Option<Ocean> {
        if self.time_travel > 0 {
            self.time_travel -= 1;
            let in_the_past = self.history[self.time_travel].clone();
            Some(in_the_past)
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<Ocean> {
        if self.time_travel + 1 < self.history.len() {
            self.time_travel += 1;
            let towards_present = self.history[self.time_travel].clone();
            Some(towards_present)
        } else {
            None
        }
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
