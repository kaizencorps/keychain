use anchor_lang::prelude::*;

declare_id!("bazzHtCvoPjpkz3dtSsmYChsbBa7ZPkjQ5xLAMYgBU6");

pub mod instructions;
pub mod common;
pub mod error;

use instructions::{
    create_listing::*,
    create_listing_domain::*,
    update_listing_domain::*,
    create_seller::*,
    delist::*,
    update_listing::*,
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

    pub fn update_listing_domain(
        ctx: Context<UpdateListingDomain>,
        name: String,
        domain_index: u8,
        args: UpdateListingDomainArgs,
    ) -> Result<()> {
        handle_update_listing_domain(ctx, name, domain_index, args)
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

    pub fn delist(
        ctx: Context<Delist>,
    ) -> Result<()> {
        handle_delist(ctx)
    }

    pub fn update_listing(
        ctx: Context<UpdateListing>,
        price: u64
    ) -> Result<()> {
        handle_update_listing(ctx, price)
    }

}

#[derive(Accounts)]
pub struct Initialize {}
