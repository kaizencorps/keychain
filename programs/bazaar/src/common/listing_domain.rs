use anchor_lang::prelude::*;


#[account]
pub struct ListingDomain {

    // the  version of the listing domain account
    pub account_version: u8,
    pub domain_index: u8,       // domain might have more than 1 ListingDomain
    pub bump: u8,
    pub treasury: Pubkey,
    pub name: [u8; 32],

}

impl ListingDomain {
    pub const MAX_SIZE: usize =
        1 + // account version
        1 + // domain index
        1 + // bump
            32 + // name
            32 + // treasury
            512; // extra space
}
