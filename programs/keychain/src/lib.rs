use anchor_lang::prelude::*;
use crate::program::Keychain;

declare_id!("KeyNfJK4cXSjBof8Tg1aEDChUMea4A7wCzLweYFRAoN");
// declare_id!("6uZ6f5k49o76Dtczsc9neRMk5foNzJ3CuwSm6QKCVp6T");

const KEYCHAIN: &str = "keychain";

// the space for keychain pdas
const KEYCHAIN_SPACE: &str = "keychains";
// the space for keychain key pdas
const KEY_SPACE: &str = "keys";

// todo: use realloc to enable any number of keys instead of limiting to 5

#[program]
pub mod keychain {
    use anchor_lang::{AccountsClose, system_program};
    use super::*;

    use anchor_lang::solana_program::{
        program::{invoke},
        system_instruction,
    };
    use anchor_lang::solana_program::program::invoke_signed;

    pub fn create_domain(ctx: Context<CreateDomain>, name: String, keychain_cost: u64) -> Result <()> {
        ctx.accounts.domain.authority = *ctx.accounts.authority.key;
        ctx.accounts.domain.bump = *ctx.bumps.get("domain").unwrap();

        // let domain_name = name.as_bytes();
        // let mut name = [0u8; 32];
        // name[..domain_name.len()].copy_from_slice(domain_name);

        // todo: check name length <= 32

        ctx.accounts.domain.name = name;
        ctx.accounts.domain.treasury = ctx.accounts.treasury.key();
        ctx.accounts.domain.keychain_cost = keychain_cost;

        msg!("created domain account: {}", ctx.accounts.domain.key());
        Ok(())
    }

    // todo: make this callable by the domain admin. currently this is a super-admin function

    // just for closing the domain account
    pub fn close_account(ctx: Context<CloseAccount>) -> Result<()> {

        let remaining_lamports = ctx.accounts.account.lamports();

        msg!("transfering all lamports out of account (to close) {}: {}", ctx.accounts.account.key(), remaining_lamports);

        // transfer sol: https://solanacookbook.com/references/programs.html#how-to-transfer-sol-in-a-program

        // Debit from_account and credit to_account
        **ctx.accounts.account.try_borrow_mut_lamports()? -= remaining_lamports;
        **ctx.accounts.authority.try_borrow_mut_lamports()? += remaining_lamports;

        Ok(())
    }

    // if done by admin, then the authority needs to be the Domain's authority
    pub fn create_keychain(ctx: Context<CreateKeychain>, _playername: String) -> Result <()> {
        let mut admin = false;

        // if the signer is the same as the domain authority, then this is a domain admin
        if ctx.accounts.authority.key() == ctx.accounts.domain.authority.key() {
            admin = true;
        } else {
            // otherwise, the wallet being added needs to be the signer
            require!(ctx.accounts.authority.key() == *ctx.accounts.wallet.key, ErrorCode::NotSigner);
        }

        let keychain = &mut ctx.accounts.keychain;
        let key = PlayerKey {
            key: *ctx.accounts.wallet.to_account_info().key,
            verified: true
        };

        // keychain.domain = ctx.accounts.domain.key();
        // add to the keychain vector
        keychain.keys.push(key);
        keychain.num_keys = 1;

        // first, verify the keychain_key derivation matches the given one (since they need to be unique)
        /*
        let (keychain_key_address, _keychain_key_bump) =
            Pubkey::find_program_address(&[ctx.accounts.keychain_key.key().as_ref(), ctx.accounts.domain.name.as_bytes(), KEYCHAIN.as_bytes()], ctx.program_id);

        if ctx.accounts.keychain_key.key() != keychain_key_address {
            msg!("derived keychain key account (pointer) doesn't match: {}", keychain_key_address);
            return Err(ErrorCode::IncorrectKeyAddress.into());
        }
         */

        // now set up the pointer/map account
        let keychain_key = &mut ctx.accounts.key;
        keychain_key.key = ctx.accounts.wallet.key();
        keychain_key.keychain = ctx.accounts.keychain.key();

        msg!("created keychain account: {}", ctx.accounts.keychain.key());
        msg!("created key account: {}", ctx.accounts.key.key());

        Ok(())
    }

    // user w/existing keychain (and verified key), adds a new (unverified) key
    pub fn add_key(ctx: Context<AddKey>, pubkey: Pubkey) -> Result <()> {
        let keychain = &mut ctx.accounts.keychain;

        let mut found_signer = false;
        let mut found_existing = false;
        let signer = *ctx.accounts.authority.to_account_info().key;

        // check that the signer is in the keychain & whether this key already exists
        for user_key in &keychain.keys {
            if user_key.verified && user_key.key == signer {
                found_signer = true;
            }
            if pubkey == user_key.key {
                found_existing = true;
            }
        }

        require!(found_signer, ErrorCode::SignerNotInKeychain);
        require!(!found_existing, ErrorCode::KeyAlreadyExists);
        require!(usize::from(keychain.num_keys) < KeyChain::MAX_KEYS, ErrorCode::MaxKeys);

        // verify that the passed in key account is correct
        /*
        let (keychain_key_address, _keychain_key_bump) =
            Pubkey::find_program_address(&[pubkey.as_ref(), ctx.accounts.domain.name.as_bytes(), KEYCHAIN.as_bytes()], ctx.program_id);
        require!(keychain_key_address == pubkey, ErrorCode::IncorrectKeyAddress);
         */

        // todo: figure out how to check that the key pda does NOT exist yet
        //       seems hard w/anchor as all passed in accounts need to be initialized
        //       -> for regular rust: https://soldev.app/course/program-security

        /*
        if **ctx.accounts.key.to_account_info().try_borrow_lamports()? > 0 {
            msg!("A Key account already exists: {}", pubkey);
            return Err(ErrorCode::KeyAlreadyExists.into());
        }
         */


        // Build the struct.
        let player_key = PlayerKey {
            key: pubkey,
            verified: false,
        };

        // Add it to the keychain.
        keychain.keys.push(player_key);
        keychain.num_keys += 1;

        Ok(())
    }

    // user verifies a new (unverified) key on a keychain (which then becomes verified)
    pub fn verify_key(ctx: Context<VerifyKey>) -> Result <()> {
        let keychain = &mut ctx.accounts.keychain;

        let mut found_signer = false;

        let signer = *ctx.accounts.authority.to_account_info().key;

        for user_key in &mut *keychain.keys {
            if user_key.key == signer {
                found_signer = true;
                if user_key.verified {
                    msg!("key already verified: {}", user_key.key);
                } else {
                    msg!("key now verified: {}", user_key.key);
                    user_key.verified = true;
                }
            }
        }

        require!(found_signer, ErrorCode::SignerNotInKeychain);

        let domain = &ctx.accounts.domain;

        // check that the payer can pay for this
        if ctx.accounts.authority.lamports() < domain.keychain_cost {
            return Err(ErrorCode::NotEnoughSol.into());
        }

        // now set up the pointer/map account
        let keychain_key = &mut ctx.accounts.key;
        keychain_key.key = signer;
        keychain_key.keychain = ctx.accounts.keychain.key();


        // pay for this key - transfer sol to treasury
        invoke(
            &system_instruction::transfer(
                ctx.accounts.authority.key,
                &domain.treasury,
                domain.keychain_cost,
            ),
            &[
                ctx.accounts.authority.to_account_info().clone(),
                ctx.accounts.treasury.clone(),
                ctx.accounts.system_program.to_account_info().clone(),
            ],
        )?;

        Ok(())
    }

    // remove a key from a keychain
    pub fn remove_key(ctx: Context<RemoveKey>, pubkey: Pubkey) -> Result <()> {
        let keychain = &mut ctx.accounts.keychain;

        let mut found_signer = false;
        let mut remove_index = usize::MAX;

        let signer = *ctx.accounts.authority.to_account_info().key;

        let mut verified = false;
        for (index, user_key) in keychain.keys.iter().enumerate() {
        // for &mut &user_key in &keychain.keys {
            if user_key.key == signer {
                found_signer = true;
            }
            if pubkey == user_key.key {
                remove_index = index;
                verified = user_key.verified;
            }
        }

        require!(found_signer, ErrorCode::SignerNotInKeychain);
        require!(remove_index != usize::MAX, ErrorCode::KeyNotFound);


        let removed_key = keychain.keys.swap_remove(remove_index);
        msg!("removed key at index: {}: {}", remove_index, removed_key.key);

        // decrement
        keychain.num_keys -= 1;

        // now close the key account if it exists (as marked by verified)
        if verified {
            msg!("Closing key account: {}", ctx.accounts.key.key());
            let keychain_key = &mut ctx.accounts.key;
            // send the lamports for closing to the domain treasury
            keychain_key.close(ctx.accounts.treasury.to_account_info());
        }

        if keychain.num_keys == 0 {
            // close the keychain account if this is the last key
            msg!("No more keys. Destroying keychain: {}", keychain.key());
            // the keychain account lamports get sent to the authority that removed the last key
            keychain.close(ctx.accounts.authority.to_account_info());
        }

        Ok(())
    }
}

// create a domain account for admin usage

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
    domain: Account<'info, Domain>,
    #[account(mut)]
    authority: Signer<'info>,
    system_program: Program <'info, System>,

    // this will be the domain's treasury
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account()]
    treasury: AccountInfo<'info>,
}

// use to destroy a Domain, keychain, or whatever account

#[derive(Accounts)]
pub struct CloseAccount<'info> {

    // this must be the upgrade authority (super-admin) - will receive the lamports
    #[account(mut)]
    authority: Signer<'info>,

    /// CHECK: this account gets closed, authority needs to be upgrade authority
    #[account(mut)]
    account: AccountInfo<'info>,

    // from: https://docs.rs/anchor-lang/latest/anchor_lang/accounts/account/struct.Account.html
    // only allow the upgrade authority (deployer) to call this
    #[account(constraint = program.programdata_address()? == Some(program_data.key()))]
    program: Program<'info, Keychain>,
    #[account(constraint = program_data.upgrade_authority_address == Some(authority.key()))]
    program_data: Account<'info, ProgramData>,
    // system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(playername: String)]
pub struct CreateKeychain<'info> {
    // space: 8 discriminator + KeyChain::MAX_SIZE
    #[account(
        init,
        payer = authority,
        seeds = [playername.as_bytes().as_ref(), KEYCHAIN_SPACE.as_bytes().as_ref(), domain.name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
        bump,
        space = 8 + KeyChain::MAX_SIZE
    )]
    keychain: Account<'info, KeyChain>,
    #[account(
        init,
        payer = authority,
        seeds = [wallet.key().as_ref(), KEY_SPACE.as_bytes().as_ref(), domain.name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
        bump,
        space = 8 + (32 * 2)
    )]
    // the first key on this keychain
    key: Account<'info, KeyChainKey>,
    #[account()]
    domain: Account<'info, Domain>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    wallet: AccountInfo<'info>,
    #[account(mut)]
    authority: Signer<'info>,
    system_program: Program <'info, System>,
}

// Create a custom struct for us to work with.
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PlayerKey {
    // size = 1 + 32 = 33
    pub key: Pubkey,
    pub verified: bool                  // initially false after existing key adds a new one, until the added key verifies
}

// todo: might wanna store the "display" version of the playername since the account should be derived from a "normalized" version of the playername
#[account]
pub struct KeyChain {
    num_keys: u16,
    domain: Pubkey,
    // Attach a Vector of type ItemStruct to the account.
    keys: Vec<PlayerKey>,
}

impl KeyChain {
    // allow up to 3 wallets for now - 2 num_keys + 4 vector + (space(T) * amount)
    pub const MAX_KEYS: usize = 5;
    pub const MAX_SIZE: usize = 2 + 32 + (4 + (KeyChain::MAX_KEYS * 33));
}

// a "pointer" account which points to the keychain it's attached to. this is to prevent keys from being added ot multiple keychains
#[account]
pub struct KeyChainKey {
    // pointer to the keychain this key is attached to
    keychain: Pubkey,
    // the key/wallet this key holds - matches the one in the keychain
    key: Pubkey,
}

// domains are needed for admin functions
#[account]
pub struct Domain {
    // max size = 32
    name: String,
    authority: Pubkey,
    treasury: Pubkey,
    keychain_cost: u64,            // the cost to add a key to a keychain
    bump: u8,
}

impl Domain {
    // 32 byte name
    pub const MAX_SIZE: usize = 32 + 32 + 32 + 1 + 8;
}

#[derive(Accounts)]
#[instruction(pubkey: Pubkey)]
pub struct AddKey<'info> {
    #[account(mut)]
    keychain: Account<'info, KeyChain>,

    // -- this doesn't work cause anchor expects a passed in account to be initialized
    // this gets passed in but NOT initialized - just checked for existence
    // key: Account<'info, KeyChainKey>,

    /// CHECK: just reading
    #[account()]
    domain: Account<'info, Domain>,

    // this needs to be an existing (and verified) UserKey in the keychain
    #[account(mut)]
    authority: Signer<'info>,
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
    key: Account<'info, KeyChainKey>,

    // this needs to be a UserKey on the keychain
    #[account(mut)]
    authority: Signer<'info>,

    #[account(has_one = treasury)]
    domain: Account<'info, Domain>,

    /// CHECK: not sure why the address or constraint check doesn't work (see the remove key)
    #[account(mut, address = domain.treasury)]
    treasury: AccountInfo<'info>,

    system_program: Program <'info, System>,
}

#[derive(Accounts)]
#[instruction(pubkey: Pubkey)]
pub struct RemoveKey<'info> {
    #[account(mut)]
    keychain: Account<'info, KeyChain>,

    // the key account that will need to be removed
    // we close manually instead of using the close attribute since an unverified key won't have the corresponding account
    #[account(
        seeds = [pubkey.as_ref(), KEY_SPACE.as_bytes().as_ref(), domain.name.as_bytes().as_ref(), KEYCHAIN.as_bytes().as_ref()],
        bump,
        mut,
    )]
    key: Account<'info, KeyChainKey>,

    #[account(has_one = treasury)]
    domain: Account<'info, Domain>,

    // this needs to be an existing (and verified) UserKey in the keychain
    #[account(mut)]
    authority: Signer<'info>,

    // #[account(mut, constraint = *treasury.key == domain.treasury)]
    /// CHECK: not sure why the address or constraint check doesn't work, but regardless we're checking on the domain w/has_one
    #[account(mut, address = domain.treasury)]
    treasury: AccountInfo<'info>
}


#[error_code]
pub enum ErrorCode {
    #[msg("You don't have enough SOL")]
    NotEnoughSol,
    #[msg("The given key account is not the correct PDA for the given address")]
    IncorrectKeyAddress,
    #[msg("That key already exists")]
    KeyAlreadyExists,
    #[msg("You cannot add any more keys on your keychain. Remove one first")]
    MaxKeys,
    #[msg("You are not a valid signer for this keychain")]
    SignerNotInKeychain,
    #[msg("That key doesn't exist on this keychain")]
    KeyNotFound,
    #[msg("Signer is not a domain admin")]
    NotDomainAdmin,
    #[msg("Can only add wallet of signer")]
    NotSigner,

}



