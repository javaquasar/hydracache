//! Invocation-local index of freshly read status bytes, never an observation cache.
#[derive(Clone, Copy)]
pub(super) enum Name {
    Pid,
    Tgid,
    Uid,
    Gid,
    Groups,
}
pub(super) struct Fields<'a> {
    first: [Option<&'a str>; 5],
    duplicate: [bool; 5],
}
impl<'a> Fields<'a> {
    pub(super) fn new(text: &'a str) -> Self {
        let mut result = Self {
            first: [None; 5],
            duplicate: [false; 5],
        };
        for line in text.lines() {
            let field = match line.as_bytes().first() {
                Some(b'P') => line.strip_prefix("Pid:").map(|v| (Name::Pid, v)),
                Some(b'T') => line.strip_prefix("Tgid:").map(|v| (Name::Tgid, v)),
                Some(b'U') => line.strip_prefix("Uid:").map(|v| (Name::Uid, v)),
                Some(b'G') => line
                    .strip_prefix("Gid:")
                    .map(|v| (Name::Gid, v))
                    .or_else(|| line.strip_prefix("Groups:").map(|v| (Name::Groups, v))),
                _ => None,
            };
            if let Some((name, body)) = field {
                let slot = name as usize;
                if result.first[slot].is_some() {
                    result.duplicate[slot] = true;
                } else {
                    result.first[slot] = Some(body);
                }
            }
        }
        result
    }
    pub(super) fn get(&self, name: Name) -> (Option<&'a str>, bool) {
        (self.first[name as usize], self.duplicate[name as usize])
    }
}
