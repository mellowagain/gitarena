use derive_more::Display;

#[derive(Display, Debug)]
pub(crate) enum Band {
    Data,
    Progress,
    Error,
}

impl Band {
    pub(crate) fn serialize(&self) -> &[u8] {
        match self {
            Band::Data => b"\x01",
            Band::Progress => b"\x02",
            Band::Error => b"\x03",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn bands_serialize_to_their_channel_byte() {
        assert_eq!(Band::Data.serialize(), b"\x01");
        assert_eq!(Band::Progress.serialize(), b"\x02");
        assert_eq!(Band::Error.serialize(), b"\x03");
    }
}
