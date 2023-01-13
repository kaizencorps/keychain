// create a domain account for admin usage

use anchor_lang::prelude::*;
use crate::account::*;


#[derive(Accounts)]
#[instruction(name: String)]
pub struct CreateDomain<'info> {
    // space: 8 discriminator + size(Domain) = 40 +
    #[account(
    init,
    payer = authority,
    seeds = [name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
    bump,
    space = 8 + Domain::MAX_SIZE,
    )]
    pub domain: Account<'info, Domain>,
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
#[instruction(keychanin_name: String)]
pub struct CreateKeychain<'info> {
    // space: 8 discriminator + KeyChain::MAX_SIZE
    #[account(
    init,
    payer = authority,
    seeds = [keychanin_name.as_bytes().as_ref(), KEYCHAIN_SPACE.as_bytes().as_ref(), domain.name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
    bump,
    space = 8 + KeyChain::MAX_SIZE
    )]
    pub keychain: Account<'info, KeyChain>,
    #[account(
    init,
    payer = authority,
    seeds = [wallet.key().as_ref(), KEY_SPACE.as_bytes().as_ref(), domain.name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
    bump,
    space = 8 + (32 * 2)
    )]
    // the first key on this keychain
    pub key: Account<'info, KeyChainKey>,
    #[account()]
    pub domain: Account<'info, Domain>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub wallet: AccountInfo<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program <'info, System>,
}

#[derive(Accounts)]
#[instruction(pubkey: Pubkey)]
pub struct AddKey<'info> {
    #[account(mut)]
    pub keychain: Account<'info, KeyChain>,

    // -- this doesn't work cause anchor expects a passed in account to be initialized
    // this gets passed in but NOT initialized - just checked for existence
    // key: Account<'info, KeyChainKey>,

    /// CHECK: just reading
    #[account()]
    pub domain: Account<'info, Domain>,

    // this needs to be an existing (and verified) UserKey in the keychain
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct VerifyKey<'info> {
    #[account(mut)]
    pub keychain: Account<'info, KeyChain>,

    // the key account gets created here
    #[account(
    init,
    payer = authority,
    seeds = [authority.key().as_ref(), KEY_SPACE.as_bytes().as_ref(), domain.name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
    bump,
    space = 8 + (32 * 2)
    )]
    pub key: Account<'info, KeyChainKey>,

    // this needs to be a UserKey on the keychain
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(has_one = treasury)]
    pub domain: Account<'info, Domain>,

    /// CHECK: not sure why the address or constraint check doesn't work (see the remove key)
    #[account(mut, address = domain.treasury)]
    pub treasury: AccountInfo<'info>,

    pub system_program: Program <'info, System>,
}

#[derive(Accounts)]
#[instruction(pubkey: Pubkey)]
pub struct RemoveKey<'info> {
    #[account(mut)]
    pub keychain: Account<'info, KeyChain>,

    // the key account that will need to be removed
    // we close manually instead of using the close attribute since an unverified key won't have the corresponding account
    #[account(
    seeds = [pubkey.as_ref(), KEY_SPACE.as_bytes().as_ref(), domain.name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
    bump,
    mut,
    )]
    pub key: Account<'info, KeyChainKey>,

    #[account(has_one = treasury)]
    pub domain: Account<'info, Domain>,

    // this needs to be an existing (and verified) UserKey in the keychain
    #[account(mut)]
    pub authority: Signer<'info>,

    // #[account(mut, constraint = *treasury.key == domain.treasury)]
    /// CHECK: not sure why the address or constraint check doesn't work, but regardless we're checking on the domain w/has_one
    #[account(mut, address = domain.treasury)]
    pub treasury: AccountInfo<'info>
}
