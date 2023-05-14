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
        mut,
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
pub struct ListPNFT<'info> {

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

    // programs
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub associated_token_program: Program<'info, AssociatedToken>,

    // pnft shit

    //can't deserialize directly coz Anchor traits not implemented
    /// CHECK: assert_decode_metadata + seeds below
    #[account(
        mut,
        seeds=[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            mpl_token_metadata::id().as_ref(),
            item.key().as_ref(),
        ],
        seeds::program = mpl_token_metadata::id(),
        bump
    )]
    pub item_metadata: UncheckedAccount<'info>,

    //note that MASTER EDITION and EDITION share the same seeds, and so it's valid to check them here
    /// CHECK: seeds below
    #[account(
    seeds=[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            mpl_token_metadata::id().as_ref(),
            item.key().as_ref(),
            mpl_token_metadata::state::EDITION.as_bytes(),
        ],
        seeds::program = mpl_token_metadata::id(),
        bump
    )]
    pub edition: UncheckedAccount<'info>,

    /// CHECK: seeds below
    #[account(
        mut,
        seeds=[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            mpl_token_metadata::id().as_ref(),
            item.key().as_ref(),
            mpl_token_metadata::state::TOKEN_RECORD_SEED.as_bytes(),
            authority_item_token.key().as_ref()
        ],
        seeds::program = mpl_token_metadata::id(),
        bump
    )]
    pub authority_token_record: UncheckedAccount<'info>,

    /// CHECK: seeds below
    #[account(
        mut,
        seeds=[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            mpl_token_metadata::id().as_ref(),
            item.key().as_ref(),
            mpl_token_metadata::state::TOKEN_RECORD_SEED.as_bytes(),
            listing_item_token.key().as_ref()
        ],
        seeds::program = mpl_token_metadata::id(),
        bump
    )]
    pub listing_token_record: UncheckedAccount<'info>,

    //can't deserialize directly coz Anchor traits not implemented
    /// CHECK: address below
    #[account(address = mpl_token_metadata::id())]
    pub token_metadata_program: UncheckedAccount<'info>,

    //sysvar ixs don't deserialize in anchor
    /// CHECK: address below
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions: UncheckedAccount<'info>,

    /// CHECK: address below
    #[account(address = mpl_token_auth_rules::id())]
    pub authorization_rules_program: UncheckedAccount<'info>,

}


#[derive(Accounts)]
pub struct TransferPNFT<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK:
    pub receiver: AccountInfo<'info>,
    #[account(mut)]
    pub src: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub dest: Box<Account<'info, TokenAccount>>,
    pub nft_mint: Box<Account<'info, Mint>>,
    // misc
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub associated_token_program: Program<'info, AssociatedToken>,

    // pfnt
    //can't deserialize directly coz Anchor traits not implemented
    /// CHECK: assert_decode_metadata + seeds below
    #[account(
    mut,
    seeds=[
        mpl_token_metadata::state::PREFIX.as_bytes(),
        mpl_token_metadata::id().as_ref(),
        nft_mint.key().as_ref(),
    ],
    seeds::program = mpl_token_metadata::id(),
    bump
    )]
    pub nft_metadata: UncheckedAccount<'info>,

    //note that MASTER EDITION and EDITION share the same seeds, and so it's valid to check them here
    /// CHECK: seeds below
    #[account(
    seeds=[
        mpl_token_metadata::state::PREFIX.as_bytes(),
        mpl_token_metadata::id().as_ref(),
        nft_mint.key().as_ref(),
        mpl_token_metadata::state::EDITION.as_bytes(),
    ],
    seeds::program = mpl_token_metadata::id(),
    bump
    )]
    pub edition: UncheckedAccount<'info>,

    /// CHECK: seeds below
    #[account(
    mut,
    seeds=[
        mpl_token_metadata::state::PREFIX.as_bytes(),
        mpl_token_metadata::id().as_ref(),
        nft_mint.key().as_ref(),
        mpl_token_metadata::state::TOKEN_RECORD_SEED.as_bytes(),
        src.key().as_ref()
    ],
    seeds::program = mpl_token_metadata::id(),
    bump
    )]
    pub owner_token_record: UncheckedAccount<'info>,

    /// CHECK: seeds below
    #[account(
    mut,
    seeds=[
        mpl_token_metadata::state::PREFIX.as_bytes(),
        mpl_token_metadata::id().as_ref(),
        nft_mint.key().as_ref(),
        mpl_token_metadata::state::TOKEN_RECORD_SEED.as_bytes(),
        dest.key().as_ref()
    ],
    seeds::program = mpl_token_metadata::id(),
    bump
    )]
    pub dest_token_record: UncheckedAccount<'info>,

    //can't deserialize directly coz Anchor traits not implemented
    /// CHECK: address below
    #[account(address = mpl_token_metadata::id())]
    pub token_metadata_program: UncheckedAccount<'info>,

    //sysvar ixs don't deserialize in anchor
    /// CHECK: address below
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions: UncheckedAccount<'info>,

    /// CHECK: address below
    #[account(address = mpl_token_auth_rules::id())]
    pub authorization_rules_program: UncheckedAccount<'info>,

    // pub mpl_token_auth_rules_program: Program<'info, MetaplexTokenAuthRules>,

}


/*
#[derive(Accounts)]
pub struct PurchasePnft<'info> {

    #[account(
    mut,
    has_one = item,
    constraint = listing.item == item.key() && listing.item_token == listing_item_token.key(),
    close = treasury,
    )]
    pub listing: Box<Account<'info, Listing>>,

    pub item: Box<Account<'info, Mint>>,

    /// CHECK: this will be handled by the metaplex code
    #[account(
    mut,
    // seeds = [METADATA.as_bytes().as_ref(), keychain.name.as_bytes().as_ref(), keychain.domain.as_bytes().as_ref(), YARDSALE.as_bytes().as_ref()],
    )]
    pub item_metadata: Box<AccountInfo<'info>>,

    /// CHECK: this will be handled by the metaplex code
    #[account(
    mut,
    // seeds = [METADATA.as_bytes().as_ref(), keychain.name.as_bytes().as_ref(), keychain.domain.as_bytes().as_ref(), YARDSALE.as_bytes().as_ref()],
    )]
    pub item_edition: Box<AccountInfo<'info>>,

    // the token record account (that pNFTs have)
    /// CHECK: this will be handled by the metaplex code
    #[account(
    mut,
    // seeds = [METADATA.as_bytes().as_ref(), keychain.name.as_bytes().as_ref(), keychain.domain.as_bytes().as_ref(), YARDSALE.as_bytes().as_ref()],
    )]
    pub item_record: Box<AccountInfo<'info>>,

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
    // pub mpl_token_auth_rules_program: Program<'info, MetaplexTokenAuthRules>,
}
*/

#[derive(Accounts)]
pub struct PurchasePNFT<'info> {

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
    pub rent: Sysvar<'info, Rent>,

    // pnft shit

    //can't deserialize directly coz Anchor traits not implemented
    /// CHECK: assert_decode_metadata + seeds below
    #[account(
        mut,
        seeds=[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            mpl_token_metadata::id().as_ref(),
            item.key().as_ref(),
        ],
        seeds::program = mpl_token_metadata::id(),
        bump
    )]
    pub item_metadata: UncheckedAccount<'info>,

    //note that MASTER EDITION and EDITION share the same seeds, and so it's valid to check them here
    /// CHECK: seeds below
    #[account(
        seeds=[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            mpl_token_metadata::id().as_ref(),
            item.key().as_ref(),
            mpl_token_metadata::state::EDITION.as_bytes(),
        ],
        seeds::program = mpl_token_metadata::id(),
        bump
    )]
    pub edition: UncheckedAccount<'info>,

    /// CHECK: seeds below
    #[account(
        mut,
        seeds=[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            mpl_token_metadata::id().as_ref(),
            item.key().as_ref(),
            mpl_token_metadata::state::TOKEN_RECORD_SEED.as_bytes(),
            authority_item_token.key().as_ref()
        ],
        seeds::program = mpl_token_metadata::id(),
        bump
    )]
    pub listing_token_record: UncheckedAccount<'info>,

    /// CHECK: seeds below
    #[account(
        mut,
        seeds=[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            mpl_token_metadata::id().as_ref(),
            item.key().as_ref(),
            mpl_token_metadata::state::TOKEN_RECORD_SEED.as_bytes(),
            listing_item_token.key().as_ref()
        ],
        seeds::program = mpl_token_metadata::id(),
        bump
    )]
    pub authority_token_record: UncheckedAccount<'info>,

    //can't deserialize directly coz Anchor traits not implemented
    /// CHECK: address below
    #[account(address = mpl_token_metadata::id())]
    pub token_metadata_program: UncheckedAccount<'info>,

    //sysvar ixs don't deserialize in anchor
    /// CHECK: address below
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions: UncheckedAccount<'info>,

    /// CHECK: address below
    #[account(address = mpl_token_auth_rules::id())]
    pub authorization_rules_program: UncheckedAccount<'info>,

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


