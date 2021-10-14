mod interaction;
mod pan;
mod pointer;
mod rect;

use crate::app::ApplicationState;
use slotmap::new_key_type;

pub use interaction::{Capture, Interaction};
pub use pan::Pan;
pub use pointer::Pointer;
pub use rect::Rect;

new_key_type! { pub struct ToolKey; }

pub trait Tool {
    fn interact(&self, interaction: Interaction, app: &mut ApplicationState) -> Capture;
}
