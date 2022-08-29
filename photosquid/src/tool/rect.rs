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
            position,
            ..
        }) => {
            let world_position = app.camera.get_animated().apply_reverse(&position);
            let color = app.toolbox.color_picker.calculate_color();

            let width = user_inputs[0].as_text_input_mut().unwrap().text().parse::<f32>().unwrap_or_default().max(4.0);
            let height = user_inputs[1].as_text_input_mut().unwrap().text().parse::<f32>().unwrap_or_default().max(4.0);
            let rotation = Rad(user_inputs[2].as_text_input_mut().unwrap().text().parse::<f32>().unwrap_or_default() * std::f32::consts::PI / 180.0);
            let radii = user_inputs[3].as_text_input_mut().unwrap().text().parse::<f32>().unwrap_or_default();

            app.insert(Squid::rect(world_position, glm::vec2(width, height), rotation, color, radii));
            Capture::AllowDrag
        }
        _ => Capture::Miss,
    }
}
