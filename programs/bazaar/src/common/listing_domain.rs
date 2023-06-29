use anchor_lang::prelude::*;


#[account]
pub struct ListingDomain {

    // the  version of the listing domain account
    pub account_version: u8,
    pub bump: u8,
    pub name: [u8; 32],

}

impl ListingDomain {
    pub const MAX_SIZE: usize =
        1 + // account version
        1 + // bump
            32 + // name
            512; // extra space
}
