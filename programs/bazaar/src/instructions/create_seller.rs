use crate::{
    error::BazaarError,
    common::{
        listing::{Listing, ListingType},
        listing_domain::ListingDomain,
        seller::{SellerAccount}
    },
    common::constant::{BAZAAR, CURRENT_LISTING_VERSION, SELLER},
};

use anchor_lang::{prelude::*, solana_program::program_option::COption};

use crate::common::constant::CURRENT_SELLER_VERSION;

use keychain::account::{CurrentDomain, CurrentKeyChain};


// create the seller's account
pub fn handle_create_seller(
    ctx: Context<CreateSeller>,
) -> Result<()> {

    let seller = &mut ctx.accounts.seller_account;

    // first listing will be 1 (checked_add)
    seller.listing_index = 0;
    seller.account_version = CURRENT_SELLER_VERSION;
    seller.bump = *ctx.bumps.get("seller_account").unwrap();
    seller.keychain = ctx.accounts.keychain.key();
    seller.num_sales = 0;

    Ok(())
}

#[derive(Accounts)]
pub struct CreateSeller<'info> {

    #[account(
        constraint = keychain.has_key(&seller.key()),
    )]
    pub keychain: Box<Account<'info, CurrentKeyChain>>,

    #[account(
        init,
        payer = seller,
        seeds = [SELLER.as_bytes().as_ref(), keychain.key().as_ref()],
        bump,
        space = 8 + SellerAccount::MAX_SIZE,
    )]
    pub seller_account: Box<Account<'info, SellerAccount>>,

    #[account(mut)]
    pub seller: Signer<'info>,

    pub system_program: Program<'info, System>,
}

