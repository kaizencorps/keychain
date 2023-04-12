// create a domain account for admin usage

use anchor_lang::prelude::*;
use crate::account::*;
use crate::program::Keychain;
use crate::constant::*;
use crate::error::*;

#[derive(Accounts)]
#[instruction(name: String)]
pub struct CreateDomain<'info> {
    // space: 8 discriminator + size(Domain) = 40 +
    #[account(
    init,
    payer = authority,
    seeds = [name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
    bump,
    space = 8 + CurrentDomain::MAX_SIZE,
    )]
    pub domain: Account<'info, CurrentDomain>,

    #[account(
    init,
    payer = authority,
    seeds = [DOMAIN_STATE.as_bytes().as_ref(), name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
    bump,
    space = 8 + DomainState::MAX_SIZE,
    )]
    pub domain_state: Account<'info, DomainState>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program <'info, System>,

    // this will be the domain's treasury
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account()]
    pub treasury: AccountInfo<'info>,
}

// used to destroy a Domain, keychain, key or whatever keychain-owned account we want. note: use with extreme caution

#[derive(Accounts)]
pub struct CloseAccount<'info> {

    // this must be the upgrade authority (super-admin) - will receive the lamports
    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: this account gets closed, authority needs to be upgrade authority
    #[account(mut)]
    pub account: AccountInfo<'info>,

    // from: https://docs.rs/anchor-lang/latest/anchor_lang/accounts/account/struct.Account.html
    // only allow the upgrade authority (deployer) to call this
    #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
    pub program: Program<'info, Keychain>,

    #[account(constraint = program_data.upgrade_authority_address == Some(authority.key()))]
    pub program_data: Account<'info, ProgramData>,
}

#[derive(Accounts)]
#[instruction(keychain_name: String)]
pub struct CreateKeychain<'info> {

    // space: 8 discriminator + KeyChain::MAX_SIZE
    #[account(
    init,
    payer = authority,
    seeds = [keychain_name.as_bytes().as_ref(), KEYCHAIN_SPACE.as_bytes().as_ref(), domain.name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
    bump,
    space = 8 + CurrentKeyChain::MAX_SIZE
    )]
    pub keychain: Account<'info, CurrentKeyChain>,

    #[account(
    init,
    payer = authority,
    seeds = [keychain.key().as_ref(), KEYCHAIN_STATE_SPACE.as_bytes().as_ref(), domain.name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
    bump,
    space = 8 + KeyChainState::MAX_SIZE
    )]
    pub keychain_state: Account<'info, KeyChainState>,

    #[account(
    init,
    payer = authority,
    seeds = [wallet.key().as_ref(), KEY_SPACE.as_bytes().as_ref(), domain.name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
    bump,
    space = 8 + KeyChainKey::MAX_SIZE
    )]
    // the first key on this keychain
    pub key: Account<'info, KeyChainKey>,

    #[account()]
    pub domain: Account<'info, CurrentDomain>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    pub wallet: AccountInfo<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program <'info, System>,
}

// just for testing - only super-admin can call this
#[derive(Accounts)]
#[instruction(keychain_name: String)]
pub struct CreateKeychainV1<'info> {

    // space: 8 discriminator + KeyChain::MAX_SIZE
    #[account(
    init,
    payer = authority,
    seeds = [keychain_name.as_bytes().as_ref(), KEYCHAIN_SPACE.as_bytes().as_ref(), domain.name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
    bump,
    space = 8 + KeyChainV1::MAX_SIZE
    )]
    pub keychain: Account<'info, KeyChainV1>,

    #[account(
    init,
    payer = authority,
    seeds = [keychain.key().as_ref(), KEYCHAIN_STATE_SPACE.as_bytes().as_ref(), domain.name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
    bump,
    space = 8 + KeyChainState::MAX_SIZE
    )]
    pub keychain_state: Account<'info, KeyChainState>,

    #[account(
    init,
    payer = authority,
    seeds = [wallet.key().as_ref(), KEY_SPACE.as_bytes().as_ref(), domain.name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
    bump,
    space = 8 + KeyChainKey::MAX_SIZE
    )]
    // the first key on this keychain
    pub key: Account<'info, KeyChainKey>,

    #[account()]
    pub domain: Account<'info, CurrentDomain>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    pub wallet: AccountInfo<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program <'info, System>,

    // only super-admin can call this (it's for testing)
    #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
    pub program: Program<'info, Keychain>,

    #[account(constraint = program_data.upgrade_authority_address == Some(authority.key()))]
    pub program_data: Account<'info, ProgramData>,
}

// only super-admin can call this (for testing)
#[derive(Accounts)]
pub struct UpgradeKeyChain<'info> {

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK:
    #[account(mut)]
    pub keychain: AccountInfo<'info,>,

    #[account(mut, has_one = keychain)]
    pub keychain_state: Account<'info, KeyChainState>,

    pub system_program: Program <'info, System>,

    #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
    pub program: Program<'info, Keychain>,

    #[account(constraint = program_data.upgrade_authority_address == Some(authority.key()))]
    pub program_data: Account<'info, ProgramData>,
}

#[derive(Accounts)]
#[instruction(key: Pubkey)]
pub struct AddKey<'info> {

    #[account(mut, constraint = keychain.has_key(&authority.key()) @ KeychainError::NotAuthorized)]
    pub keychain: Account<'info, CurrentKeyChain>,

    #[account(mut, has_one = keychain)]
    pub keychain_state: Account<'info, KeyChainState>,

    #[account(mut, constraint = keychain.has_key(&authority.key()) @ KeychainError::NotAuthorized)]
    pub authority: Signer<'info>,

}

#[derive(Accounts)]
pub struct VotePendingAction<'info> {

    // check that the key being verified has already been added to the keychain & check auth on the authority below
    #[account(mut, constraint = keychain.has_key(&authority.key()) @ KeychainError::KeyNotFound)]
    pub keychain: Account<'info, CurrentKeyChain>,

    #[account(mut, has_one = keychain, constraint = keychain_state.has_pending_action() @ KeychainError::NoPendingAction)]
    pub keychain_state: Account<'info, KeyChainState>,

    // this is required if the pending action is a key removal
    #[account(
    seeds = [keychain_state.pending_action.as_ref().unwrap().key.as_ref(), KEY_SPACE.as_bytes().as_ref(), keychain.domain.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
    bump,
    mut,
    )]
    pub keychain_key: Option<Account<'info, KeyChainKey>>,

    #[account(mut, constraint = keychain.has_key(&authority.key()) @ KeychainError::NotAuthorized)]
    pub authority: Signer<'info>,

}

#[derive(Accounts)]
pub struct VerifyKey<'info> {

    #[account(has_one = treasury @KeychainError::InvalidTreasury)]
    pub domain: Account<'info, CurrentDomain>,

    #[account(mut)]
    pub keychain: Account<'info, CurrentKeyChain>,

    #[account(mut, has_one = keychain, constraint = keychain_state.has_pending_action_type(KeyChainActionType::AddKey) @ KeychainError::NoPendingAction)]
    pub keychain_state: Account<'info, KeyChainState>,

    // the key account gets created here
    #[account(
    init,
    payer = authority,
    seeds = [&authority.key().as_ref(), KEY_SPACE.as_bytes().as_ref(), domain.name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
    bump,
    space = 8 + (32 * 2)
    )]
    pub keychain_key: Account<'info, KeyChainKey>,

    // check that the signer is the pending key
    #[account(mut, constraint = keychain_state.has_pending_action_key(&authority.key()) @ KeychainError::InvalidVerifier)]
    pub authority: Signer<'info>,

    /// CHECK: just sending lamports
    #[account(mut, address = domain.treasury, constraint = treasury.key() == domain.treasury @ KeychainError::InvalidTreasury)]
    pub treasury: AccountInfo<'info>,

    pub system_program: Program <'info, System>,
}

#[derive(Accounts)]
#[instruction(key: Pubkey)]
pub struct RemoveKey<'info> {

    // make sure the key we're removing exists on the keychain
    #[account(mut, constraint = keychain.has_key(&key) @ KeychainError::KeyNotFound)]
    pub keychain: Account<'info, CurrentKeyChain>,

    // include the state in case the keychain is closed; make sure there's no pending action
    #[account(mut, has_one = keychain, constraint = !keychain_state.has_pending_action() @ KeychainError::PendingActionExists)]
    pub keychain_state: Account<'info, KeyChainState>,

    // the key account that will need to be removed
    // we close manually instead of using the close attribute since an unverified key won't have the corresponding account
    #[account(
    seeds = [key.as_ref(), KEY_SPACE.as_bytes().as_ref(), keychain.domain.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
    bump,
    mut,
    )]
    pub keychain_key: Account<'info, KeyChainKey>,

    #[account(mut, constraint = keychain.has_key(&authority.key()) @ KeychainError::NotAuthorized)]
    pub authority: Signer<'info>,

    /*
    // #[account(has_one = treasury OR constraint = domain.treasury == treasury.key() @ KeychainError::InvalidTreasury)]
    #[account()]
    pub domain: Account<'info, CurrentDomain>,

    /// CHECK: just sending lamports
    #[account(mut, address = domain.treasury, constraint = treasury.key() == domain.treasury @ KeychainError::InvalidTreasury)]
    pub treasury: AccountInfo<'info>
     */
}
