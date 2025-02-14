use crate::errors::*;
use crate::events::CompleteEvent;
use crate::state::config::*;
use crate::utils::*;
use anchor_lang::{prelude::*, AnchorDeserialize, AnchorSerialize};
use anchor_spl::token::Mint;
use anchor_spl::token::Token;
use std::ops::Div;
use std::ops::Mul;
use std::ops::Sub;

#[account]
pub struct BondingCurve {
    pub token_mint: Pubkey,
    pub creator: Pubkey,

    pub init_lamport: u64,

    pub token_total_supply: u64,

    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,

    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,

    pub is_completed: bool,
}

#[derive(Debug, Clone)]
pub struct SellResult {
    pub token_amount: u64,
    pub sol_amount: u64,
}

#[derive(Debug, Clone)]
pub struct BuyResult {
    pub token_amount: u64,
    pub sol_amount: u64,
}

pub trait BondingCurveAccount<'info> {
    fn swap(
        &mut self,
        global_config: &Account<'info, Config>,
        token_mint: &Account<'info, Mint>,
        global_ata: &mut AccountInfo<'info>,
        user_ata: &mut AccountInfo<'info>,
        source: &mut AccountInfo<'info>,
        team_wallet: &mut AccountInfo<'info>,
        amount: u64,
        direction: u8,
        minimum_receive_amount: u64,

        user: &Signer<'info>,
        signer: &[&[&[u8]]],

        token_program: &Program<'info, Token>,
        system_program: &Program<'info, System>,
    ) -> Result<u64>;

    fn apply_sell(&mut self, token_amount: u64) -> Option<SellResult>;

    fn apply_buy(&mut self, sol_amount: u64) -> Option<BuyResult>;

    fn get_sol_for_sell_tokens(&self, token_amount: u64) -> Option<u64>;

    fn get_tokens_for_buy_sol(&self, sol_amount: u64) -> Option<u64>;
}

impl<'info> BondingCurveAccount<'info> for Account<'info, BondingCurve> {
    fn swap(
        &mut self,
        global_config: &Account<'info, Config>,

        token_mint: &Account<'info, Mint>,
        global_ata: &mut AccountInfo<'info>,
        user_ata: &mut AccountInfo<'info>,

        source: &mut AccountInfo<'info>,
        team_wallet: &mut AccountInfo<'info>,

        amount: u64,
        direction: u8,
        minimum_receive_amount: u64,

        user: &Signer<'info>,
        signer: &[&[&[u8]]],

        token_program: &Program<'info, Token>,
        system_program: &Program<'info, System>,
    ) -> Result<u64> {
        if amount <= 0 {
            return err!(ContractError::InvalidAmount);
        }

        let amount_out;

        if direction == 1 {
            //Sell tokens
            let sell_result = self.apply_sell(amount).ok_or(ContractError::SellFailed)?;

            token_transfer_user(
                user_ata.clone(),
                &user,
                global_ata.clone(),
                &token_program,
                sell_result.token_amount,
            )?;

            let adjusted_amount_in_float = convert_to_float(sell_result.sol_amount, 9)
                .div(100_f64)
                .mul(100_f64.sub(global_config.platform_sell_fee));

            let adjusted_amount = convert_from_float(adjusted_amount_in_float, 9);

            sol_transfer_with_signer(
                source.clone(),
                user.to_account_info(),
                &system_program,
                signer,
                adjusted_amount,
            )?;

            //  transfer fee to team wallet
            let fee_amount = sell_result.sol_amount - adjusted_amount;

            sol_transfer_with_signer(
                source.clone(),
                team_wallet.clone(),
                &system_program,
                signer,
                fee_amount,
            )?;

            amount_out = sell_result.token_amount;
        } else
        //buy tokens
        {
            let adjusted_amount_in_float = convert_to_float(amount, 9)
                .div(100_f64)
                .mul(100_f64.sub(global_config.platform_sell_fee));

            let adjusted_amount = convert_from_float(adjusted_amount_in_float, 9);

            let buy_result = self
                .apply_buy(adjusted_amount)
                .ok_or(ContractError::BuyFailed)?;

            if self.is_completed == true {
                emit!(CompleteEvent {
                    user: user.key(),
                    mint: token_mint.key(),
                    bonding_curve: self.key()
                });
            }

            token_transfer_with_signer(
                global_ata.clone(),
                source.clone(),
                user_ata.clone(),
                &token_program,
                signer,
                buy_result.token_amount,
            )?;

            sol_transfer_from_user(
                &user,
                source.clone(),
                &system_program,
                buy_result.sol_amount,
            )?;

            //  transfer fee to team wallet
            let fee_amount = amount - adjusted_amount;

            sol_transfer_from_user(&user, team_wallet.clone(), &system_program, fee_amount)?;
            amount_out = buy_result.sol_amount;
        }

        Ok(amount_out)
    }

    fn get_sol_for_sell_tokens(&self, token_amount: u64) -> Option<u64> {
        if token_amount == 0 {
            return None;
        }

        // Convert to common decimal basis (using 9 decimals as base)
        let current_sol = self.virtual_sol_reserves as u128;
        let current_tokens = (self.virtual_token_reserves as u128)
            .checked_mul(1_000_000_000)? // Scale tokens up to 9 decimals
            .checked_div(1_000_000)?; // From 6 decimals

        // Calculate new reserves using constant product formula
        let new_tokens = current_tokens.checked_add(
            (token_amount as u128)
                .checked_mul(1_000_000_000)? // Scale input tokens to 9 decimals
                .checked_div(1_000_000)?, // From 6 decimals
        )?;

        let new_sol = (current_sol.checked_mul(current_tokens)?).checked_div(new_tokens)?;

        let sol_out = current_sol.checked_sub(new_sol)?;

        <u128 as TryInto<u64>>::try_into(sol_out).ok()
    }

    fn get_tokens_for_buy_sol(&self, sol_amount: u64) -> Option<u64> {
        if sol_amount == 0 {
            return None;
        }

        // Convert to common decimal basis (using 9 decimals as base)
        let current_sol = self.virtual_sol_reserves as u128;
        let current_tokens = (self.virtual_token_reserves as u128)
            .checked_mul(1_000_000_000)? // Scale tokens up to 9 decimals
            .checked_div(1_000_000)?; // From 6 decimals

        // Calculate new reserves using constant product formula
        let new_sol = current_sol.checked_add(sol_amount as u128)?;
        let new_tokens = (current_sol.checked_mul(current_tokens)?).checked_div(new_sol)?;

        let tokens_out = current_tokens.checked_sub(new_tokens)?;

        // Convert back to 6 decimal places for tokens
        let tokens_out = tokens_out
            .checked_mul(1_000_000)? // Convert to 6 decimals
            .checked_div(1_000_000_000)?; // From 9 decimals

        <u128 as TryInto<u64>>::try_into(tokens_out).ok()
    }

    fn apply_buy(&mut self, mut sol_amount: u64) -> Option<BuyResult> {

        // Computing Token Amount out
        let mut token_amount = self.get_tokens_for_buy_sol(sol_amount)?;

        if token_amount >= self.real_token_reserves {
            // Last Buy
            token_amount = self.real_token_reserves;

            // Temporarily store the current state
            let current_virtual_token_reserves = self.virtual_token_reserves;
            let current_virtual_sol_reserves = self.virtual_sol_reserves;

            // Update self with the new token amount
            self.virtual_token_reserves = (current_virtual_token_reserves as u128)
                .checked_sub(token_amount as u128)?
                .try_into()
                .ok()?;
            self.virtual_sol_reserves = 57_502_679_529; // Total raise amount at end

            let recomputed_sol_amount = self.get_sol_for_sell_tokens(token_amount)?;
            sol_amount = recomputed_sol_amount;

            // Restore the state with the recomputed sol_amount
            self.virtual_token_reserves = current_virtual_token_reserves;
            self.virtual_sol_reserves = current_virtual_sol_reserves;

            // Set complete to true
            self.is_completed = true;
        }

        // Adjusting token reserve values
        // New Virtual Token Reserves
        let new_virtual_token_reserves =
            (self.virtual_token_reserves as u128).checked_sub(token_amount as u128)?;

        // New Real Token Reserves
        let new_real_token_reserves =
            (self.real_token_reserves as u128).checked_sub(token_amount as u128)?;

        // Adjusting sol reserve values
        // New Virtual Sol Reserves
        let new_virtual_sol_reserves =
            (self.virtual_sol_reserves as u128).checked_add(sol_amount as u128)?;

        // New Real Sol Reserves
        let new_real_sol_reserves =
            (self.real_sol_reserves as u128).checked_add(sol_amount as u128)?;

        self.virtual_token_reserves = new_virtual_token_reserves.try_into().ok()?;
        self.real_token_reserves = new_real_token_reserves.try_into().ok()?;
        self.virtual_sol_reserves = new_virtual_sol_reserves.try_into().ok()?;
        self.real_sol_reserves = new_real_sol_reserves.try_into().ok()?;

        Some(BuyResult {
            token_amount,
            sol_amount,
        })
    }

    fn apply_sell(&mut self, token_amount: u64) -> Option<SellResult> {
        // Computing Sol Amount out
        let sol_amount = self.get_sol_for_sell_tokens(token_amount)?;

        // Adjusting token reserve values
        // New Virtual Token Reserves
        let new_virtual_token_reserves =
            (self.virtual_token_reserves as u128).checked_add(token_amount as u128)?;

        // New Real Token Reserves
        let new_real_token_reserves =
            (self.real_token_reserves as u128).checked_add(token_amount as u128)?;

        // Adjusting sol reserve values
        // New Virtual Sol Reserves
        let new_virtual_sol_reserves =
            (self.virtual_sol_reserves as u128).checked_sub(sol_amount as u128)?;

        // New Real Sol Reserves
        let new_real_sol_reserves = self.real_sol_reserves.checked_sub(sol_amount)?;

        self.virtual_token_reserves = new_virtual_token_reserves.try_into().ok()?;
        self.real_token_reserves = new_real_token_reserves.try_into().ok()?;
        self.virtual_sol_reserves = new_virtual_sol_reserves.try_into().ok()?;
        self.real_sol_reserves = new_real_sol_reserves.try_into().ok()?;

        Some(SellResult {
            token_amount,
            sol_amount,
        })
    }
}
