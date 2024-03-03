use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Burn, Mint, MintTo, Token, TokenAccount, Transfer},
};

use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions;
use crate::error::ErrorCode;
use std::str::FromStr;
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

// pub fn reward_amount() {
    
// }


#[program]
pub mod crank_hog_v1 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    pub fn crank_hog<'info>(ctx: Context<'_, '_, '_, 'info, CrankHog<'info>>) -> Result<()> {
        let ixs = ctx.accounts.instructions.as_ref();
        let mut current_index = instructions::load_current_index_checked(ixs)? as usize;

        if instructions::load_instruction_at_checked(current_index + 1, ixs).is_ok() {
            msg!("crank hog must be last ix");
            return Err(ErrorCode::Default.into());
        }

        loop {
            if current_index == 0 {
                break;
            }

            current_index -= 1;

            let ix: Instruction = instructions::load_instruction_at_checked(current_index, ixs)?;

            let compute_program_id =
                Pubkey::from_str("ComputeBudget111111111111111111111111111111").unwrap();
            if ix.program_id != compute_program_id {
                msg!("found ix that is not compute budget {:?}", ix.program_id);
                return Err(ErrorCode::Default.into());
            }
        }

        Ok(())
    }


}

#[derive(Accounts)]
pub struct Initialize {}

#[derive(Accounts)]
pub struct CrankHog<'info> {
    #[account(mut)]
    authority: Signer<'info>,
    /// CHECK: fixed instructions sysvar account
    #[account(address = instructions::ID)]
    pub instructions: UncheckedAccount<'info>,
}