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
mod approx_instant;
mod bool_poll;
mod camera;
mod capture;
mod clearable;
mod color;
mod color_scheme;
mod context_menu;
mod data;
mod dialog;
mod dragging;
mod history;
mod icon_button;
mod interaction;
mod interaction_options;
mod layer;
mod math;
mod matrix;
mod mesh;
mod obj;
mod ocean;
mod operation;
mod options;
mod press_animation;
mod raster_color;
mod render_ctx;
mod selection;
mod shader;
mod shaders;
mod smooth;
mod squid;
mod text_helpers;
mod tool;
mod tool_button;
mod toolbox;
mod user_input;
mod vertex;

const TARGET_FPS: u64 = 60;

use app::{App, MULTISAMPLING_COUNT};
use camera::Camera;
use capture::Capture;
use color_scheme::ColorScheme;
use context_menu::ContextAction;
use dragging::Dragging;
use glium::{
    glutin::{
        event::{ElementState, Event::WindowEvent as AbstractWindowEvent, ModifiersState, MouseButton, MouseScrollDelta, WindowEvent as ConcreteWindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
        ContextBuilder, GlProfile, GlRequest,
    },
    Display,
};
use glium_text_rusttype as glium_text;
use interaction::{Interaction, MouseReleaseInteraction};
use mesh::{MeshXyz, MeshXyzUv};
use nalgebra_glm as glm;
use render_ctx::RenderCtx;
use selection::selection_contains;
use shaders::Shaders;
use slotmap::SlotMap;
use smooth::Smooth;
use squid::SquidRef;
use std::{
    collections::{btree_set::BTreeSet, HashSet},
    rc::Rc,
    time::{Duration, Instant},
};
use tool::{Tool, ToolKey, ToolKind};
use toolbox::ToolBox;

use crate::{interaction::ClickInteraction, toolbox::find_tool};

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
    let mut tools: SlotMap<ToolKey, Tool> = SlotMap::with_key();
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

    fn view_size_from_framebuffer_dimensions(framebuffer_dimensions: (u32, u32), scale_factor: f64) -> glm::Vec2 {
        let view_width = framebuffer_dimensions.0 as f32 / scale_factor as f32;
        let view_height = framebuffer_dimensions.1 as f32 / scale_factor as f32;
        glm::vec2(view_width, view_height)
    }

    let scale_factor = display.gl_window().window().scale_factor();
    let framebuffer_dimensions = display.get_framebuffer_dimensions();
    let initial_dimensions = view_size_from_framebuffer_dimensions(framebuffer_dimensions, scale_factor);

    let mut app = App {
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
        dimensions: initial_dimensions,
        projection: None,
        view: None,
        frame_start_time: Instant::now(),
        camera: Smooth::new(Camera::identity(initial_dimensions), None),
        dragging: None,
        selections: vec![],
        keys_held: BTreeSet::new(),
        mouse_buttons_held: HashSet::new(),
        modifiers_held: ModifiersState::empty(),
        text_system,
        font: Rc::new(font),
        context_menu: None,
        interaction_options: Default::default(),
        wait_for_stop_drag: false,
        operation: None,
        perform_next_operation_collectively: false,
        filename: None,
    };

    event_loop.run(move |abstract_event, _, control_flow| {
        let framebuffer_dimensions = app.display.get_framebuffer_dimensions();

        app.frame_start_time = Instant::now();
        app.dimensions = view_size_from_framebuffer_dimensions(framebuffer_dimensions, scale_factor);
        app.camera.manual_get_real().window = app.dimensions;

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
    app: &mut App,
    tools: &mut SlotMap<ToolKey, Tool>,
    options_tabs: &mut SlotMap<options::tab::TabKey, Box<dyn options::tab::Tab>>,
) -> Option<ControlFlow> {
    match abstract_event {
        AbstractWindowEvent { event, .. } => match event {
            ConcreteWindowEvent::CloseRequested => return Some(ControlFlow::Exit),
            ConcreteWindowEvent::KeyboardInput { input, .. } => on_keyboard_input(app, tools, input),
            ConcreteWindowEvent::ModifiersChanged(value) => app.modifiers_held = value,
            ConcreteWindowEvent::MouseInput { state, button, .. } => on_mouse_input(app, tools, options_tabs, state, button),
            ConcreteWindowEvent::CursorMoved { position, .. } => on_mouse_move(app, tools, position),
            ConcreteWindowEvent::ScaleFactorChanged { scale_factor, .. } => app.scale_factor = scale_factor,
            ConcreteWindowEvent::MouseWheel { delta, .. } => on_scroll(app, delta),
            _ => (),
        },
        glium::glutin::event::Event::RedrawRequested(..) => redraw(app, tools, options_tabs),
        _ => (),
    }
    None
}

fn update_components(app: &mut App) {
    let [width, height]: [f32; 2] = app.dimensions.into();

    app.toolbox.update(width, height);

    if let Some(new_color) = app.toolbox.color_picker.poll() {
        for selection in app.selections.iter().filter(|x| x.limb_id.is_none()) {
            if let Some(squid) = app.ocean.get_mut(selection.squid_id) {
                squid.set_color(new_color);
            }
        }
    }
}

fn redraw(app: &mut App, tools: &mut SlotMap<ToolKey, Tool>, options_tabs: &mut SlotMap<options::tab::TabKey, Box<dyn options::tab::Tab>>) {
    // Get dimensions of window
    let [width, height]: [f32; 2] = app.dimensions.into();
    let (width_u32, height_u32) = app.display.get_framebuffer_dimensions();

    // Create texture to hold render output (if we aren't going to render directly)
    let rendered = glium::texture::SrgbTexture2d::empty(&app.display, width_u32, height_u32).unwrap();

    // Create framebuffer (in case we aren't going to render directly)
    let mut framebuffer = glium::framebuffer::SimpleFrameBuffer::new(&app.display, &rendered).unwrap();

    // Setup matrices
    app.projection = Some(glm::ortho(0.0, width, height, 0.0, 100.0, -100.0));
    app.view = Some(app.camera.get_animated().mat());

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
    app: &mut App,
    tools: &mut SlotMap<ToolKey, Tool>,
    options_tabs: &mut SlotMap<options::tab::TabKey, Box<dyn options::tab::Tab>>,
    target: &'f mut glium::Frame,
    framebuffer: &'f mut glium::framebuffer::SimpleFrameBuffer<'f>,
) {
    let [width, height]: [f32; 2] = app.dimensions.into();

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
    // Render context is a subset of App that only
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
        real_camera: app.camera.get_real(),
        display: &app.display,
    };

    ctx.clear_color(&app.color_scheme.background);

    // Render squids and their selection points
    {
        let ctx = &mut ctx;
        let mut all_selection_points: Vec<glm::Vec2> = vec![];

        for reference in &app.ocean.get_squids_lowest().collect::<Vec<_>>() {
            if let Some(squid) = app.ocean.get_mut(*reference) {
                squid.render(ctx, None);

                if selection_contains(&app.selections, *reference) {
                    squid.get_selection_points(ctx.camera, &mut all_selection_points);
                }
            }
        }

        for point in all_selection_points {
            ctx.ring_mesh.render(ctx, point, *squid::HANDLE_SIZE, &ctx.color_scheme.foreground);
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
    use matrix::reach_inside_mat4;

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

fn do_click_context_menu(app: &mut App, button: MouseButton, mouse_position: &glm::Vec2) -> Capture {
    if let Some(context_menu) = &app.context_menu {
        // Get context menu action
        let action = context_menu.click(button, mouse_position);

        // Destroy context menu
        app.context_menu = None;

        match action {
            Some(ContextAction::DeleteSelected) => app.delete_selected(),
            Some(ContextAction::DuplicateSelected) => app.duplicate_selected(),
            Some(ContextAction::GrabSelected) => app.grab_selected(),
            Some(ContextAction::RotateSelected) => app.rotate_selected(),
            Some(ContextAction::ScaleSelected) => app.scale_selected(),
            Some(ContextAction::Collectively) => app.toggle_next_operation_collectively(),
            None => return Capture::Miss,
        }

        Capture::NoDrag
    } else {
        Capture::Miss
    }
}

fn do_click(
    app: &mut App,
    tools: &mut SlotMap<ToolKey, Tool>,
    options_tabs: &mut SlotMap<options::tab::TabKey, Box<dyn options::tab::Tab>>,
    button: MouseButton,
) -> Capture {
    // Returns whether a drag is allowed to start

    app.mouse_buttons_held.insert(button);

    use bool_poll::BoolPoll;

    if app.wait_for_stop_drag.poll() {
        app.dragging = None;
        app.operation = None;
        return Capture::NoDrag;
    }

    let position = app.mouse_position.unwrap();
    let position = glm::vec2(position.x, position.y);
    let [width, height]: [f32; 2] = app.dimensions.into();

    // Context Menu
    do_click_context_menu(app, button, &position)?;

    let interaction = Interaction::Click(ClickInteraction { button, position });

    // Tool options ribbon
    if let Some(tool_key) = app.toolbox.get_selected() {
        tools[tool_key].interact_options(interaction, app)?;
    }

    // Tool ribbon
    app.toolbox.click(interaction, width, height)?;

    if button == MouseButton::Left && position.x > width - 256.0 {
        if let Some(current_tab) = options_tabs.get_mut(app.toolbox.get_current_options_tab_key()) {
            return current_tab.interact(interaction, app);
        }
    }

    if let Some(tool_key) = app.toolbox.get_selected() {
        tools[tool_key].interact(interaction, app)?;
    }

    Capture::Miss
}

fn do_mouse_release(app: &mut App, button: MouseButton) {
    let position = app.mouse_position.unwrap();
    let position = glm::vec2(position.x, position.y);
    let animated_camera = app.camera.get_animated();
    let unordered_squids: Vec<SquidRef> = app.ocean.get_squids_unordered().collect();

    app.mouse_buttons_held.remove(&button);

    for reference in unordered_squids {
        if let Some(squid) = app.ocean.get_mut(reference) {
            squid.interact(
                &Interaction::MouseRelease(MouseReleaseInteraction { position, button }),
                &animated_camera,
                &app.interaction_options,
            );
        }
    }

    app.toolbox.mouse_release(button);

    // Primitive history
    app.add_history_marker();
}

fn do_drag(app: &mut App, tools: &mut SlotMap<ToolKey, Tool>) -> Capture {
    let drag = app.dragging.as_ref().unwrap().to_interaction();
    let [width, _]: [f32; 2] = app.dimensions.into();

    app.toolbox.drag(MouseButton::Left, &drag, width)?;

    // Redirect middle mouse button to pan tool
    if app.mouse_buttons_held.contains(&MouseButton::Middle) {
        if let Some(pan_tool) = find_tool(tools, ToolKind::Pan) {
            pan_tool.interact(drag, app)?;
        }
    }

    if let Some(tool_key) = app.toolbox.get_selected() {
        tools[tool_key].interact(drag, app)?;
    }

    Capture::Miss
}

pub fn on_keyboard_input(app: &mut App, tools: &mut SlotMap<ToolKey, Tool>, input: glium::glutin::event::KeyboardInput) {
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

fn on_mouse_input(
    app: &mut App,
    tools: &mut SlotMap<ToolKey, Tool>,
    options_tabs: &mut SlotMap<options::tab::TabKey, Box<dyn options::tab::Tab>>,
    state: ElementState,
    button: MouseButton,
) {
    if state == ElementState::Pressed {
        match do_click(app, tools, options_tabs, button) {
            Capture::NoDrag => (),
            capture => {
                app.dragging = Some(Dragging::new(app.mouse_position.unwrap_or_default()));
                app.handle_captured(&capture, &app.camera.get_animated());
            }
        }
    } else {
        do_mouse_release(app, button);

        if !app.wait_for_stop_drag {
            app.dragging = None;
        }
    }
}

fn on_mouse_move(app: &mut App, tools: &mut SlotMap<ToolKey, Tool>, position: glium::glutin::dpi::PhysicalPosition<f64>) {
    app.mouse_position = Some(position.to_logical(app.scale_factor));

    if let Some(dragging) = app.dragging.as_mut() {
        let logical_position = app.mouse_position.unwrap();
        dragging.update(glm::vec2(logical_position.x, logical_position.y));

        let capture = do_drag(app, tools);
        app.handle_captured(&capture, &app.camera.get_animated());
    }
}

fn on_scroll(app: &mut App, scroll: MouseScrollDelta) {
    if let MouseScrollDelta::PixelDelta(logical_pixel_delta) = scroll {
        app.scroll(&glm::vec2(logical_pixel_delta.x as f32, logical_pixel_delta.y as f32));
    }
}
