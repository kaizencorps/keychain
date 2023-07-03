use anchor_lang::prelude::*;
use crate::common::constant::MAX_LISTING_ITEMS;
use crate::error::BazaarError;


#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub enum ListingType {
    BAG,        // a bag of different items sold for a single price
    UNIT,       // 1 or more items of a single type, each sold for a fixed price
}

#[account]
pub struct Listing {

    // the  version of the listing account
    pub account_version: u8,
    pub bump: u8,

    // listing info
    pub currency: Pubkey,     // native mint (wSOL) or an spl token mint
    pub price: u64,
    pub proceeds: Pubkey,     // token account if currency = spl or just account if currency = sol
    pub listing_type: ListingType,
    pub items: Vec<ListingItem>,

    // pulled from the listing domain
    pub treasury: Pubkey,

}

impl Listing {
    pub const MAX_SIZE: usize =
        1 + // account version
        1 + // bump
            32 + // currency
            8 + // price
            32 + // proceeds account (token account or regular if sol = currency)
            1 + // listing type
            32 + // treasury
            // 32 + // mint
            (4 + (MAX_LISTING_ITEMS * ListingItem::SIZE)) +   // items
            192; // extra space

    pub fn add_listing_item(&mut self, item: ListingItem) -> Result<()> {
        if self.items.len() >= MAX_LISTING_ITEMS {
            return Err(BazaarError::TooManyItems.into());
        }
        self.items.push(item);
        Ok(())
    }
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct ListingItem {
    pub quantity: u64,          // u64 cause it might be a token
    pub item_mint: Pubkey,      // item's mint
    pub item_token: Pubkey,     // listing's token account
}

impl ListingItem {
    pub const SIZE: usize =
        8 +         // quantity
        32 +        // item (mint)
        32;         // item token account
}
