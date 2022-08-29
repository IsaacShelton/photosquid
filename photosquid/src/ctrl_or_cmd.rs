use glium::glutin::event::ModifiersState;

pub trait CtrlOrCmd {
    fn ctrl_or_cmd(&self) -> bool;
}

impl CtrlOrCmd for ModifiersState {
    fn ctrl_or_cmd(&self) -> bool {
        if cfg!(target_os = "macos") {
            self.logo()
        } else {
            self.ctrl()
        }
    }
}
