use anchor_lang::prelude::*;
use crate::program::Keychain;

// keychain v1
// declare_id!("KeyNfJK4cXSjBof8Tg1aEDChUMea4A7wCzLweYFRAoN");

// keychain v2
declare_id!("Key3oJGUxKaddvMRAKbyYVbE8Pf3ycrH8hyZxa7tVCo");

// declare_id!("6uZ6f5k49o76Dtczsc9neRMk5foNzJ3CuwSm6QKCVp6T");

pub mod constant;
pub mod error;
pub mod context;
pub mod account;
mod util;

// todo: use realloc to enable any number of keys instead of limiting to 5

use constant::*;
use error::*;
use context::*;
use account::*;
use util::*;

#[program]
pub mod keychain {
    use std::borrow::BorrowMut;
    use std::ops::Deref;
    use anchor_lang::{AccountsClose, system_program};
    use super::*;

    use anchor_lang::solana_program::{
        program::{invoke},
        system_instruction,
    };
    use anchor_lang::solana_program::program::invoke_signed;

    pub fn create_domain(ctx: Context<CreateDomain>, name: String, key_cost: u64) -> Result <()> {

        // check name length <= 32
        require!(name.as_bytes().len() <= 32, KeychainError::NameTooLong);

        let is_valid_name = is_valid_name(&name);
        require!(is_valid_name, KeychainError::InvalidName);

        let domain_state = &mut ctx.accounts.domain_state;
        domain_state.version = CURRENT_DOMAIN_VERSION;
        domain_state.domain = ctx.accounts.domain.key();

        let domain = &mut ctx.accounts.domain;
        domain.name = name;
        domain.authority = *ctx.accounts.authority.key;
        domain.key_cost = key_cost;
        domain.treasury = *ctx.accounts.treasury.key;
        domain.bump = *ctx.bumps.get("domain").unwrap();
        domain.keychain_action_threshold = CURRENT_KEYCHAIN_VERSION;

        msg!("created domain account: {}", ctx.accounts.domain.key());
        Ok(())
    }

    // todo: make this callable by the domain admin. currently this is a super-admin function

    // just for closing the domain account
    pub fn close_account(ctx: Context<CloseAccount>) -> Result<()> {

        let remaining_lamports = ctx.accounts.account.lamports();

        msg!("transferring all lamports out of account (to close) {}: {}", ctx.accounts.account.key(), remaining_lamports);

        // transfer sol: https://solanacookbook.com/references/programs.html#how-to-transfer-sol-in-a-program

        // Debit from_account and credit to_account
        **ctx.accounts.account.try_borrow_mut_lamports()? -= remaining_lamports;
        **ctx.accounts.authority.try_borrow_mut_lamports()? += remaining_lamports;

        Ok(())
    }

    // if done by admin, then the authority needs to be the Domain's authority
    pub fn create_keychain(ctx: Context<CreateKeychain>, keychain_name: String) -> Result <()> {

        // we only reserve 32 bytes for the name
        require!(keychain_name.as_bytes().len() <= 32, KeychainError::NameTooLong);
        require!(keychain_name.len() >= 3, KeychainError::NameTooShort);

        let is_valid_name = is_valid_name(&keychain_name);
        require!(is_valid_name, KeychainError::InvalidName);

        let mut admin = false;

        // if the signer is the same as the domain authority, then this is a domain admin
        // for now, don't allow this. this is for when a project wants to pre-allocate or create keychains on behalf of the user
        /*
        if ctx.accounts.authority.key() == ctx.accounts.domain.authority.key() {
            admin = true;
        }
        */

        // the wallet being added needs to be the signer (ignore admin scenario)
        require!(ctx.accounts.authority.key() == *ctx.accounts.wallet.key, KeychainError::NotSigner);

        let keychain_state = &mut ctx.accounts.keychain_state;
        keychain_state.keychain_version = CURRENT_KEYCHAIN_VERSION;
        keychain_state.keychain = ctx.accounts.keychain.key();
        keychain_state.action_threshold = ctx.accounts.domain.keychain_action_threshold;

        let key = UserKey {
            key: *ctx.accounts.wallet.to_account_info().key,
            verified: true
        };

        let keychain = &mut ctx.accounts.keychain;
        keychain.name = keychain_name;
        keychain.num_keys = 1;
        keychain.domain = ctx.accounts.domain.key();
        keychain.bump = *ctx.bumps.get("keychain").unwrap();
        keychain.keys = vec![key];

        // now set up the pointer/map account
        let keychain_key = &mut ctx.accounts.key;
        keychain_key.key = ctx.accounts.wallet.key();
        keychain_key.keychain = ctx.accounts.keychain.key();

        msg!("created keychain account: {}", ctx.accounts.keychain.key());
        msg!("created key account: {}", ctx.accounts.key.key());

        Ok(())
    }

    // for testing upgrade mechanism
    pub fn create_keychain_v1(ctx: Context<CreateKeychainV1>, keychain_name: String) -> Result <()> {
        require!(keychain_name.as_bytes().len() <= 32, KeychainError::NameTooLong);

        let keychain_state = &mut ctx.accounts.keychain_state;

        // set to older version
        keychain_state.keychain_version = CURRENT_KEYCHAIN_VERSION - 1;
        keychain_state.keychain = ctx.accounts.keychain.key();

        let key = UserKey {
            key: *ctx.accounts.wallet.to_account_info().key,
            verified: true
        };

        let keychain = &mut ctx.accounts.keychain;
        keychain.num_keys = 1;
        keychain.domain = *ctx.accounts.domain.to_account_info().key;
        keychain.keys = vec![key];

        // now set up the pointer/map account
        let keychain_key = &mut ctx.accounts.key;
        keychain_key.key = ctx.accounts.wallet.key();
        keychain_key.keychain = ctx.accounts.keychain.key();

        msg!("created keychain account: {}", ctx.accounts.keychain.key());
        msg!("created key account: {}", ctx.accounts.key.key());
        Ok(())
    }

    // versioning example: upgrade an old account using the keychainstate (can only be called by super-admin)
    pub fn upgrade_keychain(ctx: Context<UpgradeKeyChain>) -> Result <()> {

        // get the current keychain version
        let keychain_state = &mut ctx.accounts.keychain_state;
        if keychain_state.keychain_version == CURRENT_KEYCHAIN_VERSION {
            msg!("keychain is already up to date");
            return Ok(());
        } else {
            if keychain_state.keychain_version == CURRENT_KEYCHAIN_VERSION - 1 {
                msg!("upgrading keychain from version 1 to version 2");

                let account_data_len = ctx.accounts.keychain.try_data_len()?;
                // first let's increase the account size by 33 bytes (32 for the name + 1 for the bump)
                let rent = Rent::get()?;
                let new_size = account_data_len + 1 + 32;

                msg!("old account size: {}, new account size: {}, rent: {}", account_data_len, new_size, rent.lamports_per_byte_year);

                let new_min_balance = rent.minimum_balance(new_size);
                let lamport_diff = new_min_balance.saturating_sub(ctx.accounts.keychain.lamports());

                msg!("new min balance: {}, lamport diff: {}", new_min_balance, lamport_diff);

                // transfer in some lamports to make up the difference in rent
                invoke(
                    &system_instruction::transfer(
                        ctx.accounts.authority.key,
                        ctx.accounts.keychain.key,
                        lamport_diff,
                    ),
                    &[
                        ctx.accounts.authority.to_account_info().clone(),
                        ctx.accounts.keychain.clone(),
                        ctx.accounts.system_program.to_account_info().clone(),
                    ],
                )?;

                // now create our new data - first reallocate

                // realloc - leave account data (assumes we'll grow the account)
                ctx.accounts.keychain.realloc(new_size, false)?;

                msg!("reallocated account");

                // first deserialize the old keychain account
                // let mut account_data = ctx.accounts.keychain.try_borrow_mut_data()?;

                let mut data = ctx.accounts.keychain.try_borrow_mut_data()?;
                let dst: &mut &[u8] = &mut &***&mut data;

                // let account_data = &mut &**ctx.accounts.keychain.try_borrow_data()?;
                let v1_keychain: KeyChainV1 = KeyChainV1::try_deserialize(dst)?;

                msg!("deserialized old account data");

                // fake data
                const bump: u8 = 33;
                let name: String = "test".to_string();

                // now we create a new one
                let new_keychain = CurrentKeyChain {
                    name,
                    num_keys: v1_keychain.num_keys,
                    domain: v1_keychain.domain,
                    bump,
                    keys: v1_keychain.keys,
                };

                // bump the version
                keychain_state.keychain_version = CURRENT_KEYCHAIN_VERSION;

                let ddst: &mut [u8] = &mut data;
                let mut cursor = std::io::Cursor::new(ddst);
                new_keychain.try_serialize(&mut cursor)?;

                msg!("migrated keychain to version: {}", CURRENT_KEYCHAIN_VERSION);

            } else {
                msg!("keychain version is not supported: {}", keychain_state.keychain_version);
                return Err(KeychainError::InvalidKeychainVersion.into());
            }
        }

        Ok(())
    }

    // user w/existing keychain (and verified key), adds a new (unverified) key
    pub fn add_key(ctx: Context<AddKey>, key: Pubkey) -> Result <()> {
        let keychain = &mut ctx.accounts.keychain;

        let signer = *ctx.accounts.authority.to_account_info().key;

        require!(!keychain.has_key(&key), KeychainError::KeyAlreadyExists);
        require!(usize::from(keychain.num_keys) < MAX_KEYS, KeychainError::MaxKeys);

        // check that there isn't already a pending action
        let keychain_state = &mut ctx.accounts.keychain_state;
        require!(keychain_state.pending_action.is_none(), KeychainError::PendingActionExists);

        // signer automatically casts vote to approve
        let mut pending_action = PendingKeyChainAction::new(KeyChainActionType::AddKey, key);
        let authority_index = keychain.index_of(&signer).unwrap();
        pending_action.votes.set_index(authority_index);

        // don't even bother checking the threshold cause let's not ever allow just 1 vote to add a key

        // todo: MIGHT wanna add the account as an optional to mae sure it doesn't exist yet: https://solana.stackexchange.com/questions/3745/anchors-init-if-constraint-for-the-optional-initialization-of-accounts

        let player_key = UserKey {
            key,
            verified: false,
        };

        // Add it to the keychain.
        keychain.keys.push(player_key);
        keychain.num_keys += 1;

        Ok(())
    }

    // user verifies a new (unverified) key on a keychain - potentially becomes linked but based on votes
    pub fn verify_key(ctx: Context<VerifyKey>) -> Result <()> {
        let keychain = &mut ctx.accounts.keychain;
        let domain = &ctx.accounts.domain;
        let signer = *ctx.accounts.authority.to_account_info().key;

        // set up the pointer/map account
        let keychain_key = &mut ctx.accounts.key;
        keychain_key.key = ctx.accounts.authority.key();
        keychain_key.keychain = keychain.key();

        // check that the payer can pay for this
        if ctx.accounts.authority.lamports() < domain.key_cost {
            return Err(KeychainError::NotEnoughSol.into());
        }

        // pay for this key - transfer sol to treasury
        invoke(
            &system_instruction::transfer(
                ctx.accounts.authority.key,
                &domain.treasury,
                domain.key_cost,
            ),
            &[
                ctx.accounts.authority.to_account_info().clone(),
                ctx.accounts.treasury.clone(),
                ctx.accounts.system_program.to_account_info().clone(),
            ],
        )?;

        // todo: extract this out to a function

        // update the votes
        let action_threshold = ctx.accounts.keychain_state.action_threshold;
        let pending_action = ctx.accounts.keychain_state.pending_action.as_mut().unwrap();
        let authority_index = keychain.index_of(&signer).unwrap();
        pending_action.votes.set_index(authority_index);

        // check if we've reached the threshold - 0 means all keys must vote
        if (action_threshold > 0 && pending_action.votes.count_set() >= action_threshold) ||
            (action_threshold == 0 && u16::from(pending_action.votes.count_set()) == keychain.num_keys) {

            // we've reached the threshold - remove the pending action
            let keychain_state = &mut ctx.accounts.keychain_state;
            keychain_state.pending_action = None;

            // and mark the key as verified
            let mut added_key = keychain.get_key(&signer).unwrap();
            added_key.verified = true;
        }

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
        // see if this is an admin
        let mut admin = false;

        // if the signer is the same as the domain authority, then this is a domain admin
        if ctx.accounts.authority.key() == ctx.accounts.domain.authority.key() {
            admin = true;
        }

        require!(remove_index != usize::MAX, KeychainError::KeyNotFound);

        // admins can remove keys
        if !admin {
            require!(found_signer, KeychainError::SignerNotInKeychain);
        }

        let removed_key = keychain.keys.swap_remove(remove_index);
        msg!("removed key at index: {}: {}", remove_index, removed_key.key);

        // decrement
        keychain.num_keys -= 1;

        // now close the key account if it exists (as marked by verified)
        if verified {

            if ctx.accounts.key.is_none() {
                // then the key account wasn't passed in but needs to have been
                return Err(KeychainError::MissingKeyAccount.into());
            }

            msg!("Closing key account: {}", ctx.accounts.key.as_ref().unwrap().key());
            let keychain_key = &mut ctx.accounts.key;
            // send the lamports for closing to the domain treasury
            keychain_key.close(ctx.accounts.treasury.to_account_info())?;

        } else {
            // probably not necessary, but if the key wasn't verified, then a key account shouldn't have been passed in
            // if ctx.accounts.key.is_some()  {
                // then the key account was passed in but shouldn't have been
                // return Err(KeychainError::InvalidKeyAccount.into());
            // }
        }

        if keychain.num_keys == 0 {
            // close the keychain account if this is the last key
            msg!("No more keys. Destroying keychain: {}", keychain.key());

            // the keychain and associated state account lamports get sent to the authority that removed the last key
            keychain.close(ctx.accounts.authority.to_account_info())?;
            ctx.accounts.keychain_state.close(ctx.accounts.authority.to_account_info())?;
        }

        Ok(())
    }
}





