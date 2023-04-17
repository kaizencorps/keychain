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
        require!(ctx.accounts.from_item_token.amount == 1, YardsaleError::InvalidItem);

        // should we disallow a price of 0 ..?
        // require!(price > 0, YardsaleError::InvalidPrice);

        // first, transfer the item to the listing ata
        let cpi_accounts = Transfer {
            from: ctx.accounts.from_item_token.to_account_info(),
            to: ctx.accounts.item_token.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, 1)?;

        // now create the listing
        let listing = &mut ctx.accounts.listing;
        listing.price = price;
        listing.item = ctx.accounts.item.key();
        listing.item_token = ctx.accounts.item_token.key();
        listing.domain = ctx.accounts.keychain.domain.clone();
        listing.keychain = ctx.accounts.keychain.key();
        listing.currency = ctx.accounts.currency.key();
        listing.bump = *ctx.bumps.get("listing").unwrap();
        listing.treasury = ctx.accounts.domain.treasury.key();

        if listing.currency == NATIVE_MINT {
            // then the sale token isn't needed, but a regular accountinfo should've been specified (wallet)
            // then the sale token is needed, but an accountinfo shouldn't have been specified (wallet)
            require!(ctx.accounts.sale_account.is_some(), YardsaleError::ProceedsAccountNotSpecified);
            listing.proceeds = ctx.accounts.sale_account.as_ref().unwrap().key();
        } else {
            // then the sale token is needed, but an accountinfo shouldn't have been specified (wallet)
            require!(ctx.accounts.sale_token.is_some(), YardsaleError::ProceedsTokenAccountNotSpecified);
            listing.proceeds = ctx.accounts.sale_token.as_ref().unwrap().key();
        }

        Ok(())
    }

    // purchase an item
    pub fn purchase_item(ctx: Context<PurchaseItem>) -> Result<()>  {

        let listing = &ctx.accounts.listing;

        // check that the buyer has enough funds to purchase the item
        if listing.currency == NATIVE_MINT {
            require!(ctx.accounts.authority.lamports() > listing.price, YardsaleError::InsufficientFunds);
            require!(ctx.accounts.sale_account.is_some(), YardsaleError::ProceedsAccountNotSpecified);
            // proper account matching listing gets checked in the constraint

            // pay for the item
            invoke(
                &system_instruction::transfer(
                    ctx.accounts.authority.key,
                    &listing.proceeds,
                    listing.price,
                ),
                &[
                    ctx.accounts.authority.to_account_info().clone(),
                    ctx.accounts.sale_account.as_ref().unwrap().clone(),
                    ctx.accounts.system_program.to_account_info().clone(),
                ],
            )?;
        } else {
            require!(ctx.accounts.buyer_token.is_some(), YardsaleError::FundingAccountNotSpecified);
            require!(ctx.accounts.buyer_token.as_ref().unwrap().amount >= listing.price, YardsaleError::InsufficientFunds);
            require!(ctx.accounts.sale_token.is_some(), YardsaleError::ProceedsAccountNotSpecified);
            // proper account matching listing gets checked in the constraint

            // pay for the item with spl token
            let cpi_accounts = Transfer {
                from: ctx.accounts.buyer_token.as_ref().unwrap().to_account_info(),
                to: ctx.accounts.sale_token.as_ref().unwrap().to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            token::transfer(cpi_ctx, listing.price)?;
        }

        Ok(())
    }
}

