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

// compression
// use spl_account_compression::{
//     program::SplAccountCompression, Noop,
// };


// "prod" / staging address
// declare_id!("yar3RNWQaixwFAcAXZ4wySQAiyuSxSQYGCp4AjAotM1");

// "pnft" devnet address
declare_id!("xxzSBWCjaRWKmjqGxbxmEuhqocbaW4aUW1EzFfERS9W");

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
    use anchor_lang::solana_program;
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
        listing.item_type = ItemType::Standard;

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

    pub fn list_compressed_nft<'info>(
        ctx: Context<'_, '_, '_, 'info, ListCompressedNft<'info>>,
        asset_id: Pubkey,
        price: u64,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> Result<()> {

        // remaining_accounts are the accounts that make up the required proof
        /*
        let remaining_accounts_len = ctx.remaining_accounts.len();
        let mut accounts = Vec::with_capacity(
            8 // space for the 8 AccountMetas that are always included  (below)
                + remaining_accounts_len,
        );
        accounts.extend(vec![
            AccountMeta::new_readonly(ctx.accounts.tree_authority.key(), false),
            AccountMeta::new_readonly(ctx.accounts.leaf_owner.key(), true),
            AccountMeta::new_readonly(ctx.accounts.leaf_owner.key(), false),
            AccountMeta::new_readonly(ctx.accounts.listing.key(), false),
            AccountMeta::new(ctx.accounts.merkle_tree.key(), false),
            AccountMeta::new_readonly(ctx.accounts.log_wrapper.key(), false),
            AccountMeta::new_readonly(ctx.accounts.compression_program.key(), false),
            AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
        ]);

        // from the bubblegum src [163, 52, 200, 231, 140, 3, 69, 186] => InstructionName::Transfer,
        let transfer_discriminator: [u8; 8] = [163, 52, 200, 231, 140, 3, 69, 186];

        let mut data = Vec::with_capacity(
            8           // The length of transfer_discriminator,
                + root.len()
                + data_hash.len()
                + creator_hash.len()
                + 8 // The length of the nonce
                + 8, // The length of the index
        );
        data.extend(transfer_discriminator);
        data.extend(root);
        data.extend(data_hash);
        data.extend(creator_hash);
        data.extend(nonce.to_le_bytes());
        data.extend(index.to_le_bytes());

        let mut account_infos = Vec::with_capacity(
            8 // space for the 8 AccountInfos that are always included (below)
                + remaining_accounts_len,
        );
        account_infos.extend(vec![
            ctx.accounts.tree_authority.to_account_info(),
            ctx.accounts.leaf_owner.to_account_info(),
            ctx.accounts.leaf_owner.to_account_info(),
            ctx.accounts.listing.to_account_info(),
            ctx.accounts.merkle_tree.to_account_info(),
            ctx.accounts.log_wrapper.to_account_info(),
            ctx.accounts.compression_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ]);

        // Add "accounts" (hashes) that make up the merkle proof from the remaining accounts.
        for acc in ctx.remaining_accounts.iter() {
            accounts.push(AccountMeta::new_readonly(acc.key(), false));
            account_infos.push(acc.to_account_info());
        }

        let instruction = solana_program::instruction::Instruction {
            program_id: ctx.accounts.bubblegum_program.key(),
            accounts,
            data,
        };

        msg!("manual cpi call to bubblegum program transfer instruction");
        solana_program::program::invoke(&instruction, &account_infos[..])?;

         */

        Ok(())

        /* this way apparently doesn't work ..?   see: https://solana.stackexchange.com/questions/6410/anchor-cpi-bubblegum-burn-error-cause-not-signer
        // new with signer for pda signing
        // let cp_ctx = CpiContext::new_with_signer(
        let cpi_ctx = CpiContext::new(
            ctx.accounts.bubblegum_program.to_account_info(),
            mpl_bubblegum::cpi::accounts::Transfer {
                tree_authority: ctx.accounts.tree_authority.to_account_info(),
                leaf_owner: ctx.accounts.leaf_owner.to_account_info(),
                leaf_delegate: ctx.accounts.leaf_owner.to_account_info(),
                new_leaf_owner: ctx.accounts.listing.to_account_info(),
                merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
                log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
                compression_program: ctx.accounts.compression_program.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            }
            signer,
        );

         */

    }

    pub fn list_pnft<'info>(
        ctx: Context<'_, '_, '_, 'info, ListProgrammableNft<'info>>,
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
        listing.item_type = ItemType::Programmable;

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
    pub fn purchase_pnft<'info>(ctx: Context<'_, '_, '_, 'info, PurchaseProgrammableNft<'info>>) -> Result<()> {
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

    /*

    pub fn purchase_cnft<'info>(ctx: Context<'_, '_, '_, 'info, PurchaseCompressedNft<'info>>,
                                root: [u8; 32],
                                data_hash: [u8; 32],
                                creator_hash: [u8; 32],
                                nonce: u64,
                                index: u32,) -> Result<()> {
        msg!("attempting to send nft {} from tree {}", index, ctx.accounts.merkle_tree.key());

        let mut accounts:  Vec<solana_program::instruction::AccountMeta> = vec![
            AccountMeta::new_readonly(ctx.accounts.tree_authority.key(), false),
            AccountMeta::new_readonly(ctx.accounts.leaf_owner.key(), true),
            AccountMeta::new_readonly(ctx.accounts.leaf_owner.key(), false),
            AccountMeta::new_readonly(ctx.accounts.new_leaf_owner.key(), false),
            AccountMeta::new(ctx.accounts.merkle_tree.key(), false),
            AccountMeta::new_readonly(ctx.accounts.log_wrapper.key(), false),
            AccountMeta::new_readonly(ctx.accounts.compression_program.key(), false),
            AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
        ];

        let mut data: Vec<u8> = vec![];
        data.extend(TRANSFER_DISCRIMINATOR);
        data.extend(root);
        data.extend(data_hash);
        data.extend(creator_hash);
        data.extend(nonce.to_le_bytes());
        data.extend(index.to_le_bytes());

        let mut account_infos: Vec<AccountInfo> = vec![
            ctx.accounts.tree_authority.to_account_info(),
            ctx.accounts.leaf_owner.to_account_info(),
            ctx.accounts.leaf_owner.to_account_info(),
            ctx.accounts.new_leaf_owner.to_account_info(),
            ctx.accounts.merkle_tree.to_account_info(),
            ctx.accounts.log_wrapper.to_account_info(),
            ctx.accounts.compression_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ];

        // add "accounts" (hashes) that make up the merkle proof
        for acc in ctx.remaining_accounts.iter() {
            accounts.push(AccountMeta::new_readonly(acc.key(), false));
            account_infos.push(acc.to_account_info());
        }

        msg!("manual cpi call");

        invoke_signed(&instruction, &account_infos, signer).unwrap();


        solana_program::program::invoke_signed(
            & solana_program::instruction::Instruction {
                program_id: ctx.accounts.bubblegum_program.key(),
                accounts: accounts,
                data: data,
            },
            &account_infos[..],
            &[&[b"cNFT-vault", &[*ctx.bumps.get("leaf_owner").unwrap()]]])
            .map_err(Into::into)

    }

     */



}
