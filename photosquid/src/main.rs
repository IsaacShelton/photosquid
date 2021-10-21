#![feature(try_trait_v2)]

mod aabb;
mod accumulator;
mod algorithm;
mod app;
mod color;
mod color_impls;
mod color_picker;
mod color_scheme;
mod context_menu;
mod matrix_helpers;
mod mesh;
mod obj_helpers;
mod ocean;
mod press_animation;
mod render_ctx;
mod shader_helpers;
mod smooth;
mod squid;
mod text_helpers;
mod text_input;
mod tool;
mod tool_button;
mod toolbox;
mod vertex;

const TARGET_FPS: u64 = 60;

use app::*;
use color::Color;
use color_scheme::ColorScheme;
use context_menu::ContextAction;
use glium::{
    glutin::{
        event::{ElementState, MouseButton, VirtualKeyCode},
        event::{Event::WindowEvent as AbstractWindowEvent, WindowEvent as ConcreteWindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
        ContextBuilder, GlProfile, GlRequest,
    },
    Display,
};
use glium_text_rusttype as glium_text;
use mesh::{MeshXyz, MeshXyzUv};
use nalgebra_glm as glm;
use ocean::Ocean;
use press_animation::*;
use render_ctx::RenderCtx;
use slotmap::SlotMap;
use smooth::Smooth;
use std::{
    collections::{btree_set::BTreeSet, HashMap},
    rc::Rc,
    time::{Duration, Instant},
};
use tool::{Capture, Interaction};
use tool::{Tool, ToolKey};
use tool_button::ToolButton;
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

    // Build color scheme
    let color_scheme = ColorScheme {
        background: Color::from_hex("#2C2F33FF"),
        light_ribbon: Color::from_hex("#2f3136"),
        dark_ribbon: Color::from_hex("#23272AFF"),
        foreground: Color::from_hex("#7289DA"),
        input: Color::from_hex("#40444B"),
        error: Color::from_hex("#ed2326"),
    };

    // Build toolbox
    let mut toolbox = ToolBox::new(&display);
    let mut tools: SlotMap<ToolKey, Box<dyn Tool>> = SlotMap::with_key();

    // Create tools and corresponding tool buttons
    {
        toolbox.add(ToolButton::new(
            include_str!("_src_objs/pointer.obj"),
            Box::new(DeformPressAnimation {}),
            tools.insert(tool::Pointer::new()),
            &display,
        ));

        toolbox.add(ToolButton::new(
            include_str!("_src_objs/pan.obj"),
            Box::new(DeformPressAnimation {}),
            tools.insert(tool::Pan::new()),
            &display,
        ));

        toolbox.add(ToolButton::new(
            include_str!("_src_objs/rectangle.obj"),
            Box::new(DeformPressAnimation {}),
            tools.insert(tool::Rect::new()),
            &display,
        ));

        toolbox.add(ToolButton::new(
            include_str!("_src_objs/triangle.obj"),
            Box::new(DeformPressAnimation {}),
            tools.insert(tool::Tri::new()),
            &display,
        ));

        // Select first tool
        toolbox.select(0);
    }

    let ribbon_mesh = MeshXyz::new_ui_rect(&display);
    let ring_mesh = MeshXyz::new_ui_ring(&display);
    let square_xyzuv = MeshXyzUv::new_square(&display);

    let color_shader_program = shader_helpers::shader_from_source_that_outputs_srgb(
        &display,
        include_str!("_src_shaders/color/vertex.glsl"),
        include_str!("_src_shaders/color/fragment.glsl"),
        None,
        true,
    )
    .unwrap();
    let hue_value_picker_shader_program = shader_helpers::shader_from_source_that_outputs_srgb(
        &display,
        include_str!("_src_shaders/color_picker/hue_value/vertex.glsl"),
        include_str!("_src_shaders/color_picker/hue_value/fragment.glsl"),
        None,
        true,
    )
    .unwrap();
    let saturation_picker_shader_program = shader_helpers::shader_from_source_that_outputs_srgb(
        &display,
        include_str!("_src_shaders/color_picker/saturation/vertex.glsl"),
        include_str!("_src_shaders/color_picker/saturation/fragment.glsl"),
        None,
        true,
    )
    .unwrap();
    let rounded_rectangle_shader_program = shader_helpers::shader_from_source_that_outputs_srgb(
        &display,
        include_str!("_src_shaders/rounded_rectangle/vertex.glsl"),
        include_str!("_src_shaders/rounded_rectangle/fragment.glsl"),
        None,
        true,
    )
    .unwrap();
    let television_shader_program = shader_helpers::shader_from_source_that_outputs_srgb(
        &display,
        include_str!("_src_shaders/texture/vertex.glsl"),
        include_str!("_src_shaders/texture/fragment.glsl"),
        None,
        false,
    )
    .unwrap();

    let text_system = glium_text::TextSystem::new(&display);

    let font = glium_text::FontTexture::new(
        &display,
        std::fs::File::open(&std::path::Path::new("Roboto-Regular.ttf")).unwrap(),
        20,
        glium_text::FontTexture::ascii_character_list(),
    )
    .unwrap();

    let numeric_mappings: HashMap<VirtualKeyCode, char> = std::iter::FromIterator::from_iter([
        (VirtualKeyCode::Key0, '0'),
        (VirtualKeyCode::Key1, '1'),
        (VirtualKeyCode::Key2, '2'),
        (VirtualKeyCode::Key3, '3'),
        (VirtualKeyCode::Key4, '4'),
        (VirtualKeyCode::Key5, '5'),
        (VirtualKeyCode::Key6, '6'),
        (VirtualKeyCode::Key7, '7'),
        (VirtualKeyCode::Key8, '8'),
        (VirtualKeyCode::Key9, '9'),
        (VirtualKeyCode::Period, '.'),
        (VirtualKeyCode::Minus, '-'),
    ]);

    let television = MeshXyzUv::new_square(&display);
    let scale_factor = display.gl_window().window().scale_factor();

    let mut state = ApplicationState {
        display,
        color_scheme,
        toolbox,
        ribbon_mesh,
        ring_mesh,
        square_xyzuv,
        color_shader_program,
        hue_value_picker_shader_program,
        saturation_picker_shader_program,
        rounded_rectangle_shader_program,
        mouse_position: None,
        scale_factor,
        ocean: Ocean::new(),
        dimensions: None,
        projection: None,
        view: None,
        frame_start_time: Instant::now(),
        camera: Smooth::new(glm::vec2(0.0, 0.0), Duration::from_millis(500)),
        dragging: None,
        selections: vec![],
        keys_held: BTreeSet::new(),
        text_system,
        font: Rc::new(font),
        context_menu: None,
        numeric_mappings,
        interaction_options: InteractionOptions::new(),
    };

    event_loop.run(move |abstract_event, _, control_flow| {
        let framebuffer_dimensions = state.display.get_framebuffer_dimensions();

        state.frame_start_time = Instant::now();
        state.dimensions = Some((
            framebuffer_dimensions.0 as f32 / state.scale_factor as f32,
            framebuffer_dimensions.1 as f32 / state.scale_factor as f32,
        ));

        fn do_click(state: &mut ApplicationState, tools: &mut SlotMap<ToolKey, Box<dyn Tool>>, button: MouseButton) -> Capture {
            // Returns whether a drag is allowed to start

            let position = state.mouse_position.unwrap();
            let position = glm::vec2(position.x, position.y);
            let (width, height) = state.dimensions.unwrap_or_default();

            if let Some(context_menu) = &state.context_menu {
                let action = context_menu.click(button, &position);
                state.context_menu = None;

                if let Some(action) = action {
                    // Handle action
                    match action {
                        ContextAction::DeleteSelected => {
                            state.delete_selected();
                        }
                        ContextAction::DuplicateSelected => {
                            state.duplicate_selected();
                        }
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

            return Capture::Miss;
        }

        fn do_mouse_release(state: &mut ApplicationState, button: MouseButton) {
            let position = state.mouse_position.unwrap();
            let position = glm::vec2(position.x, position.y);
            let animated_camera = state.camera.get_animated();

            for (_, squid) in state.ocean.squids.iter_mut() {
                squid.interact(&Interaction::MouseRelease { position, button }, &animated_camera, &state.interaction_options);
            }

            state.toolbox.mouse_release(button);
        }

        fn do_drag(state: &mut ApplicationState, tools: &mut SlotMap<ToolKey, Box<dyn Tool>>) -> Capture {
            let drag = state.dragging.as_ref().unwrap().to_interaction();
            let (width, _) = state.dimensions.unwrap_or_default();

            state.toolbox.drag(MouseButton::Left, &drag, width)?;

            if let Some(tool_key) = state.toolbox.get_selected() {
                tools[tool_key].interact(drag, state)?;
            }

            Capture::Miss
        }

        // Handle user input
        match abstract_event {
            AbstractWindowEvent { event, .. } => match event {
                ConcreteWindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                ConcreteWindowEvent::KeyboardInput { input, .. } => {
                    if let Some(virtual_keycode) = input.virtual_keycode {
                        let keys_held = &mut state.keys_held;

                        match input.state {
                            ElementState::Pressed => {
                                if keys_held.insert(virtual_keycode) {
                                    // Press first time
                                    state.press_key(&virtual_keycode, &mut tools);
                                } else {
                                    // Ignore / Repeat key
                                }
                            }
                            ElementState::Released => {
                                keys_held.remove(&virtual_keycode);
                            }
                        }
                    }
                }
                ConcreteWindowEvent::MouseInput {
                    state: element_state, button, ..
                } => {
                    if element_state == ElementState::Pressed {
                        match do_click(&mut state, &mut tools, button) {
                            Capture::NoDrag => (),
                            capture => {
                                state.dragging = Some(Dragging::new(state.mouse_position.unwrap_or_default()));
                                state.handle_captured(&capture);
                            }
                        }
                    } else {
                        do_mouse_release(&mut state, button);
                        state.dragging = None;
                    }
                }
                ConcreteWindowEvent::CursorMoved { position, .. } => {
                    state.mouse_position = Some(position.to_logical(state.scale_factor));

                    if let Some(dragging) = state.dragging.as_mut() {
                        let logical_position = state.mouse_position.unwrap();
                        dragging.update(glm::vec2(logical_position.x, logical_position.y));

                        match do_drag(&mut state, &mut tools) {
                            capture => {
                                state.handle_captured(&capture);
                            }
                        }
                    }
                }
                ConcreteWindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    state.scale_factor = scale_factor;
                }
                _ => return,
            },
            glium::glutin::event::Event::RedrawRequested(_window_id) => {
                let (width_u32, height_u32) = state.display.get_framebuffer_dimensions();
                let (width, height) = state.dimensions.unwrap();
                let rendered = glium::texture::SrgbTexture2d::empty(&state.display, width_u32, height_u32).unwrap();
                let mut framebuffer = glium::framebuffer::SimpleFrameBuffer::new(&state.display, &rendered).unwrap();

                state.projection = Some(glm::ortho(0.0, width, height, 0.0, 100.0, -100.0));
                state.view = Some(glm::translation(&glm::vec2_to_vec3(&state.camera.get_animated())));

                let mut target = state.display.draw();

                // TLDR: Don't use 'target' or 'framebuffer' directly unless you really know
                // what you're doing. Instead use the methods of 'RenderCtx' to the appropriate target.
                // NOTE: We won't use 'target' and 'framebuffer' directly most of the time,
                // since which we render to is dependent on the display's scale factor.
                // We do this because on macOS, for some reason, to only way to enable MSAA,
                // is by rendering directly to the buffer. So for retina (and other non-1-factor displays),
                // we will render to a framebuffer first (since more pixels will be sampled anyways),
                // but for displays that have a 1-factor ratio, we will render directly and utilize
                // the built in MSAA for the window render target (this is the only portable way apparently)
                // Render context is a subset of ApplicationState that only
                // contains information related to rendering
                let mut ctx = RenderCtx {
                    target: &mut target,
                    framebuffer: &mut framebuffer,
                    color_shader: &state.color_shader_program,
                    hue_value_picker_shader: &state.hue_value_picker_shader_program,
                    saturation_picker_shader: &state.saturation_picker_shader_program,
                    rounded_rectangle_shader: &state.rounded_rectangle_shader_program,
                    projection: &state.projection.unwrap(),
                    view: &state.view.unwrap(),
                    width: width,
                    height: height,
                    scale_factor: state.scale_factor,
                    ribbon_mesh: &state.ribbon_mesh,
                    ring_mesh: &state.ring_mesh,
                    square_xyzuv: &state.square_xyzuv,
                    color_scheme: &state.color_scheme,
                    camera: &state.camera.get_animated(),
                    display: &state.display,
                };

                // Render components
                {
                    ctx.clear_color(&state.color_scheme.background);

                    for (_, squid) in state.ocean.get_squids_oldest_mut() {
                        squid.render(&mut ctx);
                    }

                    for (reference, squid) in state.ocean.get_squids_oldest_mut() {
                        if selection_contains(&state.selections, reference) {
                            squid.render_selected_indication(&mut ctx);
                        }
                    }

                    state
                        .toolbox
                        .render(&mut ctx, &mut tools, &state.color_scheme, &state.text_system, state.font.clone());

                    if let Some(context_menu) = &mut state.context_menu {
                        context_menu.render(&mut ctx, &state.text_system, state.font.clone());
                    }
                }

                // If we're not doing MSAA, render the framebuffer instead of having just rendered directly.
                // Draw render to window
                if state.scale_factor != 1.0 {
                    use glium::Surface;
                    use matrix_helpers::reach_inside_mat4;

                    let identity = glm::identity::<f32, 4>();

                    let uniforms = glium::uniform! {
                        transformation: reach_inside_mat4(&identity),
                        view: reach_inside_mat4(&identity),
                        projection: reach_inside_mat4(&identity),
                        texture_sampler: &rendered
                    };

                    ctx.target
                        .draw(
                            &television.vertex_buffer,
                            &television.indices,
                            &television_shader_program,
                            &uniforms,
                            &Default::default(),
                        )
                        .unwrap();
                }

                // Finalize frame
                target.finish().unwrap();
            }
            _ => (),
        }

        let (_, height) = state.dimensions.unwrap();

        // Update components
        {
            state.toolbox.update(height);

            if let Some(new_color) = state.toolbox.color_picker.poll() {
                for selection in state.selections.iter() {
                    if selection.limb_id.is_none() {
                        state.ocean.squids[selection.squid_id].set_color(new_color);
                    }
                }
            }
        }

        // Handle control flow
        match *control_flow {
            ControlFlow::Exit => (),
            _ => {
                let elapsed_time = Instant::now().duration_since(state.frame_start_time).as_millis() as u64;

                state.display.gl_window().window().request_redraw();

                let wait_millis = match 100 / TARGET_FPS >= elapsed_time {
                    true => 1000 / TARGET_FPS - elapsed_time,
                    false => 0,
                };
                let next_frame_time = state.frame_start_time + Duration::from_millis(wait_millis);
                *control_flow = ControlFlow::WaitUntil(next_frame_time);
            }
        }
    });
}
