use anchor_lang::prelude::*;


#[account]
pub struct SellerAccount {

    // the  seller's keychain pda
    pub account_version: u8,
    pub bump: u8,
    pub keychain: Pubkey,

    // pub keychain_name: String,
    // pub keychain_domain: String,
    pub listing_index: u32,
    pub num_sales: u32,

}

impl SellerAccount {
    pub const MAX_SIZE: usize =
        1 + // account version
        1 + // bump
        32 + // keychain
        4 + // listing index
        4 + // num sales
        192; // extra space
}
