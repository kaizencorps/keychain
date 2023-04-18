use anchor_lang::prelude::*;
use crate::program::Yardsale;

use anchor_spl::token::{self, CloseAccount, Mint, Token, TokenAccount, Transfer};
use spl_token::native_mint::ID as NATIVE_MINT;

declare_id!("yar3RNWQaixwFAcAXZ4wySQAiyuSxSQYGCp4AjAotM1");

pub mod error;
pub mod account;
pub mod constant;
pub mod context;

use error::*;
use account::*;
use constant::*;
use context::*;

#[program]
pub mod yardsale {
    use anchor_lang::solana_program::program::invoke;
    use anchor_lang::solana_program::system_instruction;
    use super::*;

    // list an item
    pub fn list_item(ctx: Context<ListItem>, price: u64) -> Result<()> {
        // make sure the item exists in the from account
        require!(ctx.accounts.authority_item_token.amount == 1, YardsaleError::InvalidItem);

        // should we disallow a price of 0 ..?
        // require!(price > 0, YardsaleError::InvalidPrice);

        // first, transfer the item to the listing ata
        let cpi_accounts = Transfer {
            from: ctx.accounts.authority_item_token.to_account_info(),
            to: ctx.accounts.listing_item_token.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, 1)?;

        // now create the listing
        let listing = &mut ctx.accounts.listing;
        listing.price = price;
        listing.item = ctx.accounts.item.key();
        listing.item_token = ctx.accounts.listing_item_token.key();
        listing.domain = ctx.accounts.keychain.domain.clone();
        listing.keychain = ctx.accounts.keychain.name.clone();
        listing.currency = ctx.accounts.currency.key();
        listing.bump = *ctx.bumps.get("listing").unwrap();
        listing.treasury = ctx.accounts.domain.treasury.key();

        if listing.currency == NATIVE_MINT {
            // then the sale token isn't needed, but a regular accountinfo should've been specified (wallet)
            // then the sale token is needed, but an accountinfo shouldn't have been specified (wallet)
            require!(ctx.accounts.proceeds.is_some(), YardsaleError::ProceedsAccountNotSpecified);
            listing.proceeds = ctx.accounts.proceeds.as_ref().unwrap().key();
        } else {
            // then the sale token is needed, but an accountinfo shouldn't have been specified (wallet)
            require!(ctx.accounts.proceeds_token.is_some(), YardsaleError::ProceedsTokenAccountNotSpecified);
            listing.proceeds = ctx.accounts.proceeds_token.as_ref().unwrap().key();
        }

        Ok(())
    }

    // delist an item
    pub fn delist_item(ctx: Context<DelistItem>) -> Result<()> {
        let listing = &ctx.accounts.listing;

        let listing_item_token_ai = ctx.accounts.listing_item_token.to_account_info();
        let auth_item_token_ai = ctx.accounts.authority_item_token.to_account_info();
        let lamports_claimer_ai = ctx.accounts.authority.to_account_info();
        let token_prog_ai = ctx.accounts.token_program.to_account_info();

        // transfer the item to the authority
        transfer_item_and_close(listing, listing_item_token_ai, auth_item_token_ai, lamports_claimer_ai, token_prog_ai)
    }

    // update the price of an item
    pub fn update_price(ctx: Context<UpdatePrice>, price: u64) -> Result<()> {
        let listing = &mut ctx.accounts.listing;
        listing.price = price;
        Ok(())
    }

    // purchase an item
    pub fn purchase_item(ctx: Context<PurchaseItem>) -> Result<()> {
        let listing = &ctx.accounts.listing;

        // check that the buyer has enough funds to purchase the item
        if listing.currency == NATIVE_MINT {
            require!(ctx.accounts.authority.lamports() > listing.price, YardsaleError::InsufficientFunds);
            require!(ctx.accounts.proceeds.is_some(), YardsaleError::ProceedsAccountNotSpecified);
            // proper account matching listing gets checked in the constraint

            // pay for the item with sol
            invoke(
                &system_instruction::transfer(
                    ctx.accounts.authority.key,
                    &listing.proceeds,
                    listing.price,
                ),
                &[
                    ctx.accounts.authority.to_account_info().clone(),
                    ctx.accounts.proceeds.as_ref().unwrap().clone(),
                    ctx.accounts.system_program.to_account_info().clone(),
                ],
            )?;
        } else {
            require!(ctx.accounts.authority_currency_token.is_some(), YardsaleError::FundingAccountNotSpecified);
            require!(ctx.accounts.authority_currency_token.as_ref().unwrap().amount >= listing.price, YardsaleError::InsufficientFunds);
            require!(ctx.accounts.proceeds_token.is_some(), YardsaleError::ProceedsAccountNotSpecified);
            // proper account matching listing gets checked in the constraint

            // pay for the item with spl token
            let cpi_accounts = Transfer {
                from: ctx.accounts.authority_currency_token.as_ref().unwrap().to_account_info(),
                to: ctx.accounts.proceeds_token.as_ref().unwrap().to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            token::transfer(cpi_ctx, listing.price)?;
        }

        let listing_item_token_ai = ctx.accounts.listing_item_token.to_account_info();
        let auth_item_token_ai = ctx.accounts.authority_item_token.to_account_info();
        let lamports_claimer_ai = ctx.accounts.treasury.to_account_info();
        let token_prog_ai = ctx.accounts.token_program.to_account_info();

        // now let's transfer the item to the buyer
        transfer_item_and_close(listing, listing_item_token_ai, auth_item_token_ai, lamports_claimer_ai, token_prog_ai)

    }

}

// transfers an item out of the listing's token account and closes it
fn transfer_item_and_close<'a, 'b>(listing: &Box<Account<'a, Listing>>,
                                   listing_item_token_ai: AccountInfo<'b>,
                                   to_token_ai: AccountInfo<'b>,
                                   lamports_claimer_ai: AccountInfo<'a>,
                                   token_program: AccountInfo<'a>) -> Result<()>
    where 'a: 'b, 'b: 'a {

    let seeds = &[
        listing.item.as_ref(),
        LISTINGS.as_bytes().as_ref(),
        listing.keychain.as_bytes().as_ref(),
        listing.domain.as_bytes().as_ref(),
        YARDSALE.as_bytes().as_ref(),
        &[listing.bump],
    ];
    let signer = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: listing_item_token_ai.clone(),
        to: to_token_ai.clone(),
        authority: listing.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        token_program.clone(),
        cpi_accounts,
        signer);
    token::transfer(cpi_ctx, 1)?;

    // now we can close the item listing account
    let cpi_close_accounts = CloseAccount {
        account: listing_item_token_ai.clone(),
        destination: lamports_claimer_ai.clone(),
        authority: listing.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(token_program.clone(),
                                              cpi_close_accounts, signer);
    token::close_account(cpi_ctx)?;

    Ok(())
}

