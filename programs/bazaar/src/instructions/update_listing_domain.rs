use crate::{
    error::BazaarError,
    common::{
        listing_domain::{ListingDomain},
    },
    common::constant::{LISTING_DOMAIN, DOMAIN_INDEX},
    program::Bazaar
};

use anchor_lang::{prelude::*, solana_program::program_option::COption};
use crate::common::constant::CURRENT_LISTING_DOMAIN_VERSION;

// admin function

pub fn handle_update_listing_domain(
    ctx: Context<UpdateListingDomain>,
    name: String,
    domain_index: u8,
    args: UpdateListingDomainArgs,
) -> Result<()> {

    // check bp
    require!(args.seller_fee_bp <= 10_000, BazaarError::InvalidBasisPoints);

    let listing_domain = &mut ctx.accounts.listing_domain;
    listing_domain.fee_vault = args.fee_vault;
    listing_domain.seller_fee_bp = args.seller_fee_bp;
    listing_domain.treasury = args.treasury;

    // set the account version
    listing_domain.account_version = CURRENT_LISTING_DOMAIN_VERSION;

    Ok(())
}

#[derive(Accounts)]
#[instruction(
    name: String,
    domain_index: u8,
    args: UpdateListingDomainArgs
)]
pub struct UpdateListingDomain<'info> {

    #[account(mut)]
    pub upgrade_authority: Signer<'info>,                // this must be the upgrade authority (superadmin)

    // from: https://docs.rs/anchor-lang/latest/anchor_lang/accounts/account/struct.Account.html
    // only allow the upgrade authority (deployer) to call this
    #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
    pub program: Program<'info, Bazaar>,

    #[account(constraint = program_data.upgrade_authority_address == Some(upgrade_authority.key()))]
    pub program_data: Account<'info, ProgramData>,

    #[account(
        mut,
        seeds = [LISTING_DOMAIN.as_bytes().as_ref(), name.as_ref(), DOMAIN_INDEX.as_bytes().as_ref(), domain_index.to_le_bytes().as_ref()],
        bump
    )]
    pub listing_domain: Box<Account<'info, ListingDomain>>,

    // pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdateListingDomainArgs {
    pub treasury: Pubkey,
    pub fee_vault: Pubkey,
    pub seller_fee_bp: u16
}
