pub mod git; // 1xxx

#[repr(usize)]
pub enum PacketCategory {
    Git = 1000
}
