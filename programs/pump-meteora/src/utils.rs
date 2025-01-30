use crate::*;
use anchor_spl::token::{self, Token};
use solana_program::program::{invoke, invoke_signed};
use std::ops::{Div, Mul};

pub fn convert_to_float(value: u64, decimals: u8) -> f64 {
    (value as f64).div(f64::powf(10.0, decimals as f64))
}

pub fn convert_from_float(value: f64, decimals: u8) -> u64 {
    value.mul(f64::powf(10.0, decimals as f64)) as u64
}

pub fn sol_transfer_from_user<'info>(
    signer: &Signer<'info>,
    destination: AccountInfo<'info>,
    system_program: &Program<'info, System>,
    amount: u64,
) -> Result<()> {
    let ix = solana_program::system_instruction::transfer(signer.key, destination.key, amount);
    invoke(
        &ix,
        &[
            signer.to_account_info(),
            destination,
            system_program.to_account_info(),
        ],
    )?;
    Ok(())
}