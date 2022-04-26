use std::io::{Read, Write};
use std::{fmt, fs, mem};
use std::fmt::{Debug, Display, Formatter};

use anyhow::{Context, Result};
use bincode::config::{AllowTrailing, Bounded, LittleEndian, VarintEncoding, WithOtherEndian, WithOtherIntEncoding, WithOtherLimit, WithOtherTrailing};
use bincode::{DefaultOptions, Options as _};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

// World's longest type, thank you
pub type BincodeType = WithOtherTrailing<WithOtherIntEncoding<WithOtherEndian<WithOtherLimit<DefaultOptions, Bounded>, LittleEndian>, VarintEncoding>, AllowTrailing>;

/// [Type-length-value](https://en.wikipedia.org/wiki/Type%E2%80%93length%E2%80%93value) packet to be used for GitArena IPC
#[derive(Deserialize, Serialize)]
pub struct IpcPacket<T: ?Sized> {
    id: u64,
    length: u64,
    data: T
}

impl<T: Serialize + Sized + PacketId> IpcPacket<T> {
    pub fn new(data: T) -> Self {
        let size = Self::bincode().serialized_size(&data).unwrap_or_else(|_| mem::size_of::<T>() as u64);

        IpcPacket {
            id: data.id(),
            length: size,
            data
        }
    }
}

impl<T: Sized> IpcPacket<T> {
    /// Maximum size that this struct can be serialized from (mem::size_of::<Self> + 1 MB)
    #[inline]
    pub const fn max_size() -> u64 {
        // Allow 1 MB additional limit
        mem::size_of::<T>() as u64 + 1_000_000
    }

    #[inline]
    fn bincode() -> BincodeType {
        DefaultOptions::new()
            .with_limit(Self::max_size())
            .with_little_endian()
            .with_varint_encoding()
            .allow_trailing_bytes()
    }
}

impl<T: Serialize> IpcPacket<T> {
    pub fn serialize(&self) -> bincode::Result<Vec<u8>> {
        Self::bincode().serialize(&self)
    }

    pub fn serialize_into<W: Write>(&self, destination: W) -> bincode::Result<()> {
        Self::bincode().serialize_into(destination, &self)
    }

    pub fn bincode_size(&self) -> bincode::Result<u64> {
        Self::bincode().serialized_size(&self)
    }
}

impl<'a, T: Deserialize<'a>> IpcPacket<T> {
    pub fn deserialize(input: &'a [u8]) -> bincode::Result<Self> {
        Self::bincode().deserialize::<Self>(input)
    }
}

impl<T: DeserializeOwned + ?Sized> IpcPacket<T> {
    pub fn deserialize_from<R: Read>(input: R) -> bincode::Result<Self> {
        Self::bincode().deserialize_from::<_, Self>(input)
    }
}

impl<T: Display> Display for IpcPacket<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.data.fmt(f)
    }
}

impl<T: Debug> Debug for IpcPacket<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("IpcPacket")
            .field("id", &self.id)
            .field("length", &self.length)
            .field("data", &self.data)
            .finish()
    }
}

pub trait PacketId {
    fn id(&self) -> u64;
}

/// Cross-platform way to get the socket/pipe path.
///
/// # Side effects
///
/// On *non-Windows systems* this function exhibits side effects (creation of directory `/run/gitarena`)
pub fn ipc_path() -> Result<&'static str> {
    Ok(if cfg!(windows) {
        r"\\.\pipe\gitarena-workhorse"
    } else {
        fs::create_dir_all("/run/gitarena").context("Failed to create directory")?;
        "/run/gitarena/workhorse"
    })
}
