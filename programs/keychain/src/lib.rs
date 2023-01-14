use anchor_lang::prelude::*;
use crate::program::Keychain;

declare_id!("KeyNfJK4cXSjBof8Tg1aEDChUMea4A7wCzLweYFRAoN");
// declare_id!("6uZ6f5k49o76Dtczsc9neRMk5foNzJ3CuwSm6QKCVp6T");

pub mod constant;
pub mod error;
pub mod context;
pub mod account;

// todo: use realloc to enable any number of keys instead of limiting to 5

use constant::*;
use error::*;
use context::*;
use account::*;

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

        // check name length <= 32
        require!(name.as_bytes().len() <= 32, KeychainError::NameTooLong);

        let domain_state = &mut ctx.accounts.domain;
        domain_state.version = CURRENT_DOMAIN_VERSION;
        domain_state.domain = CurrentDomain {
            name: name,
            authority: ctx.accounts.authority.key(),
            keychain_cost: keychain_cost,
            treasury: ctx.accounts.treasury.key(),
            bump: *ctx.bumps.get("domain").unwrap(),
        };

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
    pub fn create_keychain(ctx: Context<CreateKeychain>, keychain_name: String) -> Result <()> {
        require!(keychain_name.as_bytes().len() <= 32, KeychainError::NameTooLong);
        let mut admin = false;

        // if the signer is the same as the domain authority, then this is a domain admin
        if ctx.accounts.authority.key() == ctx.accounts.domain.domain.authority.key() {
            admin = true;
        } else {
            // otherwise, the wallet being added needs to be the signer
            require!(ctx.accounts.authority.key() == *ctx.accounts.wallet.key, KeychainError::NotSigner);
        }

        let keychain_state = &mut ctx.accounts.keychain;

        let key = UserKey {
            key: *ctx.accounts.wallet.to_account_info().key,
            verified: true
        };
        keychain_state.version = CURRENT_KEYCHAIN_VERSION;

        keychain_state.keychain = CurrentKeyChain {
            name: keychain_name,
            num_keys: 1,
            domain: *ctx.accounts.domain.to_account_info().key,
            bump: *ctx.bumps.get("keychain").unwrap(),
            keys: vec![key],
        };

        // first, verify the keychain_key derivation matches the given one (since they need to be unique)
        /*
        let (keychain_key_address, _keychain_key_bump) =
            Pubkey::find_program_address(&[ctx.accounts.keychain_key.key().as_ref(), ctx.accounts.domain.name.as_bytes(), KEYCHAIN.as_bytes()], ctx.program_id);

        if ctx.accounts.keychain_key.key() != keychain_key_address {
            msg!("derived keychain key account (pointer) doesn't match: {}", keychain_key_address);
            return Err(KeychainError::IncorrectKeyAddress.into());
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

    // upgrade an old account - doesn't do anything yet
    pub fn upgrade_keychain(ctx: Context<UpgradeKeyChain>) -> Result <()> {
        let account_data = &mut &**ctx.accounts.keychain.try_borrow_mut_data()?;
        // pull the 2nd byte to get the version
        let version = account_data[1];
        if version == 1 {
            // deserialize the current keychain from the rest of the bytes
            let current_keychain: CurrentKeyChain = CurrentKeyChain::try_from_slice(&account_data[2..])?;
            msg!("got keychain account: {}", current_keychain.name);
        } else {
            msg!("unknown keychain version : {}", version);
        }
        Ok(())
    }

    // user w/existing keychain (and verified key), adds a new (unverified) key
    pub fn add_key(ctx: Context<AddKey>, pubkey: Pubkey) -> Result <()> {
        let keychain_state = &mut ctx.accounts.keychain;
        let keychain = &mut keychain_state.keychain;

        let mut found_signer = false;
        let mut found_existing = false;
        let signer = *ctx.accounts.authority.to_account_info().key;

        // see if this is an admin
        let mut admin = false;

        // if the signer is the same as the domain authority, then this is a domain admin
        if ctx.accounts.authority.key() == ctx.accounts.domain.domain.authority.key() {
            admin = true;
        }

        // admins can add keys willy nilly
        if !admin {
            // check that the signer is in the keychain & whether this key already exists
            for user_key in &keychain.keys {
                if user_key.verified && user_key.key == signer {
                    found_signer = true;
                }
                if pubkey == user_key.key {
                    found_existing = true;
                }
            }
            require!(found_signer, KeychainError::SignerNotInKeychain);
            require!(!found_existing, KeychainError::KeyAlreadyExists);
        }

        require!(usize::from(keychain.num_keys) < MAX_KEYS, KeychainError::MaxKeys);

        // verify that the passed in key account is correct
        /*
        let (keychain_key_address, _keychain_key_bump) =
            Pubkey::find_program_address(&[pubkey.as_ref(), ctx.accounts.domain.name.as_bytes(), KEYCHAIN.as_bytes()], ctx.program_id);
        require!(keychain_key_address == pubkey, KeychainError::IncorrectKeyAddress);
         */

        // todo: figure out how to check that the key pda does NOT exist yet
        //       seems hard w/anchor as all passed in accounts need to be initialized
        //       -> for regular rust: https://soldev.app/course/program-security

        /*
        if **ctx.accounts.key.to_account_info().try_borrow_lamports()? > 0 {
            msg!("A Key account already exists: {}", pubkey);
            return Err(KeychainError::KeyAlreadyExists.into());
        }
         */


        // Build the struct.
        let player_key = UserKey {
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
        let keychain_state = &mut ctx.accounts.keychain;
        let keychain = &mut keychain_state.keychain;

        let signer = *ctx.accounts.authority.to_account_info().key;

        let mut admin = false;
        // if the signer is the same as the domain authority, then this is a domain admin
        if ctx.accounts.authority.key() == ctx.accounts.domain.domain.authority.key() {
            admin = true;
        }

        if !admin {
            let mut found_signer = false;
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
            require!(found_signer, KeychainError::SignerNotInKeychain);
        }

        let domain = &ctx.accounts.domain.domain;

        // check that the payer can pay for this
        if ctx.accounts.authority.lamports() < domain.keychain_cost {
            return Err(KeychainError::NotEnoughSol.into());
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
        let keychain_state = &mut ctx.accounts.keychain;
        let keychain = &mut keychain_state.keychain;

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
        if ctx.accounts.authority.key() == ctx.accounts.domain.domain.authority.key() {
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
            msg!("Closing key account: {}", ctx.accounts.key.key());
            let keychain_key = &mut ctx.accounts.key;
            // send the lamports for closing to the domain treasury
            keychain_key.close(ctx.accounts.treasury.to_account_info());
        }

        if keychain.num_keys == 0 {
            // close the keychain account if this is the last key
            msg!("No more keys. Destroying keychain: {}", keychain_state.key());
            // the keychain account lamports get sent to the authority that removed the last key
            keychain_state.close(ctx.accounts.authority.to_account_info());
        }

        Ok(())
    }
}





