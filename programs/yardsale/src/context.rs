use anchor_lang::prelude::*;
use crate::account::*;
use crate::constant::*;
use crate::error::*;

use keychain::program::Keychain;
use keychain::account::CurrentKeyChain;

use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct ListItem<'info> {

    #[account(constraint = keychain.has_key(&authority.key()))]
    pub keychain: Box<Account<'info, CurrentKeyChain>>,

    pub item: Box<Account<'info, Mint>>,

    #[account(
        mut,
        token::mint = item,
        token::authority = authority
    )]
    pub from_item_token: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        payer = authority,
        seeds = [item.key().as_ref(), keychain.name.as_bytes().as_ref(), keychain.domain.as_bytes().as_ref(), YARDSALE.as_bytes().as_ref()],
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
    pub item_token: Box<Account<'info, TokenAccount>>,

    // the currency the listing is being sold for - native mint should be acceptable
    pub currency: Account<'info, Mint>,

    // the token account to deposit the proceeds into - necessary if currency is not native
    #[account(
        mut,
        token::mint = currency,
    )]
    pub sale_token: Option<Account<'info, TokenAccount>>,

    /// CHECK: this is only specified if the currency is native (will usually just be the authority, but can be any account to send proceeds to)
    #[account()]
    pub sale_account: Option<AccountInfo<'info>>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program <'info, System>,

    // pub keychain_program: Program<'info, Keychain>,
}


#[derive(Accounts)]
pub struct PurchaseItem<'info> {

    #[account(
        // seeds = [mint.key().as_ref(), keychain.name.as_bytes().as_ref(), keychain.domain.as_bytes().as_ref(), YARDSALE.as_bytes().as_ref()],
        has_one = item,
        constraint = listing.item_token == item_token.key(),
        // bump = listing.bump,
    )]
    pub listing: Box<Account<'info, Listing>>,

    pub item: Box<Account<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = item,
        associated_token::authority = listing
    )]
    pub item_token: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = item,
        token::authority = authority
    )]
    pub to_item_token: Box<Account<'info, TokenAccount>>,

    // the currency the listing is being sold for - optional cause if it's missing then listing is in sol
    pub currency: Option<Account<'info, Mint>>,

    // the token account to deposit the proceeds into
    // also optional cause if currency is in sol then it's not needed
    #[account(
        mut,
        token::mint = currency,
    )]
    pub sale_token: Option<Account<'info, TokenAccount>>,

    // the buyer
    #[account(mut)]
    pub authority: Signer<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    // pub system_program: Program <'info, System>,
}


