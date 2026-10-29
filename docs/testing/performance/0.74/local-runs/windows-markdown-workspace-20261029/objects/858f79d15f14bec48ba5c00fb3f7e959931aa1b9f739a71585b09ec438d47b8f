//! Storage-only prototype; a borrow is not admission authority.
use super::*;

pub(super) struct Scratch<'a> {
    owner: &'a Leaf,
    buffer: Option<Vec<u8>>,
}
impl<'a> Scratch<'a> {
    pub(super) fn new(owner: &'a Leaf) -> Self {
        Self {
            owner,
            buffer: None,
        }
    }
    pub(super) fn owner(&self) -> &Leaf {
        self.owner
    }
    pub(super) fn cached_capacity(&self) -> Option<usize> {
        self.buffer.as_ref().map(Vec::capacity)
    }
    pub(super) fn revalidate(
        &mut self,
        dir: &File,
        uid: u32,
        gid: u32,
        budget: usize,
    ) -> Result<(), AuthorityError> {
        let result = self.validate(dir, uid, gid, budget);
        if result.is_err() {
            self.buffer = None;
        }
        result
    }
    fn validate(
        &mut self,
        dir: &File,
        uid: u32,
        gid: u32,
        budget: usize,
    ) -> Result<(), AuthorityError> {
        if Stamp::of(&self.owner.file.metadata()?) != self.owner.stamp {
            return Err(AuthorityError::Invalid);
        }
        // Keep the fresh File (tuple field zero) alive through both comparisons.
        let named = Self::read(dir, &self.owner.name, uid, gid, budget, &mut self.buffer)?;
        if named.1 != self.owner.stamp || named.2 != self.owner.raw {
            return Err(AuthorityError::Invalid);
        }
        if named.2.capacity() == named.1.len as usize && named.2.capacity() <= budget {
            self.buffer = Some(named.2);
        }
        Ok(())
    }
    fn read(
        dir: &File,
        name: &str,
        uid: u32,
        gid: u32,
        budget: usize,
        slot: &mut Option<Vec<u8>>,
    ) -> Result<(File, Stamp, Vec<u8>), AuthorityError> {
        let mut file = open_at(dir, name, false)?;
        let before = file.metadata()?;
        regular(&before, uid, gid, 0o400)?;
        if before.dev() != dir.metadata()?.dev() || before.len() > budget as u64 {
            return Err(AuthorityError::Budget);
        }
        let stamp = Stamp::of(&before);
        let mut raw = match slot.take() {
            Some(mut raw) if raw.capacity() == before.len() as usize => {
                raw.clear();
                raw
            }
            _ => Vec::with_capacity(before.len() as usize),
        };
        (&mut file).take(budget as u64 + 1).read_to_end(&mut raw)?;
        if Stamp::of(&file.metadata()?) != stamp || raw.len() as u64 != stamp.len {
            return Err(AuthorityError::Invalid);
        }
        Ok((file, stamp, raw))
    }
}
