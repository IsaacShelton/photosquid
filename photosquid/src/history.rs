use crate::ocean::Ocean;

#[derive(Default)]
pub struct History {
    history: Vec<Ocean>,
    time_travel: usize,
}

impl History {
    const MAX_HISTORY: usize = 100;

    pub fn push(&mut self, value: Ocean) {
        if self.history.is_empty() {
            self.history.push(Ocean::default());
        } else {
            while self.time_travel < self.history.len() - 1 {
                self.history.pop();
            }
        }

        while self.history.len() >= Self::MAX_HISTORY {
            self.history.remove(0);
            self.time_travel -= 1;
        }

        self.history.push(value);
        self.time_travel = self.history.len() - 1;
    }

    pub fn undo(&mut self) -> Option<Ocean> {
        if self.time_travel > 0 {
            self.time_travel -= 1;
            Some(self.history[self.time_travel].clone())
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<Ocean> {
        if self.time_travel + 1 < self.history.len() {
            self.time_travel += 1;
            Some(self.history[self.time_travel].clone())
        } else {
            None
        }
    }
}
