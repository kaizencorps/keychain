use anchor_lang::context::CpiContext;
use anchor_lang::prelude::{Account, AccountInfo};
use crate::*;
use crate::common::listing::Listing;
use crate::common::constant::LISTING;
use anchor_spl::token::{self, CloseAccount, Mint, Token, TokenAccount, Transfer};


// transfers an item out of the listing's token account and potentially closes it
pub fn transfer_items<'a, 'b>(listing: &Box<Account<'a, Listing>>,
                              listing_item_token_ai: AccountInfo<'b>,
                              to_token_ai: AccountInfo<'b>,
                              amount: u64,
                              lamports_claimer_ai: AccountInfo<'a>,
                              close_token: bool,
                              token_program: AccountInfo<'a>) -> Result<()>
    where 'a: 'b, 'b: 'a {

    let seeds = &[
        LISTING.as_bytes().as_ref(),
        listing.seller_account.as_ref(),
        &listing.listing_index.to_le_bytes(),
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
    token::transfer(cpi_ctx, amount)?;

    msg!("Transferred {} items to buyer", amount);

    // now we can close the item listing's token account if specified
    if close_token {
        let cpi_close_accounts = CloseAccount {
            account: listing_item_token_ai.clone(),
            destination: lamports_claimer_ai.clone(),
            authority: listing.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(token_program.clone(),
                                                  cpi_close_accounts, signer);
        token::close_account(cpi_ctx)?;
    }

    Ok(())
}
