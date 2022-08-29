use crate::{
    app::App,
    capture::Capture,
    interaction::{ClickInteraction, Interaction},
    squid::Squid,
    user_input::UserInput,
};
use angular_units::Rad;
use glium::glutin::event::MouseButton;
use nalgebra_glm as glm;

pub fn interact(user_inputs: &mut Vec<UserInput>, interaction: Interaction, app: &mut App) -> Capture {
    match interaction {
        Interaction::Click(ClickInteraction {
            button: MouseButton::Left,
            position: click_coords,
            ..
        }) => {
            let camera = app.camera.get_animated();
            let world_position = camera.apply_reverse(&click_coords);
            let color = app.toolbox.color_picker.calculate_color();

            let rotation = Rad(user_inputs[0].as_text_input_mut().unwrap().text().parse::<f32>().unwrap_or_default() * std::f32::consts::PI / 180.0);

            app.insert(Squid::tri(
                world_position + glm::vec2(0.0, -50.0),
                world_position + glm::vec2(50.0, 50.0),
                world_position + glm::vec2(-50.0, 50.0),
                rotation,
                color,
            ));

            Capture::AllowDrag
        }
        _ => Capture::Miss,
    }
}
