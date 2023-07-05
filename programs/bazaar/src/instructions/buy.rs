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
use anchor_lang::solana_program::program::invoke;
use anchor_lang::solana_program::system_instruction;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer, CloseAccount};
use spl_token::native_mint::ID as NATIVE_MINT;


use crate::common::constant::{CURRENT_SELLER_VERSION, LISTING};
use crate::common::util::transfer_items;


pub fn handle_buy(
    ctx: Context<Buy>,
    num_units: u64,         // only applies if the listing is of type unit, otherwise should be 1
) -> Result<()> {

    let listing = &mut ctx.accounts.listing;

    handle_payment(
        num_units,
        listing,
        &ctx.accounts.buyer.to_account_info(),
        &ctx.accounts.proceeds,
        &ctx.accounts.proceeds_token,
        &ctx.accounts.buyer_currency_token,
        &ctx.accounts.system_program,
        &ctx.accounts.token_program,
    )?;

    // if it's a unit listing, see if all units are bought. if so then we close the listing below
    let mut close_listing = true;
    if listing.listing_type == ListingType::UNIT {
        listing.items[0].quantity -= num_units;
        if listing.items[0].quantity > 0 {
            close_listing = false;
        }

        // transfer the specified number of units to the buyer
        let listing_item_token_ai = ctx.accounts.item_0_listing_token.to_account_info();
        let buyer_item_token_ai = ctx.accounts.item_0_buyer_token.to_account_info();
        let treasury_ai = ctx.accounts.treasury.to_account_info();
        let token_prog_ai = ctx.accounts.token_program.to_account_info();

        transfer_items(listing, listing_item_token_ai, buyer_item_token_ai, num_units, treasury_ai, close_listing, token_prog_ai)?;

    } else {
        // bag: transfer everything
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

            let buyer_item_token_ai = match i {
                0 => ctx.accounts.item_0_buyer_token.to_account_info(),
                1 => {
                    require!(ctx.accounts.item_1_buyer_token.is_some(), BazaarError::MissingBuyerItemToken);
                    ctx.accounts.item_1_buyer_token.as_ref().unwrap().to_account_info()
                },
                2 => {
                    require!(ctx.accounts.item_2_buyer_token.is_some(), BazaarError::MissingBuyerItemToken);
                    ctx.accounts.item_2_buyer_token.as_ref().unwrap().to_account_info()
                },
                _ => unreachable!("Only 3 items are supported"),
            };

            let treasury_ai = ctx.accounts.treasury.to_account_info();
            let token_prog_ai = ctx.accounts.token_program.to_account_info();

            transfer_items(listing, listing_item_token_ai, buyer_item_token_ai, listing.items[i].quantity, treasury_ai, true, token_prog_ai)?;
        }
    }

    // credit the seller account with a sale
    let seller_account = &mut ctx.accounts.seller_account;
    seller_account.num_sales = seller_account.num_sales.checked_add(1).unwrap();

    // close the listing if we need to
    if close_listing {
        msg!("Closing listing account");

        let listing_account_info = listing.to_account_info();
        let remaining_lamports = listing_account_info.lamports();

        msg!("transferring all lamports out of account (to close) {}: {}", ctx.accounts.treasury.key(), remaining_lamports);

        // transfer sol: https://solanacookbook.com/references/programs.html#how-to-transfer-sol-in-a-program

        // Debit from_account and credit to_account
        **listing_account_info.try_borrow_mut_lamports()? -= remaining_lamports;
        **ctx.accounts.treasury.try_borrow_mut_lamports()? += remaining_lamports;
    }

    Ok(())
}

// fn close_listing(listing: &mut Box<Account<'info, Listing>>,
//                  treasury: &AccountInfo<'info>,
//                  token_program: &Program <'info, Token>,
// ) -> Result<()> {
//
//     let cpi_close_accounts = CloseAccount {
//         account: listing.to_account_info(),
//         destination: treasury.clone(),
//         authority: listing.to_account_info(),
//     };
//     let cpi_ctx = CpiContext::new_with_signer(token_program.to_account_info(),
//                                               cpi_close_accounts, signer);
//     token::close_account(cpi_ctx)?;
//
//     Ok(())
// }

// transfers funds from buyer to seller
pub fn handle_payment<'info>(num_units: u64,
                             listing: &mut Box<Account<'info, Listing>>,
                             buyer: &AccountInfo<'info>,
                             proceeds: &Option<AccountInfo<'info>>,
                             proceeds_token: &Option<Account<'info, TokenAccount>>,
                             buyer_currency_token: &Option<Account<'info, TokenAccount>>,
                             system_program: &Program <'info, System>,
                             token_program: &Program <'info, Token>,
) -> Result<()> {

    // check that the buyer has enough currency
    let currency_amount_for_purchase = match listing.listing_type {
        ListingType::UNIT => {
            // check that there are enough units
            require!(listing.items[0].quantity >= num_units, BazaarError::InsufficientUnits);
            listing.price.checked_mul(num_units).unwrap()
        },
        ListingType::BAG => listing.price,
    };

    if listing.currency == NATIVE_MINT {
        require!(buyer.lamports() > currency_amount_for_purchase, BazaarError::InsufficientFunds);
        require!(proceeds.is_some(), BazaarError::ProceedsAccountNotSpecified);
        // proper account matching listing gets checked in the constraint

        // pay for the item with sol
        invoke(
            &system_instruction::transfer(
                buyer.key,
                &listing.proceeds,
                currency_amount_for_purchase,
            ),
            &[
                buyer.clone(),
                proceeds.as_ref().unwrap().clone(),
                system_program.to_account_info().clone(),
            ],
        )?;
    } else {
        require!(buyer_currency_token.is_some(), BazaarError::FundingAccountNotSpecified);
        require!(buyer_currency_token.as_ref().unwrap().amount >= currency_amount_for_purchase, BazaarError::InsufficientFunds);
        require!(proceeds_token.is_some(), BazaarError::ProceedsAccountNotSpecified);
        // proper account matching listing gets checked in the constraint

        // pay for the item with spl token
        let cpi_accounts = Transfer {
            from: buyer_currency_token.as_ref().unwrap().to_account_info(),
            to: proceeds_token.as_ref().unwrap().to_account_info(),
            authority: buyer.clone(),
        };
        let cpi_ctx = CpiContext::new(token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, listing.price)?;
    }
    msg!("Payment complete");
    Ok(())
}

#[derive(Accounts)]
pub struct Buy<'info> {

    // the buyer
    #[account(mut)]
    pub buyer: Signer<'info>,

    // if the currency is spl, then this is the buyer's token account
    #[account(
        mut,
        token::mint = currency,
        token::authority = buyer
    )]
    pub buyer_currency_token: Option<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = listing.has_item(&item_0.key()),
        has_one = seller_account,
    )]
    pub listing: Box<Account<'info, Listing>>,

    #[account(
        mut,
    )]
    pub seller_account: Box<Account<'info, SellerAccount>>,

    // the currency the listing is being sold for - native mint for straight sol
    #[account(
        constraint = listing.currency == currency.key()
    )]
    pub currency: Account<'info, Mint>,

    // the token account to deposit the proceeds into - necessary if currency is spl
    #[account(
        mut,
        token::mint = currency,
        constraint = listing.proceeds == proceeds_token.key(),
    )]
    pub proceeds_token: Option<Account<'info, TokenAccount>>,

    /// CHECK: this is only specified if the currency is native (will usually just be the seller, but can be any account to send proceeds to)
    #[account(
        mut,
        constraint = listing.proceeds == proceeds.key()
    )]
    pub proceeds: Option<AccountInfo<'info>>,

    #[account(
        constraint = listing.has_item(&item_0.key())
    )]
    pub item_0: Box<Account<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = item_0,
        associated_token::authority = buyer,
    )]
    pub item_0_buyer_token: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = item_0,
        associated_token::authority = listing,
        constraint = listing.has_item_token(&item_0_listing_token.key())
    )]
    pub item_0_listing_token: Box<Account<'info, TokenAccount>>,

    #[account(
        constraint = listing.has_item(&item_1.key())
    )]
    pub item_1: Option<Box<Account<'info, Mint>>>,

    #[account(
        mut,
        associated_token::mint = item_1,
        associated_token::authority = buyer,
    )]
    pub item_1_buyer_token: Option<Box<Account<'info, TokenAccount>>>,

    #[account(
        mut,
        associated_token::mint = item_1,
        associated_token::authority = listing,
        constraint = listing.has_item_token(&item_1_listing_token.key())
    )]
    pub item_1_listing_token: Option<Box<Account<'info, TokenAccount>>>,

    #[account(
        constraint = listing.has_item(&item_2.key())
    )]
    pub item_2: Option<Box<Account<'info, Mint>>>,

    #[account(
        mut,
        associated_token::mint = item_2,
        associated_token::authority = buyer,
    )]
    pub item_2_buyer_token: Option<Box<Account<'info, TokenAccount>>>,

    #[account(
        mut,
        associated_token::mint = item_2,
        associated_token::authority = listing,
        constraint = listing.has_item_token(&item_2_listing_token.key())
    )]
    pub item_2_listing_token: Option<Box<Account<'info, TokenAccount>>>,

    /// CHECK: just sending lamports here when closing listing accounts
    #[account(
        mut,
        constraint = listing.treasury == treasury.key(),
    )]
    pub treasury: AccountInfo<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

