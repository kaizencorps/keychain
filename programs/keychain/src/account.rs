use anchor_lang::prelude::*;
use crate::constant::MAX_KEYS;

// represents a user's wallet
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct UserKey {
    // size = 1 + 32 = 33
    pub key: Pubkey,
    pub verified: bool                  // initially false after existing key adds a new one, until the added key verifies
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CurrentKeyChain {
    pub name: String,
    pub num_keys: u16,
    pub domain: Pubkey,
    pub bump: u8,
    // Attach a Vector of type ItemStruct to the account.
    pub keys: Vec<UserKey>,
}

/*
#[account]
pub struct OldKeyChain {
    pub num_keys: u16,
    pub domain: Pubkey,
    // Attach a Vector of type ItemStruct to the account.
    pub keys: Vec<UserKey>,
}
 */

#[account]
pub struct KeyChainState {
    // name for the keychain. can be used as a username
    pub version: u8,
    pub keychain: CurrentKeyChain
}

impl KeyChainState {
    pub const MAX_SIZE: usize = 1 + 32 + 2 + 32 + 1 + (4 + (MAX_KEYS * 33));
}

// a "pointer" account which points to the keychain it's attached to. prevent keys from being added ot multiple keychains
#[account]
pub struct KeyChainKey {
    // pointer to the keychain this key is attached to
    pub keychain: Pubkey,
    // the key/wallet this key holds - matches the one in the keychain
    pub key: Pubkey,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct CurrentDomain {
    // max size = 32
    pub name: String,
    pub authority: Pubkey,
    pub treasury: Pubkey,
    pub keychain_cost: u64,            // the cost to add a key to a keychain
    pub bump: u8,
}

#[account]
pub struct DomainState {
    pub version: u8,
    pub domain: CurrentDomain
}

impl DomainState {
    // 32 byte name
    pub const MAX_SIZE: usize = 1 + 32 + 32 + 32 + 1 + 8;
}

