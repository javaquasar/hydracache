use crate::{Error, MaxDepthExceeded, Result};

// The limits come from the D-Bus specification.
const MAX_STRUCT_DEPTH: u8 = 32;
const MAX_ARRAY_DEPTH: u8 = 32;
const MAX_TOTAL_DEPTH: u8 = 64;

// Represents the current depth of all container being (de)serialized.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ContainerDepths {
    structure: u8,
    array: u8,
    variant: u8,
}

impl ContainerDepths {
    pub fn inc_structure(mut self) -> Result<Self> {
        self.structure += 1;
        self.check()
    }

    pub fn dec_structure(mut self) -> Self {
        self.structure -= 1;
        self
    }

    pub fn inc_array(mut self) -> Result<Self> {
        self.array += 1;
        self.check()
    }

    pub fn dec_array(mut self) -> Self {
        self.array -= 1;
        self
    }

    pub fn inc_variant(mut self) -> Result<Self> {
        self.variant += 1;
        self.check()
    }

    fn check(self) -> Result<Self> {
        if self.structure > MAX_STRUCT_DEPTH {
            return Err(Error::MaxDepthExceeded(MaxDepthExceeded::Structure));
        }

        if self.array > MAX_ARRAY_DEPTH {
            return Err(Error::MaxDepthExceeded(MaxDepthExceeded::Array));
        }

        let total = self.structure + self.array + self.variant;

        if total > MAX_TOTAL_DEPTH {
            return Err(Error::MaxDepthExceeded(MaxDepthExceeded::Container));
        }

        Ok(self)
    }
}
