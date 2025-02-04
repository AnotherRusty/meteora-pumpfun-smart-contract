use anchor_lang::{prelude::*, AnchorDeserialize, AnchorSerialize};
use crate::{state::{config::*}};
use crate::constants::*;
use crate::errors::*;
use crate::events::CompleteEvent;
use crate::utils::*;
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

    // pub reserve_lamport: u64,
    // pub reserve_token: u64,

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
    // Updates the token reserves in the liquidity pool
    // fn update_reserves(
    //     &mut self,
    //     global_config: &Account<'info, Config>,
    //     reserve_one: u64,
    //     reserve_two: u64,
    // ) -> Result<bool>;

    fn swap(
        &mut self,
        global_config: &Account<'info, Config>,
        token_mint: &Account<'info, Mint>,
        global_ata: &mut AccountInfo<'info>,
        user_ata: &mut AccountInfo<'info>,
        source: &mut AccountInfo<'info>,
        team_wallet: &mut AccountInfo<'info>,
        team_wallet_ata: &mut AccountInfo<'info>,
        amount: u64,
        direction: u8,
        minimum_receive_amount: u64,

        user: &Signer<'info>,
        signer: &[&[&[u8]]],

        token_program: &Program<'info, Token>,
        system_program: &Program<'info, System>,
    ) -> Result<u64>;

    // fn simulate_swap(
    //     &self,
    //     global_config: &Account<'info, Config>,
    //     token_mint: &Account<'info, Mint>,
    //     amount: u64,
    //     direction: u8,
    // ) -> Result<u64>;

    // fn cal_amount_out(
    //     &self,
    //     amount: u64,
    //     token_one_decimals: u8,
    //     direction: u8,
    //     platform_sell_fee: f64,
    //     platform_buy_fee: f64,
    // ) -> Result<(u64, u64)>;

    fn apply_sell(&mut self, token_amount: u64) -> Option<SellResult>;

    fn apply_buy(&mut self, sol_amount: u64) -> Option<BuyResult>;

    fn get_sol_for_sell_tokens(&self, token_amount: u64) -> Option<u64>;

    fn get_tokens_for_buy_sol(&self, sol_amount: u64) -> Option<u64>;

}

impl<'info> BondingCurveAccount<'info> for Account<'info, BondingCurve> {
    // fn update_reserves(
    //     &mut self,
    //     global_config: &Account<'info, Config>,
    //     reserve_token: u64,
    //     reserve_lamport: u64,
    // ) -> Result<bool> {
    //     self.reserve_token = reserve_token;
    //     self.reserve_lamport = reserve_lamport;

    //     if reserve_lamport >= global_config.curve_limit {
    //         msg!("curve is completed");
    //         self.is_completed = true;
    //         return Ok(true);
    //     }

    //     Ok(false)
    // }

    fn swap(
        &mut self,
        global_config: &Account<'info, Config>,

        token_mint: &Account<'info, Mint>,
        global_ata: &mut AccountInfo<'info>,
        user_ata: &mut AccountInfo<'info>,

        source: &mut AccountInfo<'info>,
        team_wallet: &mut AccountInfo<'info>,
        team_wallet_ata: &mut AccountInfo<'info>,

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

        msg!(
            "Swap started. direction: {}, AmountIn: {}, MinOutAmount: {}",
            direction,
            amount,
            minimum_receive_amount
        );

        let amount_out;

        if direction == 1{

        }
        else {

        }
        if direction == 1{ //Sell tokens
           let sell_result = self.apply_sell(amount).ok_or(ContractError::SellFailed)?;

           msg!("SellResult: {:#?}", sell_result);

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
            msg! {"fee: {:?}", fee_amount}

            sol_transfer_with_signer(
                source.clone(),
                team_wallet.clone(),
                &system_program,
                signer,
                fee_amount,
            )?;

            amount_out = sell_result.token_amount;

        }
        else   //buy tokens
        {
            let adjusted_amount_in_float = convert_to_float(amount, 9)
            .div(100_f64)
            .mul(100_f64.sub(global_config.platform_sell_fee));

            let adjusted_amount = convert_from_float(adjusted_amount_in_float, 9);

            let buy_result = self.apply_buy(adjusted_amount).ok_or(ContractError::BuyFailed)?;

            msg!("BuyResult: {:#?}", buy_result);

            if self.is_completed == true {
                emit!(CompleteEvent {
                    user: user.key(),
                    mint: token_mint.key(),
                    bonding_curve: self.key()
                });
            }

            msg!("token_transfer_with_signer start");

            token_transfer_with_signer(
                global_ata.clone(),
                source.clone(),
                user_ata.clone(),
                &token_program,
                signer,
                buy_result.token_amount,
            )?;
            msg!("sol_transfer_from_user start");
            sol_transfer_from_user(&user, source.clone(), &system_program, buy_result.sol_amount)?;

             //  transfer fee to team wallet
             let fee_amount = amount - adjusted_amount;
             msg! {"fee: {:?}", fee_amount}
             
             msg!("sol_transfer_from_user start");
             sol_transfer_from_user(&user, team_wallet.clone(), &system_program, fee_amount)?;
             amount_out = buy_result.sol_amount;
        }

        // if side = buy, amount to swap = min(amount, remaining reserve)
        // let amount = if direction == 1 {
        //     amount
        // } else {
        //     amount.min(global_config.curve_limit - self.reserve_lamport)
        // };

        // msg!("Mint: {:?} ", token_mint.key());
        // msg!("Swap: {:?} {:?} {:?}", user.key(), direction, amount);

        
        // let (adjusted_amount, amount_out) = self.cal_amount_out(
        //     amount,
        //     token_mint.decimals,
        //     direction,
        //     global_config.platform_sell_fee,
        //     global_config.platform_buy_fee,
        // )?;

        // if amount_out < minimum_receive_amount {
        //     return Err(ContractError::ReturnAmountTooSmall.into());
        // }

        // if direction == 1 {
        //     let new_reserves_one = self
        //         .reserve_token
        //         .checked_add(amount)
        //         .ok_or(ContractError::OverflowOrUnderflowOccurred)?;

        //     let new_reserves_two = self
        //         .reserve_lamport
        //         .checked_sub(amount_out)
        //         .ok_or(ContractError::OverflowOrUnderflowOccurred)?;

        //     self.update_reserves(global_config, new_reserves_one, new_reserves_two)?;

        //     msg! {"Reserves: {:?} {:?}", new_reserves_one, new_reserves_two};

        //     token_transfer_user(
        //         user_ata.clone(),
        //         &user,
        //         global_ata.clone(),
        //         &token_program,
        //         adjusted_amount,
        //     )?;

        //     sol_transfer_with_signer(
        //         source.clone(),
        //         user.to_account_info(),
        //         &system_program,
        //         signer,
        //         amount_out,
        //     )?;

        //     //  transfer fee to team wallet
        //     let fee_amount = amount - adjusted_amount;

        //     msg! {"fee: {:?}", fee_amount}

        //     token_transfer_user(
        //         user_ata.clone(),
        //         &user,
        //         team_wallet_ata.clone(),
        //         &token_program,
        //         fee_amount,
        //     )?;
        // } else {
        //     let new_reserves_one = self
        //         .reserve_token
        //         .checked_sub(amount_out)
        //         .ok_or(ContractError::OverflowOrUnderflowOccurred)?;

        //     let new_reserves_two = self
        //         .reserve_lamport
        //         .checked_add(amount)
        //         .ok_or(ContractError::OverflowOrUnderflowOccurred)?;

        //     let is_completed =
        //         self.update_reserves(global_config, new_reserves_one, new_reserves_two)?;

        //     if is_completed == true {
        //         emit!(CompleteEvent {
        //             user: user.key(),
        //             mint: token_mint.key(),
        //             bonding_curve: self.key()
        //         });
        //     }

        //     msg! {"Reserves: {:?} {:?}", new_reserves_one, new_reserves_two};

        //     token_transfer_with_signer(
        //         global_ata.clone(),
        //         source.clone(),
        //         user_ata.clone(),
        //         &token_program,
        //         signer,
        //         amount_out,
        //     )?;

        //     sol_transfer_from_user(&user, source.clone(), &system_program, amount)?;

        //     //  transfer fee to team wallet
        //     let fee_amount = amount - adjusted_amount;
        //     msg! {"fee: {:?}", fee_amount}

        //     sol_transfer_from_user(&user, team_wallet.clone(), &system_program, fee_amount)?;
        // }
        Ok(amount_out)
    }

   
    fn get_sol_for_sell_tokens(&self, token_amount: u64) -> Option<u64> {
        if token_amount == 0 {
            return None;
        }
        msg!("GetSolForSellTokens: token_amount: {}", token_amount);

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

        msg!("GetSolForSellTokens: sol_out: {}", sol_out);
        <u128 as TryInto<u64>>::try_into(sol_out).ok()
    }

    fn get_tokens_for_buy_sol(&self, sol_amount: u64) -> Option<u64> {
        if sol_amount == 0 {
            return None;
        }
        msg!("GetTokensForBuySol: sol_amount: {},
        self.virtual_sol_reserves: {},
        self.virtual_token_reserves: {}", sol_amount, self.virtual_sol_reserves, self.virtual_token_reserves);

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

        msg!("GetTokensForBuySol: tokens_out: {}", tokens_out);
        <u128 as TryInto<u64>>::try_into(tokens_out).ok()
    }


    fn apply_buy(&mut self, mut sol_amount: u64) -> Option<BuyResult> {
        msg!("ApplyBuy: sol_amount: {}", sol_amount);

        // Computing Token Amount out
        let mut token_amount = self.get_tokens_for_buy_sol(sol_amount)?;
        msg!("ApplyBuy: token_amount: {}", token_amount);

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
            self.virtual_sol_reserves = 115_005_359_056; // Total raise amount at end

            let recomputed_sol_amount = self.get_sol_for_sell_tokens(token_amount)?;
            msg!("ApplyBuy: recomputed_sol_amount: {}", recomputed_sol_amount);
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
        msg!(
            "ApplyBuy: new_virtual_token_reserves: {}",
            new_virtual_token_reserves
        );

        // New Real Token Reserves
        let new_real_token_reserves =
            (self.real_token_reserves as u128).checked_sub(token_amount as u128)?;
        msg!(
            "ApplyBuy: new_real_token_reserves: {}",
            new_real_token_reserves
        );

        // Adjusting sol reserve values
        // New Virtual Sol Reserves
        let new_virtual_sol_reserves =
            (self.virtual_sol_reserves as u128).checked_add(sol_amount as u128)?;
        msg!(
            "ApplyBuy: new_virtual_sol_reserves: {}",
            new_virtual_sol_reserves
        );

        // New Real Sol Reserves
        let new_real_sol_reserves =
            (self.real_sol_reserves as u128).checked_add(sol_amount as u128)?;
        msg!("ApplyBuy: new_real_sol_reserves: {}", new_real_sol_reserves);

        self.virtual_token_reserves = new_virtual_token_reserves.try_into().ok()?;
        self.real_token_reserves = new_real_token_reserves.try_into().ok()?;
        self.virtual_sol_reserves = new_virtual_sol_reserves.try_into().ok()?;
        self.real_sol_reserves = new_real_sol_reserves.try_into().ok()?;

        msg!(
            "virtual_token_reserves: {:?},
            real_token_reserves: {:?},
            virtual_sol_reserves: {:?},
            real_sol_reserves: {:?}",
            self.virtual_token_reserves,
            self.real_token_reserves,
            self.virtual_sol_reserves,
            self.real_sol_reserves
        );

        msg!(
            "BuyResult:: token_amount: {:?},
             sol_amount: {:?}",
             token_amount,
             sol_amount
        );

        Some(BuyResult {
            token_amount,
            sol_amount,
        })
    }


    fn apply_sell(&mut self, token_amount: u64) -> Option<SellResult> {
        msg!("apply_sell: token_amount: {}", token_amount);

        // Computing Sol Amount out
        let sol_amount = self.get_sol_for_sell_tokens(token_amount)?;
        msg!("apply_sell: sol_amount: {}", sol_amount);

        // Adjusting token reserve values
        // New Virtual Token Reserves
        let new_virtual_token_reserves =
            (self.virtual_token_reserves as u128).checked_add(token_amount as u128)?;
        msg!(
            "apply_sell: new_virtual_token_reserves: {}",
            new_virtual_token_reserves
        );

        // New Real Token Reserves
        let new_real_token_reserves =
            (self.real_token_reserves as u128).checked_add(token_amount as u128)?;
        msg!(
            "apply_sell: new_real_token_reserves: {}",
            new_real_token_reserves
        );

        // Adjusting sol reserve values
        // New Virtual Sol Reserves
        let new_virtual_sol_reserves =
            (self.virtual_sol_reserves as u128).checked_sub(sol_amount as u128)?;
        msg!(
            "apply_sell: new_virtual_sol_reserves: {}",
            new_virtual_sol_reserves
        );

        // New Real Sol Reserves
        let new_real_sol_reserves = self.real_sol_reserves.checked_sub(sol_amount)?;
        msg!(
            "apply_sell: new_real_sol_reserves: {}",
            new_real_sol_reserves
        );

        self.virtual_token_reserves = new_virtual_token_reserves.try_into().ok()?;
        self.real_token_reserves = new_real_token_reserves.try_into().ok()?;
        self.virtual_sol_reserves = new_virtual_sol_reserves.try_into().ok()?;
        self.real_sol_reserves = new_real_sol_reserves.try_into().ok()?;
        
        msg!(
            "virtual_token_reserves: {:?},
            real_token_reserves: {:?},
            virtual_sol_reserves: {:?},
            real_sol_reserves: {:?}",
            self.virtual_token_reserves,
            self.real_token_reserves,
            self.virtual_sol_reserves,
            self.real_sol_reserves
        );

        msg!(
            "SellResult:: token_amount: {:?},
             sol_amount: {:?}",
             token_amount,
             sol_amount
        );

        Some(SellResult {
            token_amount,
            sol_amount,
        })
    }

    // fn simulate_swap(
    //     &self,
    //     global_config: &Account<'info, Config>,
    //     token_mint: &Account<'info, Mint>,
    //     amount: u64,
    //     direction: u8,
    // ) -> Result<u64> {
    //     if amount <= 0 {
    //         return err!(ContractError::InvalidAmount);
    //     }

    //     Ok(self
    //         .cal_amount_out(
    //             amount,
    //             token_mint.decimals,
    //             direction,
    //             global_config.platform_sell_fee,
    //             global_config.platform_buy_fee,
    //         )?
    //         .1)
    // }

    // fn cal_amount_out(
    //     &self,
    //     amount: u64,
    //     token_one_decimals: u8,
    //     direction: u8,
    //     platform_sell_fee: f64,
    //     platform_buy_fee: f64,
    // ) -> Result<(u64, u64)> {
        
    //     let fee_percent = if direction == 1 {
    //         platform_sell_fee
    //     } else {
    //         platform_buy_fee
    //     };

    //     let adjusted_amount_in_float = convert_to_float(amount, token_one_decimals)
    //         .div(100_f64)
    //         .mul(100_f64.sub(fee_percent));

    //     let adjusted_amount = convert_from_float(adjusted_amount_in_float, token_one_decimals);

    //     let amount_out: u64;

    //     // sell
    //     if direction == 1 {
    //         // sell, token for sel

    //         let x_i = self.reserve_lamport as f64 / LAMPORTS_PER_SOL;  // Convert to SOL
    //         let initial_tokens = CONSTANT / (VIRTUAL_SOL + x_i);
    //         let final_tokens = initial_tokens - adjusted_amount as f64;

    //         if final_tokens <= 0.0 {
    //             return Err(ContractError::InsufficientTokens.into());
    //         }

    //         let x_f = CONSTANT / final_tokens - VIRTUAL_SOL;
    //         let sol_received = (x_i - x_f) * LAMPORTS_PER_SOL; // Convert SOL to lamports

    //         if sol_received < 1.0 {
    //             return Err(ContractError::InsufficientSol.into());
    //         }

    //         amount_out = sol_received as u64;

    //         msg!(
    //             "User sold {} tokens for {} lamports ({} SOL)",
    //             adjusted_amount, amount_out, sol_received / LAMPORTS_PER_SOL
    //         );
    //     } else {
    //         // buy, sol for token

    //         let x_i = self.reserve_lamport as f64 / LAMPORTS_PER_SOL;  // Convert to SOL
    //         let x_f = x_i + (adjusted_amount as f64 / LAMPORTS_PER_SOL); // New SOL reserve

    //         let initial_tokens = CONSTANT / x_i;
    //         let final_tokens = CONSTANT / x_f;
            
    //         msg!("token amount initial_tokens{} reserve_token {} final_token {} final_token_u64 {} x_f {}.", initial_tokens, self.reserve_token, final_tokens, final_tokens as u64, x_f);
    //         let tokens_received = self.reserve_token - final_tokens as u64;
    //         amount_out = tokens_received as u64;

    //         msg!(
    //             "User bought {} tokens for {} lamports ({} SOL)",
    //             amount_out, adjusted_amount, adjusted_amount as f64 / LAMPORTS_PER_SOL
    //         );
    //     }
    //     Ok((adjusted_amount, amount_out))
    // }
}
