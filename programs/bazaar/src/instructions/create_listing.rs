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


pub fn handle_create_listing<'info>(
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

    // todo: check price ..?

    // increment the seller's listing index
    let seller_account = &mut ctx.accounts.seller_account;
    let listing_index =  seller_account.listing_index.checked_add(1).unwrap();
    seller_account.listing_index = listing_index;

    /////// check our inputs

    // check that the item quantity args are valid
    check_item_quantities(&ctx.accounts, &args.item_quantities)?;

    // if the listing type == unit, then disallow token listing (they can only be part of a bag)
    // units are for SFTs only
    if args.listing_type == ListingType::UNIT {
        require!(ctx.accounts.item_0.decimals == 0, BazaarError::TokenUnitListingsNotAllowed);
    }

    // go through the items and transfer them into listing owned accounts + set up the ListingItem structs
   transfer_items_to_listing(&ctx.accounts, &args.item_quantities)?;

    ////// create the listing

    // first create the listing items - need to do this first since the listing will be a mutable borrow of the accounts
    let listing_items = create_listing_items(&ctx.accounts, &args.item_quantities)?;
    let listing_domain = &ctx.accounts.listing_domain;

    let listing = &mut ctx.accounts.listing;
    listing.account_version = CURRENT_LISTING_VERSION;
    listing.price = args.price;
    listing.currency = ctx.accounts.currency.key();
    listing.bump = *ctx.bumps.get("listing").unwrap();
    listing.treasury = listing_domain.treasury.key();
    listing.listing_type = args.listing_type;
    listing.seller_account = ctx.accounts.seller_account.key();
    listing.listing_index = listing_index;

    if ctx.accounts.currency.key() == NATIVE_MINT {
        require!(ctx.accounts.proceeds.is_some(), BazaarError::ProceedsAccountNotSpecified);
        listing.proceeds = ctx.accounts.proceeds.as_ref().unwrap().key();
        // listing.proceeds = proceeds.as_ref().unwrap().key();
    } else {
        require!(ctx.accounts.proceeds_token.is_some(), BazaarError::ProceedsTokenAccountNotSpecified);
        listing.proceeds = ctx.accounts.proceeds_token.as_ref().unwrap().key();
        // listing.proceeds = proceeds_token.as_ref().unwrap().key();
    }

    for listing_item in listing_items {
        listing.add_listing_item(listing_item)?;
    }

    Ok(())
}

fn transfer_items_to_listing(accounts: &CreateListing<'_>, item_quantities: &[u64]) -> Result<()> {
    let token_program = &accounts.token_program.to_account_info();
    let seller = &accounts.seller.to_account_info();

    for (i, quantity) in item_quantities.iter().enumerate() {
        let seller_item_token = match i {
            0 => &accounts.item_0_seller_token,
            1 => accounts.item_1_seller_token.as_ref().unwrap(),
            2 => accounts.item_2_seller_token.as_ref().unwrap(),
            _ => unreachable!(),
        };

        let listing_item_token = match i {
            0 => &accounts.item_0_listing_token,
            1 => accounts.item_1_listing_token.as_ref().unwrap(),
            2 => accounts.item_2_listing_token.as_ref().unwrap(),
            _ => unreachable!(),
        };

        let cpi_accounts = Transfer {
            from: seller_item_token.to_account_info(),
            to: listing_item_token.to_account_info(),
            authority: seller.clone(),
        };

        let cpi_ctx = CpiContext::new(token_program.clone(), cpi_accounts);
        token::transfer(cpi_ctx, *quantity)?;
    }

    Ok(())
}

fn create_listing_items(accounts: &CreateListing<'_>, item_quantities: &[u64]) -> Result<Vec<ListingItem>> {

    let mut listing_items = Vec::new();

    for (i, quantity) in item_quantities.iter().enumerate() {
        let item_mint = match i {
            0 => &accounts.item_0,
            1 => accounts.item_1.as_ref().unwrap(),
            2 => accounts.item_2.as_ref().unwrap(),
            _ => unreachable!(),
        };

        let listing_item_token = match i {
            0 => &accounts.item_0_listing_token,
            1 => accounts.item_1_listing_token.as_ref().unwrap(),
            2 => accounts.item_2_listing_token.as_ref().unwrap(),
            _ => unreachable!(),
        };

        listing_items.push(ListingItem {
            quantity: *quantity,
            item_mint: item_mint.key(),
            item_token: listing_item_token.key()
        });

    }

    Ok(listing_items)
}

fn check_item_quantities(accounts: &CreateListing<'_>, item_quantities: &[u64]) -> Result<()> {
    require!(item_quantities.len() > 0, BazaarError::MissingItemQuantities);
    require!(item_quantities.iter().all(|&x| x > 0), BazaarError::InvalidItemQuantity);

    for (i, quantity) in item_quantities.iter().enumerate() {
        let item_account = match i {
            0 => &accounts.item_0_seller_token,
            1 => accounts.item_1_seller_token.as_ref().unwrap(),
            2 => accounts.item_2_seller_token.as_ref().unwrap(),
            _ => unreachable!(),
        };

        require!(item_account.amount >= *quantity, BazaarError::NotEnoughItems);
    }

    Ok(())
}


#[derive(Accounts)]
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

    pub item_0: Box<Account<'info, Mint>>,

    #[account(
        mut,
        token::mint = item_0,
        token::authority = seller
    )]
    pub item_0_seller_token: Box<Account<'info, TokenAccount>>,


    #[account(
        init,
        payer = seller,
        associated_token::mint = item_0,
        associated_token::authority = listing
    )]
    pub item_0_listing_token: Box<Account<'info, TokenAccount>>,

    pub item_1: Option<Box<Account<'info, Mint>>>,

    #[account(
        mut,
        token::mint = item_1,
        token::authority = seller
    )]
    pub item_1_seller_token: Option<Box<Account<'info, TokenAccount>>>,

    #[account(
        init,
        payer = seller,
        associated_token::mint = item_1,
        associated_token::authority = listing
    )]
    pub item_1_listing_token: Option<Box<Account<'info, TokenAccount>>>,

    pub item_2: Option<Box<Account<'info, Mint>>>,

    #[account(
        mut,
        token::mint = item_2,
        token::authority = seller
    )]
    pub item_2_seller_token: Option<Box<Account<'info, TokenAccount>>>,

    #[account(
        init,
        payer = seller,
        associated_token::mint = item_2,
        associated_token::authority = listing
    )]
    pub item_2_listing_token: Option<Box<Account<'info, TokenAccount>>>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CreateListingArgs {
    pub price: u64,
    pub listing_type: ListingType,
    pub item_quantities: Vec<u64>,
}
