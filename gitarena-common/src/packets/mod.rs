use num_derive::{FromPrimitive, ToPrimitive};

pub mod git; // 1xxx

#[repr(u64)]
pub enum PacketCategory {
    Git = 1000
}

// TODO: Find a way to automatically generate this
// Would be able to do this now if proc macros have state

#[repr(u64)]
#[derive(FromPrimitive, ToPrimitive)]
pub enum PacketId {
    GitImport = 1001
}
