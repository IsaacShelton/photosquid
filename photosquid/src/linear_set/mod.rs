/*
    A dumb implementation of Set for items that don't implement Ord
*/

pub struct LinearSet<T> {
    items: Vec<T>,
}

impl<T: PartialEq> LinearSet<T> {
    pub fn new() -> Self {
        LinearSet { items: Vec::new() }
    }

    pub fn insert(&mut self, item: T) -> bool {
        if self.contains(&item) {
            false
        } else {
            self.items.push(item);
            true
        }
    }

    pub fn remove(&mut self, item: &T) -> bool {
        for i in 0..self.items.len() {
            if self.items[i] == *item {
                self.items.remove(i);
                return true;
            }
        }

        false
    }

    pub fn contains(&mut self, item: &T) -> bool {
        self.items.contains(item)
    }
}
