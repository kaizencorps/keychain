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


pub fn handle_delist<'info>(
    ctx: Context<Delist>,
) -> Result<()> {

    let listing = &mut ctx.accounts.listing;

    // bag: transfer everything to seller
    for i in 0..listing.items.len() {
        let listing_item_token_ai = match i {
            0 => ctx.accounts.item_0_listing_token.to_account_info(),
            1 => {
                require!(ctx.accounts.item_1_listing_token.is_some(), BazaarError::MissingListingItemToken);
                ctx.accounts.item_1_listing_token.as_ref().unwrap().to_account_info()
            },
            2 => {
                require!(ctx.accounts.item_2_listing_token.is_some(), BazaarError::MissingListingItemToken);
                ctx.accounts.item_2_listing_token.as_ref().unwrap().to_account_info()
            },
            _ => unreachable!("Only 3 items are supported"),
        };

        let seller_item_token_ai = match i {
            0 => ctx.accounts.item_0_seller_token.to_account_info(),
            1 => {
                require!(ctx.accounts.item_1_seller_token.is_some(), BazaarError::MissingBuyerItemToken);
                ctx.accounts.item_1_seller_token.as_ref().unwrap().to_account_info()
            },
            2 => {
                require!(ctx.accounts.item_2_seller_token.is_some(), BazaarError::MissingBuyerItemToken);
                ctx.accounts.item_2_seller_token.as_ref().unwrap().to_account_info()
            },
            _ => unreachable!("Only 3 items are supported"),
        };

        let seller_ai = ctx.accounts.seller.to_account_info();
        let token_prog_ai = ctx.accounts.token_program.to_account_info();

        transfer_items_out(listing, listing_item_token_ai, seller_item_token_ai, listing.items[i].quantity, seller_ai, true, token_prog_ai)?;
    }

    Ok(())
}


#[derive(Accounts)]
pub struct Delist<'info> {

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
        constraint = listing.has_item(&item_0.key()),
        has_one = seller_account,
        close = seller
    )]
    pub listing: Box<Account<'info, Listing>>,

    #[account(
        constraint = listing.has_item(&item_0.key()),
    )]
    pub item_0: Box<Account<'info, Mint>>,

    #[account(
        mut,
        token::mint = item_0,
        token::authority = seller
    )]
    pub item_0_seller_token: Box<Account<'info, TokenAccount>>,

    #[account(
        associated_token::mint = item_0,
        associated_token::authority = listing
    )]
    pub item_0_listing_token: Box<Account<'info, TokenAccount>>,

    #[account(
        constraint = listing.has_item(&item_1.key()),
    )]
    pub item_1: Option<Box<Account<'info, Mint>>>,

    #[account(
        mut,
        token::mint = item_1,
        token::authority = seller
    )]
    pub item_1_seller_token: Option<Box<Account<'info, TokenAccount>>>,

    #[account(
        associated_token::mint = item_1,
        associated_token::authority = listing
    )]
    pub item_1_listing_token: Option<Box<Account<'info, TokenAccount>>>,

    #[account(
        constraint = listing.has_item(&item_2.key()),
    )]
    pub item_2: Option<Box<Account<'info, Mint>>>,

    #[account(
        mut,
        token::mint = item_2,
        token::authority = seller
    )]
    pub item_2_seller_token: Option<Box<Account<'info, TokenAccount>>>,

    #[account(
        associated_token::mint = item_2,
        associated_token::authority = listing
    )]
    pub item_2_listing_token: Option<Box<Account<'info, TokenAccount>>>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

