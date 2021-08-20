use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub(crate) enum Band {
    Data,
    Progress,
    Error
}

impl Band {
    pub(crate) fn serialize(&self) -> &[u8] {
        match self {
            Band::Data => b"\x01",
            Band::Progress => b"\x02",
            Band::Error => b"\x03"
        }
    }
}

impl Display for Band {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Band::Data => f.write_str("Data (1)"),
            Band::Progress => f.write_str("Progress (2)"),
            Band::Error => f.write_str("Error (3)")
        }
    }
}
