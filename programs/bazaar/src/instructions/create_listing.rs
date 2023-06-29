use crate::{
    error::BazaarError,
    common::{
        listing::{Listing, ListingType},
    },
    common::constant::{BAZAAR, CURRENT_LISTING_VERSION, LISTING},
};

use anchor_lang::{prelude::*, solana_program::program_option::COption};

use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use spl_token::native_mint::ID as NATIVE_MINT;
use crate::common::listing_domain::ListingDomain;
use crate::common::seller::SellerAccount;

use keychain::program::Keychain;
use keychain::account::{CurrentDomain, CurrentKeyChain};


pub fn handle_create_listing(
    ctx: Context<CreateListing>,
    args: CreateListingArgs,
) -> Result<()> {

    // this shit from shdw nft standard
    // Check mint account for non-fungibility
    /*
    let incorrect_mint_authority =
        ctx.accounts.asset_mint.mint_authority != COption::Some(ctx.accounts.metadata.key());

    if incorrect_mint_authority {
        return Err(ErrorCode::InvalidMintAuthority.into());
    };

    let some_freeze_authority = ctx.accounts.asset_mint.freeze_authority.is_some();
    if some_freeze_authority {
        return Err(ErrorCode::FreezeAuthorityPresent.into());
    }
    let unitary = (ctx.accounts.asset_mint.supply == 0) & (ctx.accounts.asset_mint.decimals == 0);
    if !unitary {
        return Err(ErrorCode::DivisibleToken.into());
    }
    verbose_msg!("Verified Non-Fungibility");
     */

    // todo: check price

    // increment the seller's listing index
    let seller_account = &mut ctx.accounts.seller_account;
    seller_account.listing_index = seller_account.listing_index.checked_add(1).unwrap();

    // create the listing
    let listing = &mut ctx.accounts.listing;
    listing.account_version = CURRENT_LISTING_VERSION;
    listing.price = args.price;
    listing.currency = ctx.accounts.currency.key();
    listing.bump = *ctx.bumps.get("listing").unwrap();
    listing.listing_type = args.listing_type;

    let proceeds: &Option<AccountInfo> = &ctx.accounts.proceeds;
    let proceeds_token: &Option<Account<TokenAccount>> = &ctx.accounts.proceeds_token;

    if listing.currency == NATIVE_MINT {
        // then the sale token isn't needed, but a regular accountinfo should've been specified (wallet)
        require!(proceeds.is_some(), BazaarError::ProceedsAccountNotSpecified);
        listing.proceeds = proceeds.as_ref().unwrap().key();
    } else {
        // then the sale token is needed, but an accountinfo shouldn't have been specified (wallet)
        require!(proceeds_token.is_some(), BazaarError::ProceedsTokenAccountNotSpecified);
        listing.proceeds = proceeds_token.as_ref().unwrap().key();
    }

    // the items are specified in the remaining accounts
    /*
    for acc in ctx.remaining_accounts.iter() {
        accounts.push(AccountMeta::new_readonly(acc.key(), false));
        account_infos.push(acc.to_account_info());
    }
     */

    Ok(())
}

#[derive(Accounts)]
#[instruction(
    args: CreateListingArgs
)]
pub struct CreateListing<'info> {

    // todo: specify keychain for listing

    #[account(mut)]
    pub listing_domain: Box<Account<'info, ListingDomain>>,

    #[account(mut)]
    pub seller: Signer<'info>,

    #[account(
        mut,
        constraint = seller_account.keychain == keychain.key(),
    )]
    pub seller_account: Box<Account<'info, SellerAccount>>,

    #[account(
        constraint = keychain.has_key(&seller.key())
    )]
    pub keychain: Box<Account<'info, CurrentKeyChain>>,

    #[account(
        init,
        payer = seller,
        seeds = [LISTING.as_bytes().as_ref(), seller_account.key().as_ref(), &seller_account.listing_index.checked_add(1).unwrap().to_le_bytes()],
        bump,
        space = 8 + Listing::MAX_SIZE,
    )]
    pub listing: Box<Account<'info, Listing>>,

    // the currency the listing is being sold for - native mint for straight sol
    #[account()]
    pub currency: Account<'info, Mint>,

    // the token account to deposit the proceeds into - necessary if currency is spl
    #[account(
        token::mint = currency,
    )]
    pub proceeds_token: Option<Account<'info, TokenAccount>>,

    /// CHECK: this is only specified if the currency is native (will usually just be the seller, but can be any account to send proceeds to)
    #[account()]
    pub proceeds: Option<AccountInfo<'info>>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CreateListingArgs {
    pub price: u64,
    pub listing_type: ListingType,
}
