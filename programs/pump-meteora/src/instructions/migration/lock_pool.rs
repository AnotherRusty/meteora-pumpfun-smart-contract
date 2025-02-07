use crate::constants::*;
use crate::errors::ContractError;
use crate::state::bondingcurve::*;
use crate::state::meteora::{get_function_hash, get_lock_lp_ix_data};
use crate::state::config::*;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_instruction;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};
use anchor_spl::{associated_token, token::TokenAccount};
use anchor_spl::token::{self, Mint, Token, Transfer};
use std::str::FromStr;

#[derive(Accounts)]
pub struct LockPool<'info> {
  
    #[account(
        mut,
        seeds = [BONDING_CURVE.as_bytes(), &token_mint.key().to_bytes()], 
        bump
    )]
    bonding_curve: Account<'info, BondingCurve>,

    pub token_mint: Box<Account<'info, Mint>>,

    /// CHECK: global vault pda which stores SOL
    #[account(
        mut,
        seeds = [GLOBAL.as_bytes()],
        bump,
    )]
    pub global_vault: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: Pool account (PDA address)
    pub pool: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: lp mint
    pub lp_mint: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Token A LP
    pub a_vault_lp: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Token A LP
    pub b_vault_lp: UncheckedAccount<'info>,

    /// CHECK: Token B mint
    pub token_b_mint: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Vault accounts for token A
    pub a_vault: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Vault accounts for token B
    pub b_vault: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Vault LP accounts and mints for token A
    pub a_vault_lp_mint: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Vault LP accounts and mints for token B
    pub b_vault_lp_mint: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Accounts to bootstrap the pool with initial liquidity
    pub payer_pool_lp: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    /// CHECK: Fee receiver
    pub fee_receiver: UncheckedAccount<'info>,

    /// CHECK: Token program account
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: Associated token program account
    pub associated_token_program: UncheckedAccount<'info>,
    /// CHECK: System program account
    pub system_program: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK lock escrow
    pub lock_escrow: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK lock escrow
    pub lock_escrow1: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Escrow vault
    pub escrow_vault: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Escrow vault
    pub escrow_vault1: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK:
    pub meteora_program: AccountInfo<'info>,

    /// CHECK: Meteora Event Autority
    pub event_authority: AccountInfo<'info>,
}

pub fn lock_pool(ctx: Context<LockPool>) -> Result<()> {
    
    msg!("Create pool: end {:?}", ctx.accounts.payer_pool_lp);

    require!(
        ctx.accounts.meteora_program.key() == Pubkey::from_str(METEORA_PROGRAM_KEY).unwrap(),
        ContractError::InvalidMeteoraProgram
    );

    let signer_seeds: &[&[&[u8]]] = &[&[
        GLOBAL.as_bytes(),
        &[ctx.bumps.global_vault],
    ]];


    let meteora_program_id: Pubkey = Pubkey::from_str(METEORA_PROGRAM_KEY).unwrap();
    let source_tokens = ctx.accounts.payer_pool_lp.clone();
    let lp_mint_amount = ctx.accounts.payer_pool_lp.amount / 2;

    // Create Lock Escrow
    let escrow_accounts = vec![
        AccountMeta::new(ctx.accounts.pool.key(), false),
        AccountMeta::new(ctx.accounts.lock_escrow.key(), false),
        AccountMeta::new_readonly(ctx.accounts.payer.key(), false),
        AccountMeta::new_readonly(ctx.accounts.lp_mint.key(), false),
        AccountMeta::new(ctx.accounts.payer.key(), true), // Bonding Curve Sol Escrow is the payer/signer
        AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
    ];

    let escrow_accounts1 = vec![
        AccountMeta::new(ctx.accounts.pool.key(), false),
        AccountMeta::new(ctx.accounts.lock_escrow1.key(), false),
        AccountMeta::new_readonly(ctx.accounts.fee_receiver.key(), false),
        AccountMeta::new_readonly(ctx.accounts.lp_mint.key(), false),
        AccountMeta::new(ctx.accounts.payer.key(), true), // Bonding Curve Sol Escrow is the payer/signer
        AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
    ];


    let escrow_instruction = Instruction {
        program_id: meteora_program_id,
        accounts: escrow_accounts,
        data: get_function_hash("global", "create_lock_escrow").into(),
    };

    let escrow_instruction1 = Instruction {
        program_id: meteora_program_id,
        accounts: escrow_accounts1,
        data: get_function_hash("global", "create_lock_escrow").into(),
    };


    invoke_signed(
        &escrow_instruction,
        &[
            ctx.accounts.pool.to_account_info(),
            ctx.accounts.lock_escrow.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.lp_mint.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        signer_seeds,
    )?;

    invoke_signed(
        &escrow_instruction1,
        &[
            ctx.accounts.pool.to_account_info(),
            ctx.accounts.lock_escrow1.to_account_info(),
            ctx.accounts.fee_receiver.to_account_info(),
            ctx.accounts.lp_mint.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        signer_seeds,
    )?;


    associated_token::create_idempotent(CpiContext::new(
        ctx.accounts.associated_token_program.to_account_info(),
        associated_token::Create {
            payer: ctx.accounts.payer.to_account_info(),
            associated_token: ctx.accounts.escrow_vault.to_account_info(),
            authority: ctx.accounts.lock_escrow.to_account_info(),
            mint: ctx.accounts.lp_mint.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        },
    ))?;

    associated_token::create_idempotent(CpiContext::new(
        ctx.accounts.associated_token_program.to_account_info(),
        associated_token::Create {
            payer: ctx.accounts.payer.to_account_info(),
            associated_token: ctx.accounts.escrow_vault1.to_account_info(),
            authority: ctx.accounts.lock_escrow1.to_account_info(),
            mint: ctx.accounts.lp_mint.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        },
    ))?;


    // Lock Pool
    let lock_accounts = vec![
        AccountMeta::new(ctx.accounts.pool.key(), false),
        AccountMeta::new_readonly(ctx.accounts.lp_mint.key(), false),
        AccountMeta::new(ctx.accounts.lock_escrow.key(), false),
        AccountMeta::new(ctx.accounts.payer.key(), true), // Bonding Curve Sol Escrow is the payer/signer
        AccountMeta::new(source_tokens.key(), false),
        AccountMeta::new(ctx.accounts.escrow_vault.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.a_vault.key(), false),
        AccountMeta::new_readonly(ctx.accounts.b_vault.key(), false),
        AccountMeta::new_readonly(ctx.accounts.a_vault_lp.key(), false),
        AccountMeta::new_readonly(ctx.accounts.b_vault_lp.key(), false),
        AccountMeta::new_readonly(ctx.accounts.a_vault_lp_mint.key(), false),
        AccountMeta::new_readonly(ctx.accounts.b_vault_lp_mint.key(), false),
    ];

    let lock_accounts1 = vec![
        AccountMeta::new(ctx.accounts.pool.key(), false),
        AccountMeta::new_readonly(ctx.accounts.lp_mint.key(), false),
        AccountMeta::new(ctx.accounts.lock_escrow1.key(), false),
        AccountMeta::new(ctx.accounts.payer.key(), true), // Bonding Curve Sol Escrow is the payer/signer
        AccountMeta::new(source_tokens.key(), false),
        AccountMeta::new(ctx.accounts.escrow_vault1.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.a_vault.key(), false),
        AccountMeta::new_readonly(ctx.accounts.b_vault.key(), false),
        AccountMeta::new_readonly(ctx.accounts.a_vault_lp.key(), false),
        AccountMeta::new_readonly(ctx.accounts.b_vault_lp.key(), false),
        AccountMeta::new_readonly(ctx.accounts.a_vault_lp_mint.key(), false),
        AccountMeta::new_readonly(ctx.accounts.b_vault_lp_mint.key(), false),
    ];


    let lock_instruction = Instruction {
        program_id: meteora_program_id,
        accounts: lock_accounts,
        data: get_lock_lp_ix_data(lp_mint_amount),
    };

    let lock_instruction1 = Instruction {
        program_id: meteora_program_id,
        accounts: lock_accounts1,
        data: get_lock_lp_ix_data(lp_mint_amount),
    };

    invoke_signed(
        &lock_instruction,
        &[
            ctx.accounts.pool.to_account_info(),
            ctx.accounts.lp_mint.to_account_info(),
            ctx.accounts.lock_escrow.to_account_info(),
            ctx.accounts.payer.to_account_info(), // Bonding Curve Sol Escrow is the payer/signer
            source_tokens.to_account_info(),
            ctx.accounts.escrow_vault.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.a_vault.to_account_info(),
            ctx.accounts.b_vault.to_account_info(),
            ctx.accounts.a_vault_lp.to_account_info(),
            ctx.accounts.b_vault_lp.to_account_info(),
            ctx.accounts.a_vault_lp_mint.to_account_info(),
            ctx.accounts.b_vault_lp_mint.to_account_info(),
        ],
        signer_seeds,
    )?;

    invoke_signed(
        &lock_instruction1,
        &[
            ctx.accounts.pool.to_account_info(),
            ctx.accounts.lp_mint.to_account_info(),
            ctx.accounts.lock_escrow1.to_account_info(),
            ctx.accounts.payer.to_account_info(), // Bonding Curve Sol Escrow is the payer/signer
            source_tokens.to_account_info(),
            ctx.accounts.escrow_vault1.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.a_vault.to_account_info(),
            ctx.accounts.b_vault.to_account_info(),
            ctx.accounts.a_vault_lp.to_account_info(),
            ctx.accounts.b_vault_lp.to_account_info(),
            ctx.accounts.a_vault_lp_mint.to_account_info(),
            ctx.accounts.b_vault_lp_mint.to_account_info(),
        ],
        signer_seeds,
    )?;
    Ok(())
}
