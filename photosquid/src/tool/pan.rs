use crate::{
    app::App,
    camera::EasySmoothCamera,
    capture::Capture,
    interaction::{DragInteraction, Interaction},
    user_input::UserInput,
};

pub fn interact(_user_inputs: &mut [UserInput], interaction: Interaction, app: &mut App) -> Capture {
    match interaction {
        Interaction::Drag(DragInteraction { delta, .. }) => {
            // Get the "real" camera, which is unaffected by animation
            let real_camera = app.camera.get_real();

            // Apply reverse camera transformation to the drag vector in order to
            // bring it into world space and then move the real camera position
            // in the opposite direction (as if it was being physically dragged)
            let new_camera_location = real_camera.position - real_camera.apply_reverse_to_vector(&delta);

            app.camera.set_location(new_camera_location);
            Capture::AllowDrag
        }
        Interaction::Click { .. } => Capture::AllowDrag,
        _ => Capture::Miss,
    }
}
