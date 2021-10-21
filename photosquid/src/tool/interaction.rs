use glium::glutin::event::{MouseButton, VirtualKeyCode};
use nalgebra_glm as glm;
use std::ops::{ControlFlow, FromResidual, Try};

pub enum Interaction {
    Click { button: MouseButton, position: glm::Vec2 },
    MouseRelease { button: MouseButton, position: glm::Vec2 },
    Drag { delta: glm::Vec2, start: glm::Vec2, current: glm::Vec2 },
    Key { virtual_keycode: VirtualKeyCode },
}

#[derive(Debug, PartialEq)]
pub enum Capture {
    Miss,
    AllowDrag,
    NoDrag,
    Keyboard(KeyCapture),
    MoveSelectedSquids { delta: glm::Vec2 },
    RotateSelectedSquids { delta_theta: f32 },
}

#[derive(Debug, PartialEq)]
pub enum KeyCapture {
    Capture,
    Miss,
}

impl Try for Capture {
    type Output = Capture;
    type Residual = Capture;

    #[inline]
    fn from_output(output: Self::Output) -> Self {
        output
    }

    #[inline]
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Capture::Miss => ControlFlow::Continue(Capture::Miss),
            _ => ControlFlow::Break(self),
        }
    }
}

impl FromResidual<Capture> for Capture {
    #[inline]
    fn from_residual(residual: Capture) -> Self {
        residual
    }
}
