use anchor_lang::prelude::*;
use crate::constant::MAX_KEYS;

// represents a user's wallet
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct UserKey {
    // size = 1 + 32 = 33
    pub key: Pubkey,
    pub verified: bool                  // initially false after existing key adds a new one, until the added key verifies
}

// the current version of the keychain
#[account]
pub struct CurrentKeyChain {
    pub name: String,
    pub num_keys: u16,
    pub domain: Pubkey,
    pub bump: u8,
    // Attach a Vector of type ItemStruct to the account.
    pub keys: Vec<UserKey>,
}

impl CurrentKeyChain {
    pub const MAX_SIZE: usize = 32 + 2 + 32 + 1 + (4 + (MAX_KEYS * 33));

    pub fn has_key(&self, key: &Pubkey) -> bool {
        for k in self.keys.iter() {
            if k.key == *key {
                return true;
            }
        }
        return false;
    }

    pub fn get_key(&mut self, key: &Pubkey) -> Option<&mut UserKey> {
        for k in self.keys.iter_mut() {
            if k.key == *key {
                return Some(k);
            }
        }
        return None;
    }

    pub fn has_verified_key(&self, key: &Pubkey) -> bool {
        for k in self.keys.iter() {
            if k.key == *key {
                return k.verified;
            }
        }
        return false;
    }
}

// older versions
#[account]
pub struct KeyChainV1 {
    pub num_keys: u16,
    pub domain: Pubkey,
    pub keys: Vec<UserKey>,
}

impl KeyChainV1 {
    pub const MAX_SIZE: usize = 2 + 32 + (4 + (MAX_KEYS * 33));
}

// a "pointer" account which points to the keychain it's attached to. prevents keys from being added ot multiple keychains within a domain
#[account]
pub struct KeyChainKey {
    // pointer to the keychain this key is attached to
    pub keychain: Pubkey,
    // the key/wallet this key holds - matches the one in the keychain
    pub key: Pubkey,
}

impl KeyChainKey {
    pub const MAX_SIZE: usize = 32 + 32;
}

#[account]
pub struct CurrentDomain {
    // max size = 32
    pub name: String,
    pub authority: Pubkey,
    pub treasury: Pubkey,
    pub keychain_cost: u64,            // the cost to add a key to a keychain
    pub bump: u8,
}

impl CurrentDomain {
    pub const MAX_SIZE: usize = 32 + 32 + 32 + 8 + 1;
}

// these accounts are for versioning - they shouldn't change

#[account]
pub struct KeyChainState {
    pub keychain_version: u8,
    pub key_version: u8,
    // the keychain this account is for
    pub keychain: Pubkey
}

impl KeyChainState {
    pub const MAX_SIZE: usize = 1 + 1 + 32;
}

#[account]
pub struct DomainState {
    pub version: u8,
    // the domain this state is for
    pub domain: Pubkey
}

impl DomainState {
    // 32 byte name
    pub const MAX_SIZE: usize = 1 + 32;
}
