use anchor_lang::prelude::*;
use crate::constant::MAX_KEYS;

// represents a user's wallet - previously stored a verified field, but was moved to keychain state
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct UserKey {
    pub key: Pubkey,
}

// the current version of the keychain
#[account]
pub struct CurrentKeyChain {
    pub name: String,
    pub num_keys: u16,  // number of keys (linked or not)
    pub domain: String,
    pub bump: u8,
    // Attach a Vector of type ItemStruct to the account - user keys are all verified
    pub keys: Vec<UserKey>,
}

impl CurrentKeyChain {
    pub const MAX_SIZE: usize =
            32 +    // name
            2 +     // num_keys
            32 +    // domain
            1 +     // bump
            (4 + (MAX_KEYS * 32)) +   // keys
            192;     // extra space

    pub fn has_key(&self, key: &Pubkey) -> bool {
        for k in self.keys.iter() {
            if k.key == *key {
                return true;
            }
        }
        return false;
    }

    pub fn index_of(&self, key: &Pubkey) -> Option<usize> {
        for (i, k) in self.keys.iter().enumerate() {
            if k.key == *key {
                return Some(i);
            }
        }
        return None;
    }

    pub fn get_key(&mut self, key: &Pubkey) -> Option<&mut UserKey> {
        for k in self.keys.iter_mut() {
            if k.key == *key {
                return Some(k);
            }
        }
        return None;
    }

    pub fn add_key(&mut self, key: Pubkey) {
        self.keys.push(UserKey { key });
        self.num_keys += 1;
    }

    pub fn remove_key(&mut self, key: Pubkey) {
        let key_index = self.index_of(&key).unwrap();
        self.keys.swap_remove(key_index);
        self.num_keys -= 1;
    }

}

// older versions
#[account]
pub struct KeyChainV1 {
    pub num_keys: u16,
    pub domain: String,
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
    pub const MAX_SIZE: usize =
            32 +    // keychain
            32 +    // key
            192;     // extra space in case we need to store more data later;
}

#[account]
pub struct CurrentDomain {
    // max size = 32
    pub name: String,
    pub authority: Pubkey,
    pub treasury: Pubkey,
    pub bump: u8,
    // params
    pub key_cost: u64,            // the cost to add a key to a keychain
    pub keychain_action_threshold: u8,            // the number of keys required to verify a new key (0 = all keys)
}

impl CurrentDomain {
    pub const MAX_SIZE: usize =
            32 +    // name
            32 +    // authority
            32 +    // treasury
            8 +     // key_cost
            1 +     // bump
            1 +
            1 +     // threshold
            192;  // extra storage

}

////// these accounts are for versioning - they shouldn't change

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub enum KeyChainActionType {
    AddKey,
    RemoveKey,
}

// this stores the versioning info AND pending actions, and could possibly be used to store settings or other data in the future
#[account]
pub struct KeyChainState {
    pub keychain_version: u8,
    // the keychain this account is for
    pub keychain: Pubkey,
    pub pending_action: Option<PendingKeyChainAction>,
    pub action_threshold: u8
}

impl KeyChainState {
    pub const MAX_SIZE: usize =
        1 +                 // keychain_version
        32 +                // keychain
        1 +                // action_threshold
        1 + PendingKeyChainAction::MAX_SIZE       // pending_action
        + 192;              // extra space

    pub fn has_pending_action_type(&self, action_type: KeyChainActionType) -> bool {
        self.pending_action.is_some() && self.pending_action.as_ref().unwrap().action_type == action_type
    }

    pub fn has_pending_action(&self) -> bool {
        self.pending_action.is_some()
    }

    pub fn has_pending_action_key(&self, key: &Pubkey) -> bool {
        self.pending_action.is_some() && self.pending_action.as_ref().unwrap().key == *key
    }

    pub fn pending_key(self) -> Option<Pubkey> {
        if self.pending_action.is_some() {
            return Some(self.pending_action.unwrap().key.clone());
        }
        return None;
    }
}

// simple bitset for up to 8 bits
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub struct SmallBitSet {
    data: u8,
}

impl SmallBitSet {
    fn new() -> Self {
        SmallBitSet { data: 0 }
    }

    pub fn count_set(&self) -> u8 {
        self.data.count_ones() as u8
    }

    pub fn set_index(&mut self, index: u8) {
        self.data |= 1 << index;
    }

    pub fn unset_index(&mut self, index: u8) {
        self.data &= !(1 << index);
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub struct PendingKeyChainAction {
    pub action_type: KeyChainActionType,
    pub key: Pubkey,
    pub verified: bool,
    pub votes: SmallBitSet
}

impl PendingKeyChainAction {
    pub const MAX_SIZE: usize = 1 + 32 + 1 + 1;

    pub fn new(action_type: KeyChainActionType, key: Pubkey) -> Self {
        Self { action_type, key, verified: false, votes: SmallBitSet::new() }
    }

    pub fn verify(&mut self) {
        self.verified = true;
    }

    pub fn vote(&mut self, index: u8, vote: bool) {
        if vote {
            self.votes.set_index(index);
        } else {
            self.votes.unset_index(index);
        }
    }

    pub fn count_votes(&self) -> u8 {
        self.votes.count_set()
    }
}

// strictly for versioning info

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
