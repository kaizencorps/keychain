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

// used to destroy a Domain, keychain, or whatever account

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
    // system_program: Program<'info, System>,
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
}

// for now let anyone call this method
#[derive(Accounts)]
pub struct UpgradeKeyChain<'info> {

    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK:
    #[account(mut)]
    pub keychain: AccountInfo<'info,>,

    #[account(mut, has_one = keychain)]
    pub keychain_state: Account<'info, KeyChainState>,

    pub system_program: Program <'info, System>,

}

/*
#[derive(Accounts)]
pub struct UpgradeOldKeyChain<'info> {

    #[account(mut)]
    pub user: Signer<'info>,

    // reallocate size to include room for the name + state info (verison)
    /// CHECK:
    #[account(
    mut,
    realloc = 1 + 32 + 2 + 32 + (4 + (MAX_KEYS * 33)),
    realloc::payer = user,
    realloc::zero = true
    )]
    pub keychain: Account<'info, OldKeyChain>,
    pub system_program: Program <'info, System>,
}
 */

#[derive(Accounts)]
#[instruction(pubkey: Pubkey)]
pub struct AddKey<'info> {
    #[account(mut)]
    pub keychain: Account<'info, CurrentKeyChain>,

    // -- this doesn't work cause anchor expects a passed in account to be initialized
    // this gets passed in but NOT initialized - just checked for existence
    // key: Account<'info, KeyChainKey>,

    /// CHECK: just reading
    #[account()]
    pub domain: Account<'info, CurrentDomain>,

    // this needs to be an existing (and verified) UserKey in the keychain
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct VerifyKey<'info> {
    #[account(mut)]
    pub keychain: Account<'info, CurrentKeyChain>,

    // the key account gets created here
    #[account(
    init,
    payer = authority,
    seeds = [user_key.key().as_ref(), KEY_SPACE.as_bytes().as_ref(), domain.name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
    bump,
    space = 8 + (32 * 2)
    )]
    pub key: Account<'info, KeyChainKey>,

    /// CHECK: user's wallet, gets checked against corresponding key in keychain
    // this needs to be a UserKey on the keychain
    #[account(constraint = keychain.has_key(&user_key.key()))]
    pub user_key: AccountInfo<'info>,

    // will just be same as user_key unless it's the domain admin
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(constraint = domain.treasury == treasury.key() @ KeychainError::WrongTreasury)]
    pub domain: Account<'info, CurrentDomain>,

    /// CHECK: gets checked by domain constraint
    #[account(mut, address = domain.treasury)]
    pub treasury: AccountInfo<'info>,

    pub system_program: Program <'info, System>,
}

#[derive(Accounts)]
#[instruction(pubkey: Pubkey)]
pub struct RemoveKey<'info> {
    #[account(mut)]
    pub keychain: Account<'info, CurrentKeyChain>,

    // the key account that will need to be removed
    // we close manually instead of using the close attribute since an unverified key won't have the corresponding account
    #[account(
    seeds = [pubkey.as_ref(), KEY_SPACE.as_bytes().as_ref(), domain.name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
    bump,
    mut,
    )]
    pub key: Account<'info, KeyChainKey>,

     // include the state in case the keychain is closed
    #[account(mut, has_one = keychain)]
    pub keychain_state: Account<'info, KeyChainState>,

    // #[account(has_one = treasury)]
    #[account(constraint = domain.treasury == treasury.key() @ KeychainError::WrongTreasury)]
    pub domain: Account<'info, CurrentDomain>,

    // this needs to be an existing (and verified) UserKey in the keychain
    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: gets checked by domain constraint
    #[account(mut, address = domain.treasury)]
    pub treasury: AccountInfo<'info>
}
