#![warn(
    clippy::explicit_iter_loop,
    clippy::semicolon_if_nothing_returned,
    clippy::wildcard_imports,
    clippy::too_many_lines,
    clippy::redundant_else
)]
#![feature(try_trait_v2)]

mod aabb;
mod accumulator;
mod algorithm;
mod annotations;
mod app;
mod bool_poll;
mod capture;
mod checkbox;
mod color;
mod color_impls;
mod color_scheme;
mod context_menu;
mod dragging;
mod history;
mod icon_button;
mod interaction;
mod interaction_options;
mod layer;
mod math_helpers;
mod matrix_helpers;
mod mesh;
mod obj_helpers;
mod ocean;
mod operation;
mod options;
mod press_animation;
mod render_ctx;
mod selection;
mod shader_helpers;
mod shaders;
mod smooth;
mod squid;
mod text_helpers;
mod text_input;
mod tool;
mod tool_button;
mod toolbox;
mod user_input;
mod vertex;

const TARGET_FPS: u64 = 60;

use app::{ApplicationState, MULTISAMPLING_COUNT};
use capture::Capture;
use color_scheme::ColorScheme;
use context_menu::ContextAction;
use dragging::Dragging;
use glium::{
    glutin::{
        event::{ElementState, Event::WindowEvent as AbstractWindowEvent, ModifiersState, MouseButton, WindowEvent as ConcreteWindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
        ContextBuilder, GlProfile, GlRequest,
    },
    Display,
};
use glium_text_rusttype as glium_text;
use interaction::Interaction;
use mesh::{MeshXyz, MeshXyzUv};
use nalgebra_glm as glm;
use render_ctx::RenderCtx;
use selection::selection_contains;
use shaders::Shaders;
use slotmap::SlotMap;
use smooth::Smooth;
use squid::{Initiation, SquidRef};
use std::{
    collections::btree_set::BTreeSet,
    rc::Rc,
    time::{Duration, Instant},
};
use tool::{Tool, ToolKey};
use toolbox::ToolBox;

fn main() {
    // <コ:彡

    // Build window
    let event_loop = EventLoop::new();
    let window_builder = WindowBuilder::new()
        .with_title("Photosquid :)")
        .with_inner_size(glium::glutin::dpi::LogicalSize::new(1280, 720));
    let context_builder = ContextBuilder::new()
        .with_srgb(true)
        .with_gl_profile(GlProfile::Core)
        .with_gl(GlRequest::Specific(glium::glutin::Api::OpenGl, (4, 0)))
        .with_multisampling(MULTISAMPLING_COUNT)
        .with_double_buffer(Some(true))
        .with_vsync(true)
        .with_depth_buffer(8);
    let display = Display::new(window_builder, context_builder, &event_loop).unwrap();

    // Build toolbox
    let mut toolbox = ToolBox::new(&display);
    let mut tools: SlotMap<ToolKey, Box<dyn Tool>> = SlotMap::with_key();
    let mut options_tabs: SlotMap<options::tab::TabKey, Box<dyn options::tab::Tab>> = SlotMap::with_key();

    // Create standard tool set
    toolbox.create_standard_tools(&mut tools, &display);
    toolbox.create_standard_options_tabs(&mut options_tabs, &display);

    let ribbon_mesh = MeshXyz::new_ui_rect(&display);
    let ring_mesh = MeshXyz::new_ui_ring(&display);
    let check_mesh = MeshXyz::new_ui_check(&display);
    let square_xyzuv = MeshXyzUv::new_square(&display);

    let shaders = Shaders::new(&display);
    let text_system = glium_text::TextSystem::new(&display);

    let font = glium_text::FontTexture::new(
        &display,
        std::fs::File::open(&std::path::Path::new("Roboto-Regular.ttf")).unwrap(),
        20,
        glium_text::FontTexture::ascii_character_list(),
    )
    .unwrap();

    let scale_factor = display.gl_window().window().scale_factor();

    let mut app = ApplicationState {
        display,
        color_scheme: Default::default(),
        toolbox,
        ribbon_mesh,
        ring_mesh,
        check_mesh,
        square_xyzuv,
        shaders,
        mouse_position: None,
        scale_factor,
        ocean: Default::default(),
        history: Default::default(),
        dimensions: None,
        projection: None,
        view: None,
        frame_start_time: Instant::now(),
        camera: Smooth::new(glm::vec2(0.0, 0.0), Duration::from_millis(500)),
        dragging: None,
        selections: vec![],
        keys_held: BTreeSet::new(),
        modifiers_held: ModifiersState::empty(),
        text_system,
        font: Rc::new(font),
        context_menu: None,
        interaction_options: Default::default(),
        wait_for_stop_drag: false,
        operation: None,
        perform_next_operation_collectively: false,
    };

    event_loop.run(move |abstract_event, _, control_flow| {
        let framebuffer_dimensions = app.display.get_framebuffer_dimensions();

        app.frame_start_time = Instant::now();
        app.dimensions = Some((
            framebuffer_dimensions.0 as f32 / app.scale_factor as f32,
            framebuffer_dimensions.1 as f32 / app.scale_factor as f32,
        ));

        // Handle user input
        if let Some(new_control_flow) = on_event(abstract_event, &mut app, &mut tools, &mut options_tabs) {
            *control_flow = new_control_flow;
            return;
        }

        // Update components
        update_components(&mut app);

        // Handle control flow
        if !matches!(*control_flow, ControlFlow::Exit) {
            let elapsed_time = Instant::now().duration_since(app.frame_start_time).as_millis() as u64;
            app.display.gl_window().window().request_redraw();

            let wait_millis = match 100 / TARGET_FPS >= elapsed_time {
                true => 1000 / TARGET_FPS - elapsed_time,
                false => 0,
            };

            let next_frame_time = app.frame_start_time + Duration::from_millis(wait_millis);
            *control_flow = ControlFlow::WaitUntil(next_frame_time);
        }
    });
}

fn on_event(
    abstract_event: glium::glutin::event::Event<()>,
    app: &mut ApplicationState,
    tools: &mut SlotMap<ToolKey, Box<dyn Tool>>,
    options_tabs: &mut SlotMap<options::tab::TabKey, Box<dyn options::tab::Tab>>,
) -> Option<ControlFlow> {
    match abstract_event {
        AbstractWindowEvent { event, .. } => match event {
            ConcreteWindowEvent::CloseRequested => return Some(ControlFlow::Exit),
            ConcreteWindowEvent::KeyboardInput { input, .. } => on_keyboard_input(app, tools, input),
            ConcreteWindowEvent::ModifiersChanged(value) => app.modifiers_held = value,
            ConcreteWindowEvent::MouseInput { state, button, .. } => on_mouse_input(app, tools, state, button),
            ConcreteWindowEvent::CursorMoved { position, .. } => on_mouse_move(app, tools, position),
            ConcreteWindowEvent::ScaleFactorChanged { scale_factor, .. } => app.scale_factor = scale_factor,
            _ => (),
        },
        glium::glutin::event::Event::RedrawRequested(..) => redraw(app, tools, options_tabs),
        _ => (),
    }
    None
}

fn update_components(app: &mut ApplicationState) {
    let (width, height) = app.dimensions.unwrap();

    app.toolbox.update(width, height);

    if let Some(new_color) = app.toolbox.color_picker.poll() {
        for selection in app.selections.iter().filter(|x| x.limb_id.is_none()) {
            if let Some(squid) = app.ocean.get_mut(selection.squid_id) {
                squid.set_color(new_color);
            }
        }
    }
}

fn redraw(
    app: &mut ApplicationState,
    tools: &mut SlotMap<ToolKey, Box<dyn Tool>>,
    options_tabs: &mut SlotMap<options::tab::TabKey, Box<dyn options::tab::Tab>>,
) {
    // Get dimensions of window
    let (width, height) = app.dimensions.unwrap();
    let (width_u32, height_u32) = app.display.get_framebuffer_dimensions();

    // Create texture to hold render output (if we aren't going to render directly)
    let rendered = glium::texture::SrgbTexture2d::empty(&app.display, width_u32, height_u32).unwrap();

    // Create framebuffer (in case we aren't going to render directly)
    let mut framebuffer = glium::framebuffer::SimpleFrameBuffer::new(&app.display, &rendered).unwrap();

    // Setup matrices
    app.projection = Some(glm::ortho(0.0, width, height, 0.0, 100.0, -100.0));
    app.view = Some(glm::translation(&glm::vec2_to_vec3(&app.camera.get_animated())));

    // Create target
    let mut target = app.display.draw();

    // Render main application
    render_app(app, tools, options_tabs, &mut target, &mut framebuffer);

    // If we rendered indirectly, then render the final output to screen now
    if app.scale_factor != 1.0 {
        render_television(&mut target, &rendered, &app.square_xyzuv, &app.shaders.television_shader);
    }

    // Finalize render
    target.finish().unwrap();
}

fn render_app<'f>(
    app: &mut ApplicationState,
    tools: &mut SlotMap<ToolKey, Box<dyn Tool>>,
    options_tabs: &mut SlotMap<options::tab::TabKey, Box<dyn options::tab::Tab>>,
    target: &'f mut glium::Frame,
    framebuffer: &'f mut glium::framebuffer::SimpleFrameBuffer<'f>,
) {
    let (width, height) = app.dimensions.unwrap();

    // TLDR: Don't use 'target' or 'framebuffer' directly unless you really know
    // what you're doing. Instead use methods of 'RenderCtx' that will automatically
    // use the appropriate target.

    // NOTE: We won't use 'target' and 'framebuffer' directly most of the time,
    // since which we render to is dependent on the display's scale factor.
    // We do this because on macOS, for some reason, to only way to enable MSAA,
    // is by rendering directly to the buffer. So for retina (and other non-1-factor displays),
    // we will render to a framebuffer first (since more pixels will be sampled anyways),
    // but for displays that have a 1-factor ratio, we will render directly and utilize
    // the built in MSAA for the window render target (this is the only portable way apparently)
    // Render context is a subset of ApplicationState that only
    // contains information related to rendering

    let mut ctx: RenderCtx<'_, 'f> = RenderCtx {
        target,
        framebuffer,
        color_shader: &app.shaders.color_shader,
        hue_value_picker_shader: &app.shaders.hue_value_picker_shader,
        saturation_picker_shader: &app.shaders.saturation_picker_shader,
        rounded_rectangle_shader: &app.shaders.rounded_rectangle_shader,
        projection: &app.projection.unwrap(),
        view: &app.view.unwrap(),
        width,
        height,
        scale_factor: app.scale_factor,
        ribbon_mesh: &app.ribbon_mesh,
        ring_mesh: &app.ring_mesh,
        check_mesh: &app.check_mesh,
        square_xyzuv: &app.square_xyzuv,
        color_scheme: &app.color_scheme,
        camera: &app.camera.get_animated(),
        display: &app.display,
    };

    ctx.clear_color(&app.color_scheme.background);

    for reference in &app.ocean.get_squids_lowest().collect::<Vec<_>>() {
        if let Some(squid) = app.ocean.get_mut(*reference) {
            squid.render(&mut ctx, None);

            if selection_contains(&app.selections, *reference) {
                squid.render_selected_indication(&mut ctx);
            }
        }
    }

    app.toolbox.render(
        &mut ctx,
        tools,
        options_tabs,
        &app.color_scheme,
        &app.text_system,
        app.font.clone(),
        &mut app.ocean,
        &app.selections,
    );

    if let Some(context_menu) = &mut app.context_menu {
        context_menu.render(&mut ctx, &app.text_system, app.font.clone());
    }
}

fn render_television(target: &mut glium::Frame, rendered: &glium::texture::SrgbTexture2d, television: &MeshXyzUv, television_shader_program: &glium::Program) {
    // If we're not doing MSAA, render a framebuffer instead of having just rendered directly.
    // Draw render to window

    use glium::Surface;
    use matrix_helpers::reach_inside_mat4;

    let identity = glm::identity::<f32, 4>();

    let uniforms = glium::uniform! {
        transformation: reach_inside_mat4(&identity),
        view: reach_inside_mat4(&identity),
        projection: reach_inside_mat4(&identity),
        texture_sampler: rendered
    };

    target
        .draw(
            &television.vertex_buffer,
            &television.indices,
            television_shader_program,
            &uniforms,
            &Default::default(),
        )
        .unwrap();
}

fn do_click(state: &mut ApplicationState, tools: &mut SlotMap<ToolKey, Box<dyn Tool>>, button: MouseButton) -> Capture {
    // Returns whether a drag is allowed to start

    use bool_poll::BoolPoll;

    if state.wait_for_stop_drag.poll() {
        state.dragging = None;
        state.operation = None;
        return Capture::NoDrag;
    }

    let position = state.mouse_position.unwrap();
    let position = glm::vec2(position.x, position.y);
    let (width, height) = state.dimensions.unwrap_or_default();

    if let Some(context_menu) = &state.context_menu {
        let action = context_menu.click(button, &position);
        state.context_menu = None;

        if let Some(action) = action {
            // Handle action
            match action {
                ContextAction::DeleteSelected => state.delete_selected(),
                ContextAction::DuplicateSelected => state.duplicate_selected(),
                ContextAction::GrabSelected => {
                    if state.perform_next_operation_collectively {
                        if let Some(center) = state.get_selection_group_center() {
                            state.initiate(Initiation::Spread {
                                point: state.get_mouse_in_world_space(),
                                center,
                            });
                        }
                        state.perform_next_operation_collectively = false;
                    } else {
                        state.initiate(Initiation::Translate);
                    }
                }
                ContextAction::RotateSelected => {
                    if state.perform_next_operation_collectively {
                        if let Some(center) = state.get_selection_group_center() {
                            state.initiate(Initiation::Revolve {
                                point: state.get_mouse_in_world_space(),
                                center,
                            });
                        }
                        state.perform_next_operation_collectively = false;
                    } else {
                        state.initiate(Initiation::Rotate);
                    }
                }
                ContextAction::ScaleSelected => {
                    if state.perform_next_operation_collectively {
                        if let Some(center) = state.get_selection_group_center() {
                            state.initiate(Initiation::Dilate {
                                point: state.get_mouse_in_world_space(),
                                center,
                            });
                        }
                        state.perform_next_operation_collectively = false;
                    } else {
                        state.initiate(Initiation::Scale);
                    }
                }
                ContextAction::Collectively => state.perform_next_operation_collectively = !state.perform_next_operation_collectively,
            }
            return Capture::NoDrag;
        }
    }

    // Tool options ribbon
    if let Some(tool_key) = state.toolbox.get_selected() {
        tools[tool_key].interact_options(Interaction::Click { button, position }, state)?;
    }

    // Tool ribbon
    if state.toolbox.click(button, &position, width, height) {
        return Capture::AllowDrag;
    }

    if let Some(tool_key) = state.toolbox.get_selected() {
        return tools[tool_key].interact(Interaction::Click { button, position }, state);
    }

    Capture::Miss
}

fn do_mouse_release(app: &mut ApplicationState, button: MouseButton) {
    let position = app.mouse_position.unwrap();
    let position = glm::vec2(position.x, position.y);
    let animated_camera = app.camera.get_animated();
    let unordered_squids: Vec<SquidRef> = app.ocean.get_squids_unordered().collect();

    for reference in unordered_squids {
        if let Some(squid) = app.ocean.get_mut(reference) {
            squid.interact(&Interaction::MouseRelease { position, button }, &animated_camera, &app.interaction_options);
        }
    }

    app.toolbox.mouse_release(button);

    // Primitive history
    app.add_history_marker();
}

fn do_drag(app: &mut ApplicationState, tools: &mut SlotMap<ToolKey, Box<dyn Tool>>) -> Capture {
    let drag = app.dragging.as_ref().unwrap().to_interaction();
    let (width, _) = app.dimensions.unwrap_or_default();

    app.toolbox.drag(MouseButton::Left, &drag, width)?;

    if let Some(tool_key) = app.toolbox.get_selected() {
        tools[tool_key].interact(drag, app)?;
    }

    Capture::Miss
}

pub fn on_keyboard_input(app: &mut ApplicationState, tools: &mut SlotMap<ToolKey, Box<dyn Tool>>, input: glium::glutin::event::KeyboardInput) {
    if let Some(virtual_keycode) = input.virtual_keycode {
        let keys_held = &mut app.keys_held;

        match input.state {
            ElementState::Pressed => {
                if keys_held.insert(virtual_keycode) {
                    // Press first time
                    app.press_key(virtual_keycode, tools);
                }
            }
            ElementState::Released => {
                keys_held.remove(&virtual_keycode);
            }
        }
    }
}

fn on_mouse_input(app: &mut ApplicationState, tools: &mut SlotMap<ToolKey, Box<dyn Tool>>, state: ElementState, button: MouseButton) {
    if state == ElementState::Pressed {
        match do_click(app, tools, button) {
            Capture::NoDrag => (),
            capture => {
                app.dragging = Some(Dragging::new(app.mouse_position.unwrap_or_default()));
                app.handle_captured(&capture);
            }
        }
    } else {
        do_mouse_release(app, button);

        if !app.wait_for_stop_drag {
            app.dragging = None;
        }
    }
}

fn on_mouse_move(app: &mut ApplicationState, tools: &mut SlotMap<ToolKey, Box<dyn Tool>>, position: glium::glutin::dpi::PhysicalPosition<f64>) {
    app.mouse_position = Some(position.to_logical(app.scale_factor));

    if let Some(dragging) = app.dragging.as_mut() {
        let logical_position = app.mouse_position.unwrap();
        dragging.update(glm::vec2(logical_position.x, logical_position.y));

        let capture = do_drag(app, tools);
        app.handle_captured(&capture);
    }
}
