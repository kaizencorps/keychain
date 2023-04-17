use anchor_lang::prelude::*;
use crate::error::YardsaleError;

#[account]
pub struct Listing {

    // these are for doing gPA lookups
    pub bump: u8,
    pub domain: String,
    pub keychain: Pubkey,

    pub treasury: Pubkey,

    // todo: add collection for lookups too
    // pub collection: Pubkey,

    pub item: Pubkey,
    pub item_token: Pubkey,

    pub price: u64,
    // none if priced in sol
    pub currency: Pubkey,
    pub proceeds: Pubkey,     // token account if currency = spl or just account if currency = sol

}

impl Listing {
    pub const MAX_SIZE: usize =
        1 + // bump
        32 + // domain
            32 + // keychain
            32 + // treasury
            32 + // mint
            32 + // ata
            8 + // price
            32 + // currency
            32 + // proceeds account (token account or regular if sol = currency)
            192; // extra space
}

