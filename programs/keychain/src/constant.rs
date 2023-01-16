
// root seed
pub const KEYCHAIN: &str = "keychain";
// the space for keychain pdas
pub const KEYCHAIN_SPACE: &str = "keychains";
// the space for keychain key pdas
pub const KEY_SPACE: &str = "keys";
// the space for domain state
pub const DOMAIN_STATE: &str = "domain_state";
// the space for keychain state
pub const KEYCHAIN_STATE_SPACE: &str  = "keychain_states";


pub const CURRENT_KEYCHAIN_VERSION: u8 = 2;
pub const CURRENT_KEY_VERSION: u8 = 0;
pub const CURRENT_DOMAIN_VERSION: u8 = 1;

// allow up to 3 wallets for now - 2 num_keys + 4 vector + (space(T) * amount)
pub const MAX_KEYS: usize = 5;
