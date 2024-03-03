use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Burn, Mint, MintTo, Token, TokenAccount, Transfer},
};

use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions;
use crate::error::ErrorCode;
use std::str::FromStr;
use std::cmp::max;
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub fn reward_amount(now: i64, since_last: i64) -> Result<i64> {
    let robin_bday_ts: i64 = 1724876400;
    let multiplier: i64 = (max(1, robin_bday_ts - now)).nth_root(2);
    let amount: i64 = (now - since_last).saturating_mul(multiplier);
    Ok(amount)
}

#[account]
pub struct HogVault {
    pub since_last: i64,
    /// The account that can either finalize the vault to make conditional tokens
    /// redeemable for underlying tokens or revert the vault to make deposit
    /// slips redeemable for underlying tokens.
    pub settlement_authority: Pubkey,
    /// A nonce to allow a single account to be the settlement authority of multiple
    /// vaults with the same underlying token mints.
    pub nonce: u64,
    /// The vault's storage account for deposited funds.
    pub hog_token_mint: Pubkey,
    pub pda_bump: u8,

}

// done in a macro instead of function bcuz lifetimes
macro_rules! generate_vault_seeds {
    ($vault:expr) => {{
        &[
            b"hog_vault",
            &$vault.nonce.to_le_bytes(),
            &[$vault.pda_bump],
        ]
    }};
}


#[program]
pub mod crank_hog_v1 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    pub fn crank_hog<'info>(ctx: Context<'_, '_, '_, 'info, CrankHog<'info>>) -> Result<()> {

        let clock = Clock::get()?;
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

            let ix = instructions::load_instruction_at_checked(current_index, ixs)?;

            let compute_program_id =
                Pubkey::from_str("ComputeBudget111111111111111111111111111111").unwrap();
            if ix.program_id != compute_program_id {
                msg!("found ix that is not compute budget {:?}", ix.program_id);
                return Err(ErrorCode::Default.into());
            }
        }

        let hog_vault = &ctx.accounts.vault;
        let amount = reward_amount(clock.unix_timestamp, hog_vault.since_last);
        let seeds = generate_vault_seeds!(hog_vault);
        let signer: &[&[&[u8]]; 1] = &[&seeds[..]];

        token::mint_to(
            CpiContext::new_with_signer(
                &ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: &ctx.accounts.hog_token_mint.to_account_info(),
                    to: &ctx.accounts
                        .user_hog_token_account
                        .to_account_info(),
                    authority: &ctx.accounts.vault.to_account_info(),
                },
                signer,
            ),
            amount,
        )?;

        hog_vault.since_last = clock.unix_timestamp;
    
        Ok(())
    }


}

#[derive(Accounts)]
pub struct Initialize <'info> {
    #[account(
        init,
        seeds = [b"hogvault"],
        space = HogVault::SIZE,
        bump,
        payer = payer
    )]
    pub hog_vault: AccountLoader<'info, HogVault>,
    pub sponsor: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CrankHog<'info> {
    pub vault: Account<'info, HogVault>,
    token_program: Program<'info, Token>,
    #[account(mut)]
    pub hog_token_mint: Account<'info, Mint>,
    #[account(
        mut,
        token::authority = authority,
        token::mint = hog_token_mint
    )]
    pub user_hog_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    authority: Signer<'info>,
    /// CHECK: fixed instructions sysvar account
    #[account(address = instructions::ID)]
    pub instructions: UncheckedAccount<'info>,
}