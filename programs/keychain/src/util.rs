use anchor_lang::prelude::Pubkey;
use crate::account::{CurrentKeyChain, KeyChainState};

// checks that a given string contains only lowercase letters and numbers, with a few special characters
pub fn is_valid_name(s: &str) -> bool {
    s.chars().all(|c| !c.is_whitespace()  && (c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_'))
}

pub fn set_vote(keychain: &CurrentKeyChain, keychain_state: &mut KeyChainState, signer: &Pubkey, vote: bool) {
    let pending_action = keychain_state.pending_action.as_mut().unwrap();
    let authority_index = keychain.index_of(signer).unwrap() as u8;
    if vote {
        pending_action.votes.set_index(authority_index);
    } else {
        pending_action.votes.unset_index(authority_index);
    }
}
