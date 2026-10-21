//! Invocation-local collection scratch, bounded by the caller's take(33).
//! No retained observation, descriptor, authority or clock.
pub(super) struct Values {
    values: [u32; 33],
    len: usize,
}
impl FromIterator<u32> for Values {
    fn from_iter<T: IntoIterator<Item = u32>>(iter: T) -> Self {
        let mut result = Self {
            values: [0; 33],
            len: 0,
        };
        for value in iter {
            result.values[result.len] = value;
            result.len += 1;
        }
        result
    }
}
impl Values {
    pub(super) fn into_vec(self) -> Vec<u32> {
        self.values[..self.len].to_vec()
    }
}

#[cfg(test)]
#[path = "../tests/support/projection_groups.rs"]
mod tests;
