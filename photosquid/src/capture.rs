use nalgebra_glm as glm;
use std::ops::{ControlFlow, FromResidual, Try};

#[derive(Debug, PartialEq)]
pub enum Capture {
    Miss,
    AllowDrag,
    NoDrag,
    TakeFocus,
    Keyboard(KeyCapture),
    MoveSelectedSquids { delta: glm::Vec2 },
    RotateSelectedSquids { delta_theta: f32 },
    ScaleSelectedSquids { total_scale_factor: f32 },
    SpreadSelectedSquids { current: glm::Vec2 },
    RevolveSelectedSquids { current: glm::Vec2 },
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
