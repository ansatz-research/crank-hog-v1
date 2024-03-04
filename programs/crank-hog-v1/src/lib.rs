use anchor_lang::prelude::*;
use anchor_spl::{
    metadata::{create_metadata_accounts_v3, CreateMetadataAccountsV3, Metadata},
    token::{self, Mint, MintTo, Token, TokenAccount},
};
use mpl_token_metadata::{pda::find_metadata_account, state::DataV2};
use solana_program::pubkey;
use anchor_spl::associated_token::AssociatedToken;
use anchor_lang::solana_program::sysvar::instructions;
use std::str::FromStr;
use std::cmp::max;
use num_integer::Roots;
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

const ADMIN_PUBKEY: Pubkey = pubkey!("HfFi634cyurmVVDr9frwu4MjGLJzz9XbAJz981HdVaNz");

pub fn reward_amount(now: i64, since_last: i64) -> Result<u64> {
    let robin_bday_ts: i64 = 1724876400;
    let multiplier: u64 = ((max(1, robin_bday_ts - now)) as u64).nth_root(2_u32);
    let amount: u64 = ((now - since_last).max(0) as u64).saturating_mul(multiplier);
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
    pub underlying_token_mint: Pubkey,
    /// The vault's storage account for deposited funds.
    pub underlying_token_account: Pubkey,
    pub pda_bump: u8,

}

// done in a macro instead of function bcuz lifetimes
macro_rules! generate_vault_seeds {
    ($vault:expr) => {{
        &[
            b"hog_vault",
            $vault.settlement_authority.as_ref(),
            $vault.underlying_token_mint.as_ref(),
            &$vault.nonce.to_le_bytes(),
            &[$vault.pda_bump],
        ]
    }};
}


#[program]
pub mod crank_hog_v1 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>,
        settlement_authority: Pubkey,
        nonce: u64
    ) -> Result<()> {
        let vault = &mut ctx.accounts.vault;

        vault.underlying_token_mint = ctx.accounts.underlying_token_mint.key();
        vault.nonce = nonce;
        vault.underlying_token_account = ctx.accounts.vault_underlying_token_account.key();
        vault.pda_bump = *ctx.bumps.get("vault").unwrap();

        Ok(())
    }

    // Create new token mint with PDA as mint authority
    pub fn create_mint(
        ctx: Context<CreateMint>,
        uri: String,
        name: String,
        symbol: String,
    ) -> Result<()> {
        // PDA seeds and bump to "sign" for CPI
        let seeds = b"reward";
        let bump = *ctx.bumps.get("reward_token_mint").unwrap();
        let signer: &[&[&[u8]]] = &[&[seeds, &[bump]]];

        // On-chain token metadata for the mint
        let data_v2 = DataV2 {
            name: name,
            symbol: symbol,
            uri: uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };

        // CPI Context
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_metadata_program.to_account_info(),
            CreateMetadataAccountsV3 {
                metadata: ctx.accounts.metadata_account.to_account_info(), // the metadata account being created
                mint: ctx.accounts.reward_token_mint.to_account_info(), // the mint account of the metadata account
                mint_authority: ctx.accounts.reward_token_mint.to_account_info(), // the mint authority of the mint account
                update_authority: ctx.accounts.reward_token_mint.to_account_info(), // the update authority of the metadata account
                payer: ctx.accounts.admin.to_account_info(), // the payer for creating the metadata account
                system_program: ctx.accounts.system_program.to_account_info(), // the system program account
                rent: ctx.accounts.rent.to_account_info(), // the rent sysvar account
            },
            signer,
        );

        create_metadata_accounts_v3(
            cpi_ctx, // cpi context
            data_v2, // token metadata
            true,    // is_mutable
            true,    // update_authority_is_signer
            None,    // collection details
        )?;

        Ok(())
    }

    pub fn crank_hog<'info>(ctx: Context<CrankHog>) -> Result<()> {
        let clock = Clock::get()?;
        let ixs = ctx.accounts.instructions.as_ref();
        let mut current_index = instructions::load_current_index_checked(ixs)? as usize;

        if instructions::load_instruction_at_checked(current_index + 1, ixs).is_ok() {
            msg!("crank hog must be last ix");
            return Err(ErrorCode::DisrespectfulCrank.into());
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
                return Err(ErrorCode::DisrespectfulCrank.into());
            }
        }

        let hog_vault = &mut ctx.accounts.vault;
        let amount = reward_amount(clock.unix_timestamp, hog_vault.since_last).unwrap() as u64;
        let seeds = generate_vault_seeds!(hog_vault);
        let signer: &[&[&[u8]]; 1] = &[&seeds[..]];

        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.hog_token_mint.to_account_info(),
                    to: ctx.accounts
                        .user_hog_token_account
                        .to_account_info(),
                    authority: hog_vault.to_account_info(),
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
#[instruction(settlement_authority: Pubkey, nonce: u64)]
pub struct Initialize <'info> {
    #[account(
        init,
        seeds = [b"hog_vault", settlement_authority.key().as_ref(), underlying_token_mint.key().as_ref(), &nonce.to_le_bytes()],
        space = 8 + std::mem::size_of::<HogVault>(),
        bump,
        payer = payer
    )]
    pub vault: Account<'info, HogVault>,
    pub underlying_token_mint: Account<'info, Mint>,
    pub vault_underlying_token_account: Account<'info, TokenAccount>,
    pub sponsor: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateMint<'info> {
    #[account(
        mut,
        address = ADMIN_PUBKEY
    )]
    pub admin: Signer<'info>,

    // The PDA is both the address of the mint account and the mint authority
    #[account(
        init,
        seeds = [b"reward"],
        bump,
        payer = admin,
        mint::decimals = 9,
        mint::authority = reward_token_mint,

    )]
    pub reward_token_mint: Account<'info, Mint>,

    ///CHECK: Using "address" constraint to validate metadata account address
    #[account(
        mut,
        address=find_metadata_account(&reward_token_mint.key()).0
    )]
    pub metadata_account: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct CrankHog<'info> {
    pub vault: Account<'info, HogVault>,

    // TOKEN ACCOUNTS
    pub token_program: Program<'info, Token>,
    #[account(mut)]
    pub hog_token_mint: Account<'info, Mint>,
    #[account(
        mut,
        token::authority = authority,
        token::mint = hog_token_mint
    )]
    pub user_hog_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: fixed instructions sysvar account
    #[account(address = instructions::ID)]
    pub instructions: UncheckedAccount<'info>,
}


#[error_code]
pub enum ErrorCode {
    #[msg("You must crank with respect brother...")]
    DisrespectfulCrank,
}