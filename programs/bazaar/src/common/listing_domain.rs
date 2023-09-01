use anchor_lang::prelude::*;


#[account]
pub struct ListingDomain {

    // the  version of the listing domain account
    pub account_version: u8,
    pub domain_index: u8,       // domain might have more than 1 ListingDomain
    pub bump: u8,
    pub treasury: Pubkey,       // platform treasury (stache)
    pub name: [u8; 32],

    // new in v1
    pub fee_vault: Pubkey,      // where fees get sent
    pub seller_fee_bp: u16

}

impl ListingDomain {
    pub const MAX_SIZE: usize =
        1 + // account version
        1 + // domain index
        1 + // bump
        32 + // treasury
        32 + // name
        32 + // fee vault
        2 + // seller fee bp
        478; // extra space
}
