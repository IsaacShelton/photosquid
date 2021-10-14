//#![feature(try_trait_v2)]
use glium::glutin::event::MouseButton;
use nalgebra_glm as glm;
use std::ops::{ControlFlow, FromResidual, Try};

pub enum Interaction {
    Click { button: MouseButton, position: glm::Vec2 },
    MouseRelease { button: MouseButton, position: glm::Vec2 },
    Drag { delta: glm::Vec2, start: glm::Vec2, current: glm::Vec2 },
}

#[derive(Debug)]
pub enum Capture {
    Miss,
    AllowDrag,
    NoDrag,
    MoveSelectedSquids { delta: glm::Vec2 },
    RotateSelectedSquids { delta_theta: f32 },
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
