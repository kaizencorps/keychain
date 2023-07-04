use anchor_lang::prelude::*;

declare_id!("EQQ46qSk1pZKVEqsejCyAvFenNQ1XTnZie4RfTXPTmgs");

pub mod instructions;
pub mod common;
pub mod error;

use instructions::{
    create_listing::*,
    create_listing_domain::*,
    create_seller::*,
    buy::*,
};

#[program]
pub mod bazaar {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }


    pub fn create_listing_domain(
        ctx: Context<CreateListingDomain>,
        args: CreateListingDomainArgs,
    ) -> Result<()> {
        handle_create_listing_domain(ctx, args)
    }

    pub fn create_seller(
        ctx: Context<CreateSeller>,
    ) -> Result<()> {
        handle_create_seller(ctx)
    }

    pub fn create_listing(
        ctx: Context<CreateListing>,
        args: CreateListingArgs,
    ) -> Result<()> {
        handle_create_listing(ctx, args)
    }

    pub fn buy(
        ctx: Context<Buy>,
        num_items: u64
    ) -> Result<()> {
        handle_buy(ctx, num_items)
    }
}

#[derive(Accounts)]
pub struct Initialize {}
