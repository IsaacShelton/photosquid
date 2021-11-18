pub trait BoolPoll {
    fn poll(&mut self) -> Self;
}

impl BoolPoll for bool {
    fn poll(&mut self) -> Self {
        if *self {
            *self = false;
            true
        } else {
            false
        }
    }
}
