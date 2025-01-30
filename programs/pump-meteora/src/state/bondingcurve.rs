use anchor_lang::{prelude::*, AnchorDeserialize, AnchorSerialize};


#[account]
pub struct BondingCurve {
    pub token_mint: Pubkey,
    pub creator: Pubkey,

    pub init_lamport: u64,

    pub reserve_lamport: u64,
    pub reserve_token: u64,

    pub is_completed: bool,
}
