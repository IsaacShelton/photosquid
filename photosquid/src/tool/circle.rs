use crate::{
    app::App,
    capture::Capture,
    interaction::{ClickInteraction, Interaction},
    squid::Squid,
    user_input::UserInput,
};
use glium::glutin::event::MouseButton;

pub fn interact(user_inputs: &mut Vec<UserInput>, interaction: Interaction, app: &mut App) -> Capture {
    match interaction {
        Interaction::Click(ClickInteraction {
            button: MouseButton::Left,
            position,
            ..
        }) => {
            let world_position = app.camera.get_animated().apply_reverse(&position);
            let color = app.toolbox.color_picker.calculate_color();
            let radius = user_inputs[0].as_text_input_mut().unwrap().text().parse::<f32>().unwrap_or_default().max(4.0);

            app.insert(Squid::circle(world_position, radius, color));
            Capture::AllowDrag
        }
        _ => Capture::Miss,
    }
}
