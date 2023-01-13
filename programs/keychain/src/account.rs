use anchor_lang::prelude::*;

// represents a user's wallet
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct UserKey {
    // size = 1 + 32 = 33
    pub key: Pubkey,
    pub verified: bool                  // initially false after existing key adds a new one, until the added key verifies
}

// todo: might wanna store the "display" version of the playername since the account should be derived from a "normalized" version of the playername
#[account]
pub struct KeyChain {
    // name for the keychain. can be used as a username
    pub name: String,
    pub num_keys: u16,
    pub domain: Pubkey,
    // Attach a Vector of type ItemStruct to the account.
    pub keys: Vec<UserKey>,
}

impl KeyChain {
    // allow up to 3 wallets for now - 2 num_keys + 4 vector + (space(T) * amount)
    pub const MAX_KEYS: usize = 5;
    pub const MAX_SIZE: usize = 2 + 32 + 32 + (4 + (KeyChain::MAX_KEYS * 33));
}

// a "pointer" account which points to the keychain it's attached to. prevent keys from being added ot multiple keychains
#[account]
pub struct KeyChainKey {
    // pointer to the keychain this key is attached to
    pub keychain: Pubkey,
    // the key/wallet this key holds - matches the one in the keychain
    pub key: Pubkey,
}

// domains are needed for admin functions
#[account]
pub struct Domain {
    // max size = 32
    pub name: String,
    pub authority: Pubkey,
    pub treasury: Pubkey,
    pub keychain_cost: u64,            // the cost to add a key to a keychain
    pub bump: u8,
}

impl Domain {
    // 32 byte name
    pub const MAX_SIZE: usize = 32 + 32 + 32 + 1 + 8;
}

