use crate::{
    camera::Camera,
    capture::Capture,
    color_scheme::ColorScheme,
    context_menu::ContextMenu,
    dragging::Dragging,
    history::History,
    interaction::Interaction,
    interaction_options::InteractionOptions,
    mesh::{MeshXyz, MeshXyzUv},
    ocean::Ocean,
    operation::Operation,
    selection::{selection_contains, Selection},
    shaders::Shaders,
    smooth::Smooth,
    squid::{Initiation, Squid, SquidRef},
    tool::{Tool, ToolKey},
    toolbox::ToolBox,
};
use angular_units::Rad;
use glium::{
    glutin::{
        dpi::LogicalPosition,
        event::{ModifiersState, VirtualKeyCode},
        window::CursorIcon,
    },
    Display,
};
use glium_text_rusttype::{FontTexture, TextSystem};
use nalgebra_glm as glm;
use slotmap::SlotMap;
use std::{collections::btree_set::BTreeSet, rc::Rc, time::Instant};

pub const MULTISAMPLING_COUNT: u16 = 4;

pub struct ApplicationState {
    pub display: Display,
    pub color_scheme: ColorScheme,
    pub toolbox: ToolBox,
    pub ribbon_mesh: MeshXyz,
    pub ring_mesh: MeshXyz,
    pub check_mesh: MeshXyz,
    pub square_xyzuv: MeshXyzUv,
    pub shaders: Shaders,
    pub mouse_position: Option<LogicalPosition<f32>>,
    pub scale_factor: f64,
    pub ocean: Ocean,
    pub history: History,
    pub dimensions: Option<(f32, f32)>,
    pub projection: Option<glm::Mat4>,
    pub view: Option<glm::Mat4>,
    pub frame_start_time: Instant,
    pub camera: Smooth<Camera>,
    pub dragging: Option<Dragging>,
    pub selections: Vec<Selection>,
    pub keys_held: BTreeSet<VirtualKeyCode>,
    pub modifiers_held: ModifiersState,
    pub text_system: TextSystem,
    pub font: Rc<FontTexture>,
    pub context_menu: Option<ContextMenu>,
    pub interaction_options: InteractionOptions,
    pub wait_for_stop_drag: bool,
    pub operation: Option<Operation>,
    pub perform_next_operation_collectively: bool,
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
        let unordered_squids: Vec<SquidRef> = self.ocean.get_squids_unordered().collect();

        for reference in unordered_squids {
            if let Some(squid) = self.ocean.get_mut(reference) {
                squid.interact(&Interaction::PreClick, &self.camera.get_animated(), &self.interaction_options);
            }
        }
    }

    pub fn try_interact_with_selections(&mut self, interaction: &Interaction) -> Capture {
        let highest_squids: Vec<SquidRef> = self.ocean.get_squids_unordered().collect();

        for reference in highest_squids {
            if selection_contains(&self.selections, reference) {
                if let Some(squid) = self.ocean.get_mut(reference) {
                    squid.interact(interaction, &self.camera.get_animated(), &self.interaction_options)?;
                }
            }
        }

        Capture::Miss
    }

    pub fn press_key(&mut self, key: VirtualKeyCode, tools: &mut SlotMap<ToolKey, Box<dyn Tool>>) {
        use crate::camera::EasySmoothCamera;

        if self.modifiers_held.control_or_command() && key == VirtualKeyCode::Z {
            if self.modifiers_held.shift() {
                self.redo();
            } else {
                self.undo();
            }
            return;
        }

        if self.modifiers_held.control_or_command() && key == VirtualKeyCode::Equals {
            self.camera.increase_zoom();
            return;
        }

        if self.modifiers_held.control_or_command() && key == VirtualKeyCode::Minus {
            self.camera.decrease_zoom();
            return;
        }

        if let Some(tool_key) = self.toolbox.get_selected() {
            if tools[tool_key].interact(Interaction::Key { virtual_keycode: key }, self) != Capture::Miss {
                return;
            }

            if tools[tool_key].interact_options(Interaction::Key { virtual_keycode: key }, self) != Capture::Miss {
                return;
            }
        }

        match key {
            VirtualKeyCode::Key1 => self.toolbox.select_tool(0),
            VirtualKeyCode::Key2 => self.toolbox.select_tool(1),
            VirtualKeyCode::Key3 => self.toolbox.select_tool(2),
            VirtualKeyCode::Key4 => self.toolbox.select_tool(3),
            VirtualKeyCode::Key5 => self.toolbox.select_tool(4),
            VirtualKeyCode::Key6 => self.toolbox.select_tool(5),
            VirtualKeyCode::Key7 => self.toolbox.select_tool(6),
            VirtualKeyCode::Key8 => self.toolbox.select_tool(7),
            VirtualKeyCode::Key9 => self.toolbox.select_tool(8),
            VirtualKeyCode::Key0 => self.toolbox.select_tool(9),
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

    pub fn handle_captured(&mut self, capture: &Capture, camera: &Camera) {
        match capture {
            Capture::Miss => (),
            Capture::AllowDrag => (),
            Capture::NoDrag => (),
            Capture::TakeFocus => (),
            Capture::Keyboard(..) => (),
            Capture::MoveSelectedSquids { delta_in_world } => {
                for squid_id in self.get_selected_squids() {
                    if let Some(squid) = self.ocean.get_mut(squid_id) {
                        squid.translate(delta_in_world, &self.interaction_options);
                    }
                }
            }
            Capture::RotateSelectedSquids { delta_theta } => {
                for squid_id in self.get_selected_squids() {
                    if let Some(squid) = self.ocean.get_mut(squid_id) {
                        squid.rotate(*delta_theta, &self.interaction_options);
                    }
                }
            }
            Capture::ScaleSelectedSquids { total_scale_factor } => {
                for squid_id in self.get_selected_squids() {
                    if let Some(squid) = self.ocean.get_mut(squid_id) {
                        squid.scale(*total_scale_factor, &self.interaction_options);
                    }
                }
            }
            Capture::SpreadSelectedSquids { current } => {
                for squid_id in self.get_selected_squids() {
                    if let Some(squid) = self.ocean.get_mut(squid_id) {
                        squid.spread(current, &self.interaction_options);
                    }
                }
            }
            Capture::RevolveSelectedSquids { current } => {
                for squid_id in self.get_selected_squids() {
                    if let Some(squid) = self.ocean.get_mut(squid_id) {
                        squid.revolve(current, &self.interaction_options);
                    }
                }
            }
            Capture::DilateSelectedSquids { current } => {
                for squid_id in self.get_selected_squids() {
                    if let Some(squid) = self.ocean.get_mut(squid_id) {
                        squid.dilate(current, &self.interaction_options);
                    }
                }
            }
        }
    }

    pub fn delete_selected(&mut self) {
        for squid_id in self.get_selected_squids() {
            self.ocean.remove(squid_id);
        }
        self.selections.clear();
    }

    pub fn duplicate_selected(&mut self) {
        let offset = self.interaction_options.duplication_offset;
        let created: Vec<SquidRef> = self
            .get_selected_squids()
            .iter()
            .filter(|squid_id| self.ocean.get(**squid_id).is_some())
            .copied()
            .collect();
        let created: Vec<SquidRef> = created
            .iter()
            .map(|squid_id| self.insert(self.ocean.get(*squid_id).unwrap().duplicate(&offset)))
            .collect();

        self.selections.clear();
        self.selections = created.iter().map(|squid_id| Selection::new(*squid_id, None)).collect();
    }

    pub fn get_selected_squids(&self) -> Vec<SquidRef> {
        self.selections.iter().filter(|x| x.limb_id.is_none()).map(|x| x.squid_id).collect()
    }

    pub fn initiate(&mut self, initiation: Initiation) {
        self.dragging = Some(Dragging::new(self.mouse_position.unwrap_or_default()));
        self.wait_for_stop_drag = true;

        match initiation {
            Initiation::Translate { .. } => (),
            Initiation::Rotate => {
                let position = self.get_mouse_in_world_space();

                if let Some(rotate_point) = self.get_closest_selection_center(&position) {
                    let point = self.camera.get_animated().apply(&rotate_point);
                    let rotation = Rad((rotate_point.y - position.y).atan2(position.x - rotate_point.x)) - Rad::pi_over_2();
                    self.operation = Some(Operation::Rotate { point, rotation });
                }
            }
            Initiation::Scale => {
                let point = self.get_mouse_in_world_space();

                if let Some(origin) = self.get_closest_selection_center(&point) {
                    self.operation = Some(Operation::Scale { point, origin });
                }
            }
            Initiation::Spread { point, center } => {
                self.operation = Some(Operation::Spread { point, origin: center });
            }
            Initiation::Revolve { point, center } => {
                self.operation = Some(Operation::Revolve { point, origin: center });
            }
            Initiation::Dilate { point, center } => {
                self.operation = Some(Operation::Dilate { point, origin: center });
            }
        }

        for squid_id in self.get_selected_squids() {
            if let Some(squid) = self.ocean.get_mut(squid_id) {
                squid.initiate(initiation);
            }
        }
    }

    pub fn get_closest_selection_center(&self, position: &glm::Vec2) -> Option<glm::Vec2> {
        let mut least_distance = f32::INFINITY;
        let mut closest_center: Option<glm::Vec2> = None;
        for squid_ref in &self.get_selected_squids() {
            if let Some(squid) = self.ocean.get(*squid_ref) {
                let center = squid.get_center();
                let distance = glm::distance(position, &center);

                if distance < least_distance {
                    least_distance = distance;
                    closest_center = Some(center);
                }
            }
        }
        closest_center
    }

    pub fn get_selection_group_center(&self) -> Option<glm::Vec2> {
        let selected_squids = self.get_selected_squids();

        if selected_squids.is_empty() {
            return None;
        }

        let mut average: glm::Vec2 = glm::zero();

        for squid_ref in &selected_squids {
            if let Some(squid) = self.ocean.get(*squid_ref) {
                average += squid.get_center();
            }
        }

        Some(average / selected_squids.len() as f32)
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
        self.ocean.insert(value)
    }

    pub fn prune_selection(&mut self) {
        self.selections = self.selections.iter().filter(|x| self.ocean.get(x.squid_id).is_some()).copied().collect();
    }

    pub fn get_mouse_in_world_space(&self) -> glm::Vec2 {
        let mouse = self.mouse_position.unwrap();
        let camera = self.camera.get_animated();
        camera.apply_reverse(&glm::vec2(mouse.x, mouse.y))
    }
}
