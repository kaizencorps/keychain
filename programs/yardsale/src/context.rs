use anchor_lang::prelude::*;
use crate::account::*;
use crate::constant::*;
use crate::error::*;

use keychain::program::Keychain;
use keychain::account::{CurrentDomain, CurrentKeyChain};

use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct ListItem<'info> {

    // todo: pass in the keychain domain as well and pull in fee info ..?
    //      OR: create a domain for yardsale as well ..?

    #[account(
        constraint = domain.name == keychain.domain
    )]
    pub domain: Box<Account<'info, CurrentDomain>>,

    #[account(
        constraint = keychain.has_key(&authority.key()),
    )]
    pub keychain: Box<Account<'info, CurrentKeyChain>>,

    pub item: Box<Account<'info, Mint>>,

    #[account(
        mut,
        token::mint = item,
        token::authority = authority
    )]
    pub authority_item_token: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        payer = authority,
        seeds = [item.key().as_ref(), LISTINGS.as_bytes().as_ref(), keychain.name.as_bytes().as_ref(), keychain.domain.as_bytes().as_ref(), YARDSALE.as_bytes().as_ref()],
        bump,
        space = 8 + Listing::MAX_SIZE,
    )]
    pub listing: Box<Account<'info, Listing>>,

    #[account(
        init,
        payer = authority,
        associated_token::mint = item,
        associated_token::authority = listing
    )]
    pub listing_item_token: Box<Account<'info, TokenAccount>>,

    // the currency the listing is being sold for - native mint should be acceptable
    #[account()]
    pub currency: Account<'info, Mint>,

    // the token account to deposit the proceeds into - necessary if currency is spl
    #[account(
        token::mint = currency,
    )]
    pub proceeds_token: Option<Account<'info, TokenAccount>>,

    /// CHECK: this is only specified if the currency is native (will usually just be the authority, but can be any account to send proceeds to)
    #[account()]
    pub proceeds: Option<AccountInfo<'info>>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program <'info, System>,

    // pub keychain_program: Program<'info, Keychain>,
}

#[derive(Accounts)]
pub struct DelistItem<'info> {

    #[account(
        mut,
        has_one = item,
        constraint = listing.item == item.key() && listing.item_token == listing_item_token.key() && listing.domain == keychain.domain && listing.keychain == keychain.name,
        close = authority,
    )]
    pub listing: Box<Account<'info, Listing>>,

    #[account(
        constraint = keychain.has_key(&authority.key()),
    )]
    pub keychain: Box<Account<'info, CurrentKeyChain>>,

    pub item: Box<Account<'info, Mint>>,

    // the token account the item gets returned to
    #[account(
        token::mint = item,
        token::authority = authority
    )]
    pub authority_item_token: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = item,
        associated_token::authority = listing
    )]
    pub listing_item_token: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program <'info, System>,
}

#[derive(Accounts)]
pub struct UpdatePrice<'info> {

    #[account(
        mut,
        has_one = item,
        constraint = listing.item == item.key() && listing.domain == keychain.domain && listing.keychain == keychain.name,
    )]
    pub listing: Box<Account<'info, Listing>>,

    #[account(
        constraint = keychain.has_key(&authority.key()),
    )]
    pub keychain: Box<Account<'info, CurrentKeyChain>>,

    pub item: Box<Account<'info, Mint>>,

    #[account(mut)]
    pub authority: Signer<'info>,
}


#[derive(Accounts)]
pub struct PurchaseItem<'info> {

    #[account(
        mut,
        has_one = item,
        constraint = listing.item == item.key() && listing.item_token == listing_item_token.key(),
        close = treasury,
    )]
    pub listing: Box<Account<'info, Listing>>,

    pub item: Box<Account<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = item,
        associated_token::authority = listing
    )]
    pub listing_item_token: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = item,
        associated_token::authority = authority
    )]
    pub authority_item_token: Box<Account<'info, TokenAccount>>,

    // the currency the listing is being sold for - optional cause if it's missing then listing is in sol

    #[account(
        constraint = listing.currency == currency.key(),
    )]
    pub currency: Account<'info, Mint>,

    // needed if the currency is spl
    #[account(
        mut,
        token::mint = currency,
        constraint = listing.proceeds == proceeds_token.key(),
    )]
    pub proceeds_token: Option<Account<'info, TokenAccount>>,

    /// CHECK: this is only specified if the currency is native
    #[account(
        mut,
        constraint = listing.proceeds == proceeds.key(),
    )]
    pub proceeds: Option<AccountInfo<'info>>,

    // the buyer
    #[account(mut)]
    pub authority: Signer<'info>,

    // if the currency is spl, then this is the buyer's token account
    #[account(
    mut,
    token::mint = currency,
    token::authority = authority
    )]
    pub authority_currency_token: Option<Account<'info, TokenAccount>>,

    /// CHECK: just sending lamports here when closing the listing
    #[account(
        mut,
        constraint = listing.treasury == treasury.key(),
    )]
    pub treasury: AccountInfo<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program <'info, System>,
}


