use crate::program::Yardsale;

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::{invoke, invoke_signed};

use anchor_spl::token::{self, CloseAccount, Mint, Token, TokenAccount, Transfer};
use anchor_spl::associated_token::AssociatedToken;
use spl_token::native_mint::ID as NATIVE_MINT;

use mpl_token_metadata::{
    self,
    instruction::{builders::TransferBuilder, InstructionBuilder, TransferArgs},
    processor::AuthorizationData,
    state::{Metadata, ProgrammableConfig::V1, TokenMetadataAccount, TokenStandard},
};

// "prod" / staging address
declare_id!("yar3RNWQaixwFAcAXZ4wySQAiyuSxSQYGCp4AjAotM1");

// "pnft" devnet address
// declare_id!("dYaMxY3mLDYRQV68tgiV7gfPUC4eNzvEjvrDYSy4itq");

pub mod error;
pub mod account;
pub mod constant;
pub mod context;
pub mod util;

use error::*;
use account::*;
use constant::*;
use context::*;
use util::*;

#[program]
pub mod yardsale {
    use anchor_lang::solana_program::program::invoke;
    use anchor_lang::solana_program::system_instruction;
    use mpl_token_auth_rules::payload::{Payload, PayloadType, SeedsVec};
    use mpl_token_metadata::instruction::builders::TransferBuilder;
    use mpl_token_metadata::instruction::TransferArgs;
    use mpl_token_metadata::pda::find_token_record_account;
    use mpl_token_metadata::state::PayloadKey;
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

    pub fn list_pnft<'info>(
        ctx: Context<'_, '_, '_, 'info, ListPNFT<'info>>,
        price: u64,
        authorization_data: Option<AuthorizationDataLocal>,
        rules_acc_present: bool,
    ) -> Result<()> {

        // make sure the item exists in the from account
        require!(ctx.accounts.authority_item_token.amount == 1, YardsaleError::InvalidItem);

        // first, transfer the item to the listing ata
        let rem_acc = &mut ctx.remaining_accounts.iter();
        let auth_rules = if rules_acc_present {
            Some(next_account_info(rem_acc)?)
        } else {
            None
        };
        send_pnft(
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.authority_item_token,
            &ctx.accounts.listing_item_token,
            &ctx.accounts.listing.to_account_info(),
            &ctx.accounts.item,
            &ctx.accounts.item_metadata,
            &ctx.accounts.edition,
            &ctx.accounts.system_program,
            &ctx.accounts.token_program,
            &ctx.accounts.associated_token_program,
            &ctx.accounts.instructions,
            &ctx.accounts.authority_token_record,
            &ctx.accounts.listing_token_record,
            &ctx.accounts.authorization_rules_program,
            auth_rules,
            authorization_data,
            None
        )?;

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

        make_purchase(
            listing,
            &ctx.accounts.authority.to_account_info(),
            &ctx.accounts.proceeds,
            &ctx.accounts.proceeds_token,
            &ctx.accounts.authority_currency_token,
            &ctx.accounts.system_program,
            &ctx.accounts.token_program,
        )?;

        let listing_item_token_ai = ctx.accounts.listing_item_token.to_account_info();
        let auth_item_token_ai = ctx.accounts.authority_item_token.to_account_info();
        let treasury_ai = ctx.accounts.treasury.to_account_info();
        let token_prog_ai = ctx.accounts.token_program.to_account_info();

        // now let's transfer the item to the buyer
        transfer_item_and_close(listing, listing_item_token_ai, auth_item_token_ai, treasury_ai, token_prog_ai)
    }

    pub fn delist_pnft(ctx: Context<DelistPNFT>) -> Result<()> {
        let listing = &ctx.accounts.listing;

        // this is for the pNFT transfer
        tranfer_pnft_from_pda(
            listing,
            &ctx.accounts.listing_item_token,
            &ctx.accounts.seller,
            &ctx.accounts.seller_item_token,
            &ctx.accounts.item,
            &ctx.accounts.item_metadata,
            &ctx.accounts.edition,
            &ctx.accounts.seller_token_record,
            &ctx.accounts.listing_token_record,
            &ctx.accounts.ruleset,
            &ctx.accounts.authorization_rules_program,
            &ctx.accounts.token_metadata_program,
            &ctx.accounts.instructions,
            &ctx.accounts.token_program,
            &ctx.accounts.associated_token_program,
            &ctx.accounts.system_program
        )?;

        // todo: for now just always close the account, but later we'll support SFTs

        // now we can close the item listing token account

        let listing_item_token = ctx.accounts.listing_item_token.to_account_info();
        let seller = ctx.accounts.seller.to_account_info();

        close_listing_owned_account(listing, listing_item_token, seller, &ctx.accounts.token_program)?;

        Ok(())

    }


    // purchase an item
    pub fn purchase_pnft<'info>(ctx: Context<'_, '_, '_, 'info, PurchasePNFT<'info>>) -> Result<()> {
        let listing = &ctx.accounts.listing;

        make_purchase(
            listing,
            &ctx.accounts.buyer.to_account_info(),
            &ctx.accounts.proceeds,
            &ctx.accounts.proceeds_token,
            &ctx.accounts.buyer_currency_token,
            &ctx.accounts.system_program,
            &ctx.accounts.token_program,
        )?;

        // this is for the pNFT transfer
        tranfer_pnft_from_pda(
            listing,
            &ctx.accounts.listing_item_token,
            &ctx.accounts.buyer,
            &ctx.accounts.buyer_item_token,
            &ctx.accounts.item,
            &ctx.accounts.item_metadata,
            &ctx.accounts.edition,
            &ctx.accounts.buyer_token_record,
            &ctx.accounts.listing_token_record,
            &ctx.accounts.ruleset,
            &ctx.accounts.authorization_rules_program,
            &ctx.accounts.token_metadata_program,
            &ctx.accounts.instructions,
            &ctx.accounts.token_program,
            &ctx.accounts.associated_token_program,
            &ctx.accounts.system_program
        )?;

        // todo: for now just always close the account, but later we'll support SFTs

        // now we can close the item listing token account

        let listing_item_token = ctx.accounts.listing_item_token.to_account_info();
        let treasury = ctx.accounts.treasury.to_account_info();

        close_listing_owned_account(listing, listing_item_token, treasury, &ctx.accounts.token_program)?;

        Ok(())

    }

}
