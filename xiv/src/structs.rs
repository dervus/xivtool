use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Race {
    pub id: u32,
    pub masculine: String,
    pub feminine: String,
    pub rse_m_body: i32,
    pub rse_m_hands: i32,
    pub rse_m_legs: i32,
    pub rse_m_feet: i32,
    pub rse_f_body: i32,
    pub rse_f_hands: i32,
    pub rse_f_legs: i32,
    pub rse_f_feet: i32,
    pub unk1: u8, // expansion?
    pub unk2: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelChara {
    pub id: u32,
    pub kind: u8,
    pub model: u16,
    pub base: u8,
    pub variant: u8,
    pub sqpack: u16,
    pub unk1: u8,
}
