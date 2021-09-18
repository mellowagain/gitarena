use std::fmt::Display;

use enum_display_derive::Display;

#[derive(Display, Debug)]
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
