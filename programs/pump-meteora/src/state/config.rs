use crate::errors::*;
use anchor_lang::{prelude::*, AnchorDeserialize, AnchorSerialize};
use core::fmt::Debug;


#[account]
pub struct Config {
    pub authority: Pubkey,
    //  use this for 2 step ownership transfer
    pub pending_authority: Pubkey,

    pub team_wallet: Pubkey,

    pub init_bonding_curve: f64, // bonding curve init percentage. The remaining amount is sent to team wallet for distribution to agent

    pub platform_buy_fee: f64, //  platform fee percentage
    pub platform_sell_fee: f64,
    pub platform_migration_fee: f64,

    pub curve_limit: u64, //  lamports to complete te bonding curve

    pub lamport_amount_config: AmountConfig<u64>,
    pub token_supply_config: AmountConfig<u64>,
    pub token_decimals_config: AmountConfig<u8>,

    pub initialized: bool,
    pub global_authority: Pubkey, // can update settings

}


#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub enum AmountConfig<T: PartialEq + PartialOrd + Debug> {
    Range { min: Option<T>, max: Option<T> },
    Enum(Vec<T>),
}

impl<T: PartialEq + PartialOrd + Debug> AmountConfig<T> {
    pub fn validate(&self, value: &T) -> Result<()> {
        match self {
            Self::Range { min, max } => {
                if let Some(min) = min {
                    if value < min {
                        msg!("value {value:?} too small, expected at least {min:?}");
                        return Err(ValueTooSmall.into());
                    }
                }
                if let Some(max) = max {
                    if value > max {
                        msg!("value {value:?} too large, expected at most {max:?}");
                        return Err(ValueTooLarge.into());
                    }
                }

                Ok(())
            }
            Self::Enum(options) => {
                if options.contains(value) {
                    Ok(())
                } else {
                    msg!("invalid value {value:?}, expected one of: {options:?}");
                    Err(ValueInvalid.into())
                }
            }
        }
    }
}
