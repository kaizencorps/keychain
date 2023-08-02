use crate::{
    error::BazaarError,
    common::{
        listing::{Listing, ListingType, ListingItem},
    },
    common::constant::{BAZAAR, CURRENT_LISTING_VERSION, LISTING},
};

use anchor_lang::{prelude::*};
use anchor_spl::associated_token::AssociatedToken;

use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use spl_token::native_mint::ID as NATIVE_MINT;
use crate::common::listing_domain::ListingDomain;
use crate::common::seller::SellerAccount;

use keychain::program::Keychain;
use keychain::account::{CurrentDomain, CurrentKeyChain};
use crate::common::util::transfer_items_out;


pub fn handle_update_listing<'info>(
    ctx: Context<UpdateListing>,
    price: u64
) -> Result<()> {

    let listing = &mut ctx.accounts.listing;
    listing.price = price;

    Ok(())
}


#[derive(Accounts)]
pub struct UpdateListing<'info> {

    #[account(
        constraint = keychain.has_key(&seller.key())
    )]
    pub keychain: Box<Account<'info, CurrentKeyChain>>,

    #[account(mut)]
    pub seller: Signer<'info>,

    #[account(
        mut,
        constraint = seller_account.keychain == keychain.key(),
    )]
    pub seller_account: Box<Account<'info, SellerAccount>>,

    #[account(
        mut,
        has_one = seller_account,
    )]
    pub listing: Box<Account<'info, Listing>>,

}

