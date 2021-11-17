use crate::squid::SquidRef;

#[derive(Clone)]
pub struct Layer {
    pub name: String,
    pub squids: Vec<SquidRef>,
}

impl Layer {
    pub fn new(name: String) -> Self {
        Self { name, squids: vec![] }
    }

    pub fn add(&mut self, reference: SquidRef) {
        self.squids.insert(0, reference);
    }

    pub fn remove_mention(&mut self, reference: SquidRef) {
        self.squids.retain(|squid_reference| !squid_reference.eq(&reference));
    }

    #[allow(dead_code)]
    pub fn get_lowest(&self) -> impl Iterator<Item = SquidRef> + '_ {
        self.squids.iter().rev().copied()
    }

    #[allow(dead_code)]
    pub fn get_highest(&self) -> impl Iterator<Item = SquidRef> + '_ {
        self.squids.iter().copied()
    }

    #[allow(dead_code)]
    pub fn get_name(&self) -> &str {
        &self.name
    }
}

impl Default for Layer {
    fn default() -> Self {
        Self::new("Default Layer".into())
    }
}
