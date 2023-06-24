use anchor_lang::solana_program::system_instruction;
use mpl_token_auth_rules::payload::{Payload, PayloadType, SeedsVec};
use mpl_token_metadata::state::PayloadKey;
use crate::*;


#[inline(never)]
pub fn assert_decode_metadata<'info>(
    nft_mint: &Account<'info, Mint>,
    metadata_account: &AccountInfo<'info>,
) -> Result<Metadata> {
    let (key, _) = Pubkey::find_program_address(
        &[
            mpl_token_metadata::state::PREFIX.as_bytes(),
            mpl_token_metadata::id().as_ref(),
            nft_mint.key().as_ref(),
        ],
        &mpl_token_metadata::id(),
    );
    if key != *metadata_account.key {
        require!(true, YardsaleError::BadMetadata);
    }
    // Check account owner (redundant because of find_program_address above, but why not).
    if *metadata_account.owner != mpl_token_metadata::id() {
        return Err(error!(YardsaleError::BadMetadata));
    }

    Ok(Metadata::from_account_info(metadata_account)?)
}


// transfers an item out of the listing's token account and closes it
pub fn transfer_item_and_close<'a, 'b>(listing: &Box<Account<'a, Listing>>,
                                       listing_item_token_ai: AccountInfo<'b>,
                                       to_token_ai: AccountInfo<'b>,
                                       lamports_claimer_ai: AccountInfo<'a>,
                                       token_program: AccountInfo<'a>) -> Result<()>
    where 'a: 'b, 'b: 'a {

    let seeds = &[
        listing.item.as_ref(),
        LISTINGS.as_bytes().as_ref(),
        listing.keychain.as_bytes().as_ref(),
        listing.domain.as_bytes().as_ref(),
        YARDSALE.as_bytes().as_ref(),
        &[listing.bump],
    ];
    let signer = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: listing_item_token_ai.clone(),
        to: to_token_ai.clone(),
        authority: listing.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        token_program.clone(),
        cpi_accounts,
        signer);
    token::transfer(cpi_ctx, 1)?;

    // now we can close the item listing's token account - we do this in code so we can specify who gets the rent lamports
    let cpi_close_accounts = CloseAccount {
        account: listing_item_token_ai.clone(),
        destination: lamports_claimer_ai.clone(),
        authority: listing.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(token_program.clone(),
                                              cpi_close_accounts, signer);
    token::close_account(cpi_ctx)?;

    Ok(())
}


pub fn make_purchase<'info>(listing: &Box<Account<'info, Listing>>,
                            buyer: &AccountInfo<'info>,
                            proceeds: &Option<AccountInfo<'info>>,
                            proceeds_token: &Option<Account<'info, TokenAccount>>,
                            buyer_currency_token: &Option<Account<'info, TokenAccount>>,
                            system_program: &Program <'info, System>,
                            token_program: &Program <'info, Token>,
) -> Result<()> {

// check that the buyer has enough funds to purchase the item
    if listing.currency == NATIVE_MINT {
        require!(buyer.lamports() > listing.price, YardsaleError::InsufficientFunds);
        require!(proceeds.is_some(), YardsaleError::ProceedsAccountNotSpecified);
        // proper account matching listing gets checked in the constraint

        // pay for the item with sol
        invoke(
            &system_instruction::transfer(
                buyer.key,
                &listing.proceeds,
                listing.price,
            ),
            &[
                buyer.clone(),
                proceeds.as_ref().unwrap().clone(),
                system_program.to_account_info().clone(),
            ],
        )?;
    } else {
        require!(buyer_currency_token.is_some(), YardsaleError::FundingAccountNotSpecified);
        require!(buyer_currency_token.as_ref().unwrap().amount >= listing.price, YardsaleError::InsufficientFunds);
        require!(proceeds_token.is_some(), YardsaleError::ProceedsAccountNotSpecified);
        // proper account matching listing gets checked in the constraint

        // pay for the item with spl token
        let cpi_accounts = Transfer {
            from: buyer_currency_token.as_ref().unwrap().to_account_info(),
            to: proceeds_token.as_ref().unwrap().to_account_info(),
            authority: buyer.clone(),
        };
        let cpi_ctx = CpiContext::new(token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, listing.price)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn send_pnft<'info>(

    authority_and_owner: &AccountInfo<'info>,
    //(!) payer can't carry data, has to be a normal KP:
    // https://github.com/solana-labs/solana/blob/bda0c606a19ce1cc44b5ab638ff0b993f612e76c/runtime/src/system_instruction_processor.rs#L197
    payer: &AccountInfo<'info>,
    source_ata: &Account<'info, TokenAccount>,
    dest_ata: &Account<'info, TokenAccount>,
    dest_owner: &AccountInfo<'info>,
    nft_mint: &Account<'info, Mint>,
    nft_metadata: &UncheckedAccount<'info>,
    nft_edition: &UncheckedAccount<'info>,
    system_program: &Program<'info, System>,
    token_program: &Program<'info, Token>,
    ata_program: &Program<'info, AssociatedToken>,
    instructions: &UncheckedAccount<'info>,
    owner_token_record: &UncheckedAccount<'info>,
    dest_token_record: &UncheckedAccount<'info>,
    authorization_rules_program: &UncheckedAccount<'info>,
    rules_acc: Option<&AccountInfo<'info>>,
    authorization_data: Option<AuthorizationDataLocal>,
    //if passed, use signed_invoke() instead of invoke()
    program_signer: Option<&Box<Account<'info, Listing>>>,
) -> Result<()> {

    let mut builder = TransferBuilder::new();

    builder
        .authority(*authority_and_owner.key)
        .token_owner(*authority_and_owner.key)
        .token(source_ata.key())
        .destination_owner(*dest_owner.key)
        .destination(dest_ata.key())
        .mint(nft_mint.key())
        .metadata(nft_metadata.key())
        .edition(nft_edition.key())
        .payer(*payer.key);
    let mut account_infos = vec![
        //   0. `[writable]` Token account
        source_ata.to_account_info(),
        //   1. `[]` Token account owner
        authority_and_owner.to_account_info(),
        //   2. `[writable]` Destination token account
        dest_ata.to_account_info(),
        //   3. `[]` Destination token account owner
        dest_owner.to_account_info(),
        //   4. `[]` Mint of token asset
        nft_mint.to_account_info(),
        //   5. `[writable]` Metadata account
        nft_metadata.to_account_info(),
        //   6. `[optional]` Edition of token asset
        nft_edition.to_account_info(),
        //   7. `[signer] Transfer authority (token or delegate owner)
        authority_and_owner.to_account_info(),
        //   8. `[optional, writable]` Owner record PDA
        //passed in below, if needed
        //   9. `[optional, writable]` Destination record PDA
        //passed in below, if needed
        //   10. `[signer, writable]` Payer
        payer.to_account_info(),
        //   11. `[]` System Program
        system_program.to_account_info(),
        //   12. `[]` Instructions sysvar account
        instructions.to_account_info(),
        //   13. `[]` SPL Token Program
        token_program.to_account_info(),
        //   14. `[]` SPL Associated Token Account program
        ata_program.to_account_info(),
        //   15. `[optional]` Token Authorization Rules Program
        //passed in below, if needed
        //   16. `[optional]` Token Authorization Rules account
        //passed in below, if needed
    ];

    let metadata = assert_decode_metadata(nft_mint, &nft_metadata.to_account_info())?;
    if let Some(standard) = metadata.token_standard {
        if standard == TokenStandard::ProgrammableNonFungible {
            msg!("programmable standard triggered");
            //1. add to builder
            builder
                .owner_token_record(owner_token_record.key())
                .destination_token_record(dest_token_record.key());

            //2. add to accounts (if try to pass these for non-pNFT, will get owner errors, since they don't exist)
            account_infos.push(owner_token_record.to_account_info());
            account_infos.push(dest_token_record.to_account_info());
        }
    }

    //if auth rules passed in, validate & include it in CPI call
    if let Some(config) = metadata.programmable_config {
        match config {
            V1 { rule_set } => {
                if let Some(rule_set) = rule_set {
                    msg!("ruleset triggered");
                    //safe to unwrap here, it's expected
                    let rules_acc = rules_acc.unwrap();

                    //1. validate
                    require!(rule_set == *rules_acc.key, YardsaleError::BadRuleset);

                    //2. add to builder
                    builder.authorization_rules_program(*authorization_rules_program.key);
                    builder.authorization_rules(*rules_acc.key);

                    //3. add to accounts
                    account_infos.push(authorization_rules_program.to_account_info());
                    account_infos.push(rules_acc.to_account_info());
                }
            }
        }
    }

    let transfer_ix = builder
        .build(TransferArgs::V1 {
            amount: 1, //currently 1 only
            authorization_data: authorization_data
                .map(|authorization_data| AuthorizationData::try_from(authorization_data).unwrap()),
        })
        .unwrap()
        .instruction();

    if let Some(listing) = program_signer {
        let signer_seeds = &[
            listing.item.as_ref(),
            LISTINGS.as_bytes().as_ref(),
            listing.keychain.as_bytes().as_ref(),
            listing.domain.as_bytes().as_ref(),
            YARDSALE.as_bytes().as_ref(),
            &[listing.bump],
        ];
        invoke_signed(&transfer_ix, &account_infos, &[signer_seeds]).unwrap();

    } else {
        invoke(&transfer_ix, &account_infos)?;
    }
    // invoke(&transfer_ix, &account_infos)?;
    Ok(())
}

// transfer a pnft out of a pda
pub fn tranfer_pnft_from_pda<'info>(
    listing: &Box<Account<'info, Listing>>,
    listing_item_token: &Box<Account<'info, TokenAccount>>,
    buyer: &AccountInfo<'info>,
    buyer_item_token: &Box<Account<'info, TokenAccount>>,
    item: &Box<Account<'info, Mint>>,
    item_metadata: &AccountInfo<'info>,
    edition: &AccountInfo<'info>,
    buyer_token_record: &UncheckedAccount<'info>,
    listing_token_record: &UncheckedAccount<'info>,
    ruleset: &Option<UncheckedAccount<'info>>,
    authorization_rules_program: &UncheckedAccount<'info>,
    token_metadata_program: &UncheckedAccount<'info>,
    instructions: &UncheckedAccount<'info>,
    token_program: &Program<'info, Token>,
    associated_token_program: &Program<'info, AssociatedToken>,
    system_program: &Program<'info, System>,
) -> Result<()> {

    // /** this auth data isn't necessary for now
    let seeds = SeedsVec {
        seeds: vec![listing.item.key().as_ref().to_vec(),
                    LISTINGS.as_bytes().to_vec(),
                    listing.keychain.as_bytes().to_vec(),
                    listing.domain.as_bytes().to_vec(),
                    YARDSALE.as_bytes().to_vec(),
        ]
    };

    let mut payload = Payload::new();
    payload.insert(PayloadKey::SourceSeeds.to_string(), PayloadType::Seeds(seeds));

    let auth_data = Some(AuthorizationData {
        payload
    });

    let transfer_args = TransferArgs::V1 {
        amount: 1,
        authorization_data: auth_data
        // this could be None and still work
        // authorization_data: None,
    };

    let mut builder = TransferBuilder::new();
    builder
        .authority(listing.key())
        .token_owner(listing.key())
        .token(listing_item_token.key())
        .destination_owner(buyer.key())
        .destination(buyer_item_token.key())
        .mint(item.key())
        .metadata(item_metadata.key())
        .edition(edition.key())
        .owner_token_record(listing_token_record.key())
        .destination_token_record(buyer_token_record.key())
        .authorization_rules_program(authorization_rules_program.key())
        .payer(buyer.key());

    // ruleset is optional
    let ruleset_account_info;
    if let Some(ruleset) = ruleset {
        builder.authorization_rules(ruleset.key());
        ruleset_account_info = ruleset.to_account_info();
    } else {
        // per pnft guide, set the ruleset to the token metadata program if it's not provided/needed
        ruleset_account_info = token_metadata_program.to_account_info();
    }

    msg!("building transfer instruction");
    let build_result = builder.build(transfer_args);

    let instruction = match build_result {
        Ok(transfer) => {
            msg!("transfer instruction built");
            transfer.instruction()
        }
        Err(err) => {
            msg!("Error building transfer instruction: {:?}", err);
            return Err(YardsaleError::TransferBuilderFailed.into());
        }
    };

    // these SHOULD be the transfer instructions but the ordering is based on the
    // rooster program, and DOESN"T match the order of the transfer instructions in the tm program
    let account_infos = [
        listing.to_account_info(),
        listing_item_token.to_account_info(),
        buyer.to_account_info(),
        buyer_item_token.to_account_info(),
        item.to_account_info(),
        item_metadata.to_account_info(),
        edition.to_account_info(),
        listing_token_record.to_account_info(),
        buyer_token_record.to_account_info(),
        ruleset_account_info,
        listing.to_account_info(),
        token_metadata_program.to_account_info(),
        system_program.to_account_info(),
        instructions.to_account_info(),
        token_program.to_account_info(),
        associated_token_program.to_account_info(),
        authorization_rules_program.to_account_info(),
    ];

    msg!("invoking transfer instruction");

    let seeds = &[
        listing.item.as_ref(),
        LISTINGS.as_bytes().as_ref(),
        listing.keychain.as_bytes().as_ref(),
        listing.domain.as_bytes().as_ref(),
        YARDSALE.as_bytes().as_ref(),
        &[listing.bump],
    ];
    let signer = &[&seeds[..]];

    invoke_signed(&instruction, &account_infos, signer).unwrap();

    Ok(())

}

// close an account owned by the given listing (usually a token account)
pub fn close_listing_owned_account<'info>(
    listing: &Box<Account<'info, Listing>>,
    account_to_close: AccountInfo<'info>,
    lamports_collector: AccountInfo<'info>,
    token_prog: &Program<'info, Token>,
) -> Result<()> {

    let cpi_close_accounts = CloseAccount {
        account: account_to_close,
        destination: lamports_collector,
        authority: listing.to_account_info(),
    };
    let signer_seeds = &[
        listing.item.as_ref(),
        LISTINGS.as_bytes().as_ref(),
        listing.keychain.as_bytes().as_ref(),
        listing.domain.as_bytes().as_ref(),
        YARDSALE.as_bytes().as_ref(),
        &[listing.bump],
    ];
    let signer = &[&signer_seeds[..]];
    let cpi_ctx = CpiContext::new_with_signer(token_prog.to_account_info(),
                                              cpi_close_accounts, signer);
    token::close_account(cpi_ctx)?;

    Ok(())
}

// properly populate a newly created listing account
pub fn create_listing(listing: &mut Box<Account<Listing>>,
                      listing_bump: u8,
                      item: Pubkey,
                      listing_item_token: Pubkey,
                      domain: String,
                      keychain: String,
                      currency: Pubkey,
                      treasury: Pubkey,
                      proceeds: &Option<AccountInfo>,
                      proceeds_token: &Option<Account<TokenAccount>>,
                      item_type: ItemType,
                      price: u64) -> Result<()> {
    listing.price = price;
    listing.item = item;
    listing.item_token = listing_item_token;
    listing.domain = domain;
    listing.keychain = keychain;
    listing.currency = currency;
    listing.bump = listing_bump;
    listing.treasury = treasury;
    listing.item_type = item_type;

    if listing.currency == NATIVE_MINT {
        // then the sale token isn't needed, but a regular accountinfo should've been specified (wallet)
        require!(proceeds.is_some(), YardsaleError::ProceedsAccountNotSpecified);
        listing.proceeds = proceeds.as_ref().unwrap().key();
    } else {
        // then the sale token is needed, but an accountinfo shouldn't have been specified (wallet)
        require!(proceeds_token.is_some(), YardsaleError::ProceedsTokenAccountNotSpecified);
        listing.proceeds = proceeds_token.as_ref().unwrap().key();
    }
    Ok(())
}

// get all the accounts needed to call the bubblegum program w/transfer instruction
pub fn create_cnft_transfer_accounts(tree_authority: Pubkey,
                                     leaf_owner: Pubkey,
                                     new_leaf_owner: Pubkey,
                                     merkle_tree: Pubkey,
                                     log_wrapper_program: Pubkey,
                                     compression_program: Pubkey,
                                     system_program: Pubkey) -> Vec<AccountMeta> {
    let accounts:  Vec<solana_program::instruction::AccountMeta> = vec![
        AccountMeta::new_readonly(tree_authority, false),
        AccountMeta::new_readonly(leaf_owner, true),
        AccountMeta::new_readonly(leaf_owner, false),
        AccountMeta::new_readonly(new_leaf_owner, false),
        AccountMeta::new(merkle_tree, false),
        AccountMeta::new_readonly(log_wrapper_program, false),
        AccountMeta::new_readonly(compression_program, false),
        AccountMeta::new_readonly(system_program, false),
    ];
    return accounts;
}

pub fn create_cnft_transfer_data(
    root: [u8; 32],
    data_hash: [u8; 32],
    creator_hash: [u8; 32],
    nonce: u64,
    index: u32,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(
        8           // The length of transfer_discriminator,
            + root.len()
            + data_hash.len()
            + creator_hash.len()
            + 8 // The length of the nonce
            + 8, // The length of the index
    );
    data.extend(TRANSFER_DISCRIMINATOR);
    data.extend(root);
    data.extend(data_hash);
    data.extend(creator_hash);
    data.extend(nonce.to_le_bytes());
    data.extend(index.to_le_bytes());
    return data;
}
