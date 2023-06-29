use crate::{
    error::BazaarError,
    common::{
        listing::{Listing, ListingType},
        listing_domain::{ListingDomain},
    },
    common::constant::{BAZAAR, LISTING_DOMAIN, CURRENT_LISTING_VERSION, CURRENT_LISTING_DOMAIN_VERSION, DOMAIN_INDEX},
    program::Bazaar
};

use anchor_lang::{prelude::*, solana_program::program_option::COption};

use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use spl_token::native_mint::ID as NATIVE_MINT;


// admin function

pub fn handle_create_listing_domain(
    ctx: Context<CreateListingDomain>,
    args: CreateListingDomainArgs,
) -> Result<()> {

    // we only reserve 32 bytes for the domain name
    let domain_name_bytes = args.name.as_bytes();
    require!(domain_name_bytes.len() <= 32, BazaarError::NameTooLong);

    let mut name = [0u8; 32];
    name[..domain_name_bytes.len()].copy_from_slice(domain_name_bytes);

    let mut listing_domain = &mut ctx.accounts.listing_domain;
    listing_domain.name = name;
    listing_domain.bump = *ctx.bumps.get("listing_domain").unwrap();
    listing_domain.account_version = CURRENT_LISTING_DOMAIN_VERSION;
    listing_domain.domain_index = args.domain_index;

    Ok(())
}

#[derive(Accounts)]
#[instruction(
    args: CreateListingDomainArgs
)]
pub struct CreateListingDomain<'info> {

    #[account(mut)]
    pub upgrade_authority: Signer<'info>,                // this must be the upgrade authority (superadmin)

    // from: https://docs.rs/anchor-lang/latest/anchor_lang/accounts/account/struct.Account.html
    // only allow the upgrade authority (deployer) to call this
    #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
    pub program: Program<'info, Bazaar>,

    #[account(constraint = program_data.upgrade_authority_address == Some(upgrade_authority.key()))]
    pub program_data: Account<'info, ProgramData>,

    #[account(
        init,
        payer = upgrade_authority,
        seeds = [LISTING_DOMAIN.as_bytes().as_ref(), args.name.as_bytes().as_ref(), DOMAIN_INDEX.as_bytes().as_ref(), args.domain_index.to_le_bytes().as_ref()],
        bump,
        space = 8 + ListingDomain::MAX_SIZE,
    )]
    pub listing_domain: Box<Account<'info, ListingDomain>>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CreateListingDomainArgs {
    pub name: String,
    pub domain_index: u8
}
