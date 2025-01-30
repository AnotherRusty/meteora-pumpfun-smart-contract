use anchor_lang::prelude::*;

#[event]
pub struct LaunchEvent {
    pub creator: Pubkey,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub metadata: Pubkey,

    pub decimals: u8,
    pub token_supply: u64,

    pub reserve_lamport: u64,
    pub reserve_token: u64,
}
