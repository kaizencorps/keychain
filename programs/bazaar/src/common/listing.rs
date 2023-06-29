use anchor_lang::prelude::*;


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


    // pub domain: String,
    // pub keychain: String,
    // todo: add collection for lookups too
    // pub collection: Pubkey,

    // pulled from the domain
    // pub treasury: Pubkey,

    // pub item: Pubkey,
    // pub item_token: Pubkey,     // not used if listing is a c_nft - just set to item


}

impl Listing {
    pub const MAX_SIZE: usize =
        1 + // account version
        1 + // bump
            32 + // currency
            8 + // price
            32 + // proceeds account (token account or regular if sol = currency)
            1 + // listing type
            // 32 + // domain
            // 32 + // keychain
            // 32 + // treasury
            // 32 + // mint
            32 + // ata
            192; // extra space
}
