use crate::constants::{METEORA_PROGRAM_KEY,CONFIG, BONDING_CURVE, QUOTE_MINT, GLOBAL, TOKEN_VAULT_SEED, TEST_INITIAL_METEORA_TOKEN_RESERVES};
use crate::state::{bondingcurve::*, meteora::get_pool_create_ix_data};
use crate::{errors::ContractError, state::config::*};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang::solana_program::{instruction::Instruction, system_instruction};
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use std::str::FromStr;

#[derive(Accounts)]
pub struct InitializePoolWithConfig<'info> {
    #[account(
        seeds = [CONFIG.as_bytes()],
        bump,
    )]
    global_config: Box<Account<'info, Config>>,

    //  team wallet
    /// CHECK: should be same with the address in the global_config
    #[account(
        mut,
        constraint = global_config.team_wallet == team_wallet.key() @ContractError::IncorrectAuthority
    )]
    pub team_wallet: AccountInfo<'info>,

    pub token_mint: Box<Account<'info, Mint>>,
  
    #[account(
        mut,
        seeds = [BONDING_CURVE.as_bytes(), &token_mint.key().to_bytes()], 
        bump
    )]
    bonding_curve: Account<'info, BondingCurve>,

    #[account(mut)]
    /// CHECK: Pool account (PDA address)
    pub pool: UncheckedAccount<'info>,

    /// CHECK: Config for fee
    pub config: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: lp mint
    pub lp_mint: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Token A LP
    pub a_vault_lp: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Token A LP
    pub b_vault_lp: UncheckedAccount<'info>,

    /// CHECK: Token A mint
    pub token_a_mint: UncheckedAccount<'info>,

    #[account(mut)]
    pub token_b_mint: Box<Account<'info, Mint>>,

    #[account(mut)]
    /// CHECK: Vault accounts for token A
    pub a_vault: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Vault accounts for token B
    pub b_vault: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [TOKEN_VAULT_SEED.as_bytes(), a_vault.key.as_ref()],
        bump,
        seeds::program = vault_program.key()
    )]
    pub a_token_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [TOKEN_VAULT_SEED.as_bytes(), b_vault.key.as_ref()],
        bump,
        seeds::program = vault_program.key()
    )]
    pub b_token_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    /// CHECK: Vault LP accounts and mints for token A
    pub a_vault_lp_mint: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Vault LP accounts and mints for token B
    pub b_vault_lp_mint: UncheckedAccount<'info>,

    /// CHECK: global vault pda which stores SOL
    #[account(
        mut,
        seeds = [GLOBAL.as_bytes()],
        bump,
    )]
    pub global_vault: AccountInfo<'info>,

    /// CHECK: ata of global vault
    #[account(
        mut,
        seeds = [
            global_vault.key().as_ref(),
            anchor_spl::token::spl_token::ID.as_ref(),
            token_mint.key().as_ref(),
        ],
        bump,
        seeds::program = anchor_spl::associated_token::ID
    )]
    global_token_account: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: Accounts to bootstrap the pool with initial liquidity
    pub payer_token_a: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Accounts to bootstrap the pool with initial liquidity
    pub payer_token_b: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Accounts to bootstrap the pool with initial liquidity
    pub payer_pool_lp: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Protocol fee token a accounts
    pub protocol_token_a_fee: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Protocol fee token b accounts
    pub protocol_token_b_fee: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    /// CHECK: LP mint metadata PDA. Metaplex do the checking.
    pub mint_metadata: UncheckedAccount<'info>,
    /// CHECK: Additional program accounts
    pub rent: UncheckedAccount<'info>,
    /// CHECK: Metadata program account
    pub metadata_program: UncheckedAccount<'info>,
    /// CHECK: Vault program account
    pub vault_program: UncheckedAccount<'info>,
    /// CHECK: Token program account
    pub token_program: Program<'info, Token>,
    /// CHECK: Associated token program account
    pub associated_token_program: UncheckedAccount<'info>,
    /// CHECK: System program account
    system_program: Program<'info, System>,
    ///CHECK: Event Authority account
    pub event_authority: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Meteora Program
    pub meteora_program: AccountInfo<'info>,
}

pub fn initialize_pool_with_config(ctx: Context<InitializePoolWithConfig>) -> Result<()> {
    let quote_mint: Pubkey = Pubkey::from_str(QUOTE_MINT).unwrap();

    // msg!("initialize_pool_with_config start");

    require!(
        ctx.accounts.bonding_curve.token_mint.key() == ctx.accounts.token_b_mint.key(),
        ContractError::NotBondingCurveMint
    );

    require!(
        quote_mint.key() == ctx.accounts.token_a_mint.key(),
        ContractError::NotSOL
    );

    require!(
        ctx.accounts.bonding_curve.is_completed,
        ContractError::NotCompleted
    );

    require!(
        ctx.accounts.meteora_program.key() == Pubkey::from_str(METEORA_PROGRAM_KEY).unwrap(),
        ContractError::InvalidMeteoraProgram
    );

    // msg!("current real_sol_reserves: {}", ctx.accounts.bonding_curve.real_sol_reserves);
    // msg!("current real_token_reserves: {}", ctx.accounts.bonding_curve.real_token_reserves);
   
    let token_a_amount = ctx
        .accounts
        .bonding_curve
        .real_sol_reserves
        .checked_sub(2_000_000_000)
        .ok_or(ContractError::ArithmeticError)?
        .checked_sub(40_000_000)
        .ok_or(ContractError::ArithmeticError)?;

    // msg!("Token A Amount: {}", token_a_amount);

    let token_b_amount = TEST_INITIAL_METEORA_TOKEN_RESERVES;

    // msg!("Token B Amount: {}", token_b_amount);

    let signer_seeds: &[&[&[u8]]] = &[&[
        GLOBAL.as_bytes(),
        &[ctx.bumps.global_vault],
    ]];

    // Transfer Mint B to payer token b - Bonding Curve is Signer
    let cpi_accounts = Transfer {
        from: ctx.accounts.global_token_account.to_account_info(),
        to: ctx.accounts.payer_token_b.to_account_info(),
        authority: ctx.accounts.global_vault.to_account_info(),
    };

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        ),
        token_b_amount,
    )?;

    // Transfer and wrap sol to payer token a - Sol Escrow is Signer
    // Transfer
    let sol_ix = system_instruction::transfer(
        &ctx.accounts.global_vault.to_account_info().key,
        &ctx.accounts.payer_token_a.to_account_info().key,
        token_a_amount,
    );

    invoke_signed(
        &sol_ix,
        &[
            ctx.accounts
                .global_vault
                .to_account_info()
                .clone(),
            ctx.accounts.payer_token_a.to_account_info().clone(),
            ctx.accounts.system_program.to_account_info(),
        ],
        signer_seeds,
    )?;

    // Sync Native mint ATA
    let cpi_accounts = token::SyncNative {
        account: ctx.accounts.payer_token_a.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
    token::sync_native(cpi_ctx)?;


    // Create pool
    let mut accounts = vec![
        AccountMeta::new(ctx.accounts.pool.key(), false),
        AccountMeta::new_readonly(ctx.accounts.config.key(), false),
        AccountMeta::new(ctx.accounts.lp_mint.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_a_mint.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_b_mint.key(), false),
        AccountMeta::new(ctx.accounts.a_vault.key(), false),
        AccountMeta::new(ctx.accounts.b_vault.key(), false),
        AccountMeta::new(ctx.accounts.a_token_vault.key(), false),
        AccountMeta::new(ctx.accounts.b_token_vault.key(), false),
        AccountMeta::new(ctx.accounts.a_vault_lp_mint.key(), false),
        AccountMeta::new(ctx.accounts.b_vault_lp_mint.key(), false),
        AccountMeta::new(ctx.accounts.a_vault_lp.key(), false),
        AccountMeta::new(ctx.accounts.b_vault_lp.key(), false),
        AccountMeta::new(ctx.accounts.payer_token_a.key(), false),
        AccountMeta::new(ctx.accounts.payer_token_b.key(), false),
        AccountMeta::new(ctx.accounts.payer_pool_lp.key(), false),
        AccountMeta::new(ctx.accounts.protocol_token_a_fee.key(), false),
        AccountMeta::new(ctx.accounts.protocol_token_b_fee.key(), false),
        AccountMeta::new(ctx.accounts.payer.key(), true), 
        AccountMeta::new_readonly(ctx.accounts.rent.key(), false),
        AccountMeta::new(ctx.accounts.mint_metadata.key(), false),
        AccountMeta::new_readonly(ctx.accounts.metadata_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.vault_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.associated_token_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
    ];

    accounts.extend(ctx.remaining_accounts.iter().map(|acc| AccountMeta {
        pubkey: *acc.key,
        is_signer: false,
        is_writable: true,
    }));

    let data = get_pool_create_ix_data(token_a_amount, token_b_amount);

    let instruction = Instruction {
        program_id: ctx.accounts.meteora_program.key(),
        accounts,
        data,
    };

    invoke_signed(
        &instruction,
        &[
            ctx.accounts.pool.to_account_info(),
            ctx.accounts.config.to_account_info(),
            ctx.accounts.lp_mint.to_account_info(),
            ctx.accounts.token_a_mint.to_account_info(),
            ctx.accounts.token_b_mint.to_account_info(),
            ctx.accounts.a_vault.to_account_info(),
            ctx.accounts.b_vault.to_account_info(),
            ctx.accounts.a_token_vault.to_account_info(),
            ctx.accounts.b_token_vault.to_account_info(),
            ctx.accounts.a_vault_lp_mint.to_account_info(),
            ctx.accounts.b_vault_lp_mint.to_account_info(),
            ctx.accounts.a_vault_lp.to_account_info(),
            ctx.accounts.b_vault_lp.to_account_info(),
            ctx.accounts.payer_token_a.to_account_info(),
            ctx.accounts.payer_token_b.to_account_info(),
            ctx.accounts.payer_pool_lp.to_account_info(),
            ctx.accounts.protocol_token_a_fee.to_account_info(),
            ctx.accounts.protocol_token_b_fee.to_account_info(),
            ctx.accounts.payer.to_account_info(), 
            ctx.accounts.rent.to_account_info(),
            ctx.accounts.mint_metadata.to_account_info(),
            ctx.accounts.metadata_program.to_account_info(),
            ctx.accounts.vault_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.associated_token_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.event_authority.to_account_info(),
        ],
        signer_seeds, // Signer is the SOL Escrow
    )?;


    // Fee transfer
    let sol_ix = system_instruction::transfer(
        &ctx.accounts.global_vault.to_account_info().key,
        &ctx.accounts.team_wallet.to_account_info().key,
        2_000_000_000,
    );

    invoke_signed(
        &sol_ix,
        &[
            ctx.accounts
                .global_vault
                .to_account_info()
                .clone(),
            ctx.accounts.team_wallet.clone(),
            ctx.accounts.system_program.to_account_info(),
        ],
        signer_seeds,
    )?;

    Ok(())
}

