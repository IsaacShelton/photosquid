use super::{Capture, Interaction, Tool};
use crate::app::ApplicationState;

pub struct Pan {}

impl Pan {
    pub fn new() -> Box<dyn Tool> {
        Box::new(Self {})
    }
}

impl Tool for Pan {
    fn interact(&self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        if let Interaction::Drag { delta, .. } = interaction {
            app.camera.set(app.camera.get_real() + delta);
        }
        Capture::AllowDrag
    }
}
