// Import necessary modules from the anchor_lang library.
use anchor_lang::prelude::*;
// Import necessary modules from the anchor_spl library for token operations.
use anchor_spl::token::{self, CloseAccount, SetAuthority, TokenAccount, Transfer};
// Import the AuthorityType enum from the spl_token library.
use spl_token::instruction::AuthorityType;

// Declare the program ID.
declare_id!("2gcFaJwn6AcRqgZdKSmTPjHJAXpwKu3EH67DFHThzpbP");

// Define the anchor_auction module.
#[program]
pub mod anchor_auction {
    // Import the Add trait to use the add method for i64.
    use std::ops::Add;
    // Import everything from the parent module.
    use super::*;

    // Define a constant byte slice for the escrow PDA seed.
    const ESCROW_PDA_SEED: &[u8] = b"escrow";

    // Define the exhibit function to exhibit an item for auction.
    pub fn exhibit(
        ctx: Context<Exhibit>, // Context for the Exhibit struct.
        initial_price: u64,    // Initial price for the auction.
        auction_duration_sec: u64, // Duration of the auction in seconds.
    ) -> Result<()> {
        // Set the exhibitor's public key in the escrow account.
        ctx.accounts.escrow_account.exhibitor_pubkey = ctx.accounts.exhibitor.key();
        // Set the exhibitor's fungible token (FT) receiving account public key in the escrow account.
        ctx.accounts.escrow_account.exhibitor_ft_receiving_pubkey = ctx.accounts.exhibitor_ft_receiving_account.key();
        // Set the exhibitor's non-fungible token (NFT) temporary account public key in the escrow account.
        ctx.accounts.escrow_account.exhibiting_nft_temp_pubkey = ctx.accounts.exhibitor_nft_temp_account.key();
        // Initially, set the highest bidder's public key to the exhibitor's public key in the escrow account.
        ctx.accounts.escrow_account.highest_bidder_pubkey = ctx.accounts.exhibitor.key();
        // Set the highest bidder's FT temporary account public key to the exhibitor's FT receiving account public key.
        ctx.accounts.escrow_account.highest_bidder_ft_temp_pubkey = ctx.accounts.exhibitor_ft_receiving_account.key();
        // Set the highest bidder's FT returning account public key to the exhibitor's FT receiving account public key.
        ctx.accounts.escrow_account.highest_bidder_ft_returning_pubkey = ctx.accounts.exhibitor_ft_receiving_account.key();
        // Set the initial price for the auction in the escrow account.
        ctx.accounts.escrow_account.price = initial_price;
        // Calculate and set the auction end time in the escrow account.
        ctx.accounts.escrow_account.end_at = ctx.accounts.clock.unix_timestamp.add(auction_duration_sec as i64);

        // Find the Program Derived Address (PDA) for the escrow account.
        let (pda, _bump_seed) = Pubkey::find_program_address(&[ESCROW_PDA_SEED], ctx.program_id);
        // Set the authority of the NFT to the PDA.
        token::set_authority(
            ctx.accounts.to_set_authority_context(),
            AuthorityType::AccountOwner,
            Some(pda)
        )?;

        // Transfer the NFT to the PDA-controlled escrow account.
        token::transfer(
            ctx.accounts.to_transfer_to_pda_context(),
            1
        )?;

        // Return an Ok result.
        Ok(())
    }

    // Define the cancel function to cancel an ongoing auction.
    pub fn cancel(ctx: Context<Cancel> ) -> Result<()> {
        // Find the PDA for the escrow account.
        let (_, bump_seed) = Pubkey::find_program_address(&[ESCROW_PDA_SEED], ctx.program_id);
        // Create the seeds for the signer.
        let signers_seeds: &[&[&[u8]]] = &[&[&ESCROW_PDA_SEED[..], &[bump_seed]]];

        // Transfer the NFT back to the exhibitor.
        token::transfer(
            ctx.accounts
                .to_transfer_to_exhibitor_context()
                .with_signer(signers_seeds),
            ctx.accounts.exhibitor_nft_temp_account.amount
        )?;

        // Close the PDA-controlled escrow account.
        token::close_account(
            ctx.accounts
                .to_close_context()
                .with_signer(signers_seeds)
        )?;

        // Return an Ok result.
        Ok(())
    }

    // Define the bid function for users to place bids.
    pub fn bid(ctx: Context<Bid>, price: u64) -> Result<()> {
        // Find the PDA for the escrow account.
        let (pda, bump_seed) = Pubkey::find_program_address(&[ESCROW_PDA_SEED], ctx.program_id);
        // Create the seeds for the signer.
        let signers_seeds: &[&[&[u8]]] = &[&[&ESCROW_PDA_SEED[..], &[bump_seed]]];

        // Check if the current highest bidder is not the exhibitor.
        if ctx.accounts.escrow_account.highest_bidder_pubkey != ctx.accounts.escrow_account.exhibitor_pubkey {
            // Transfer the current highest bid amount back to the previous highest bidder.
            token::transfer(
                ctx.accounts
                    .to_transfer_to_previous_bidder_context()
                    .with_signer(signers_seeds),
                ctx.accounts.escrow_account.price
            )?;

            // Close the previous highest bidder's temporary FT account.
            token::close_account(
                ctx.accounts
                    .to_close_context()
                    .with_signer(signers_seeds)
            )?;
        }

        // Set the authority of the bidder's FT account to the PDA.
        token::set_authority(
            ctx.accounts.to_set_authority_context(),
            AuthorityType::AccountOwner,
            Some(pda)
        )?;
        // Transfer the bid amount from the bidder's FT account to the PDA-controlled escrow account.
        token::transfer(
            ctx.accounts.to_transfer_to_pda_context(),
            price,
        )?;

        // Update the escrow account with the new highest bid amount.
        ctx.accounts.escrow_account.price = price;
        // Update the escrow account with the new highest bidder's public key.
        ctx.accounts.escrow_account.highest_bidder_pubkey = ctx.accounts.bidder.key();
        // Update the escrow account with the new highest bidder's FT temporary account public key.
        ctx.accounts.escrow_account.highest_bidder_ft_temp_pubkey = ctx.accounts.bidder_ft_temp_account.key();
        // Update the escrow account with the new highest bidder's FT returning account public key.
        ctx.accounts.escrow_account.highest_bidder_ft_returning_pubkey = ctx.accounts.bidder_ft_account.key();

        // Return an Ok result.
        Ok(())
    }

    // Define the close function to close the auction and distribute the assets.
    pub fn close(ctx: Context<Close>) -> Result<()> {
        // Find the PDA for the escrow account.
        let (_, bump_seed) = Pubkey::find_program_address(&[ESCROW_PDA_SEED], ctx.program_id);
        // Create the seeds for the signer.
        let signers_seeds: &[&[&[u8]]] = &[&[&ESCROW_PDA_SEED[..], &[bump_seed]]];

        // Transfer the NFT from the escrow account to the highest bidder.
        token::transfer(
            ctx.accounts
                .to_transfer_to_highest_bidder_context()
                .with_signer(signers_seeds),
            ctx.accounts.exhibitor_nft_temp_account.amount,
        )?;

        // Transfer the highest bid amount from the escrow account to the exhibitor.
        token::transfer(
            ctx.accounts
                .to_transfer_to_exhibitor_context()
                .with_signer(signers_seeds),
            ctx.accounts.highest_bidder_ft_temp_account.amount,
        )?;

        // Close the highest bidder's temporary FT account.
        token::close_account(
            ctx.accounts.to_close_ft_context()
                .with_signer(signers_seeds),
        )?;

        // Close the exhibitor's temporary NFT account.
        token::close_account(
            ctx.accounts.to_close_nft_context()
                .with_signer(signers_seeds),
        )?;

        // Return an Ok result.
        Ok(())
    }
}

// Define the Exhibit struct with associated accounts and instructions.
#[derive(Accounts)]
#[instruction(initial_price: u64, auction_duration_sec: u64)]
pub struct Exhibit<'info> {
    // The exhibitor's account, which must be a signer.
    /// CHECK: This is not dangerous, does not need check (ask rich or dean)
    #[account(signer)]
    pub exhibitor: AccountInfo<'info>,
    // The exhibitor's NFT account, which must have an amount of 1.
    #[account(
        mut,
        constraint = exhibitor_nft_token_account.amount == 1
    )]
    pub exhibitor_nft_token_account: Account<'info, TokenAccount>,
    // The exhibitor's temporary NFT account.
    pub exhibitor_nft_temp_account: Account<'info, TokenAccount>,
    // The exhibitor's FT receiving account.
    pub exhibitor_ft_receiving_account:Account<'info, TokenAccount>,
    // The escrow account, which must have a balance of zero.
    #[account(zero)]
    pub escrow_account: Box<Account<'info, Auction>>,
    // The system clock account for getting the current UNIX timestamp.
    pub clock: Sysvar<'info, Clock>,
    // The SPL token program account.
    /// CHECK: This is not dangerous, does not need check (ask rich or dean)
    pub token_program: AccountInfo<'info>,
}

// Define the Cancel struct with associated accounts.
#[derive(Accounts)]
pub struct Cancel<'info> {
    // The exhibitor's account, which must be a signer.
    /// CHECK: This is not dangerous, does not need check (ask rich or dean)
    #[account(signer)]
    pub exhibitor: AccountInfo<'info>,
    // The exhibitor's NFT account.
    #[account(mut)]
    pub exhibitor_nft_token_account: Account<'info, TokenAccount>,
    // The exhibitor's temporary NFT account.
    #[account(mut)]
    pub exhibitor_nft_temp_account: Account<'info, TokenAccount>,
    // The escrow account with various constraints.
    #[account(
        mut,
        constraint = escrow_account.exhibitor_pubkey == exhibitor.key(),
        constraint = escrow_account.highest_bidder_pubkey == exhibitor.key(),
        constraint = escrow_account.exhibiting_nft_temp_pubkey == exhibitor_nft_temp_account.key(),
        close = exhibitor
    )]
    pub escrow_account: Box<Account<'info, Auction>>,
    // The PDA account.
    /// CHECK: This is not dangerous, does not need check (ask rich or dean)
    pub pda: AccountInfo<'info>,
    // The SPL token program account.
    /// CHECK: This is not dangerous, does not need check (ask rich or dean)
    pub token_program: AccountInfo<'info>,
}

// Define the Bid struct with associated accounts and instructions.
#[derive(Accounts)]
#[instruction(price: u64)]
pub struct Bid<'info> {
    // The bidder's account, which must be a signer.
    /// CHECK: This is not dangerous, does not need check (ask rich or dean)
    #[account(signer)]
    pub bidder: AccountInfo<'info>,
    // The bidder's temporary FT account.
    #[account(mut)]
    pub bidder_ft_temp_account: Account<'info, TokenAccount>,
    // The bidder's FT account, which must have an amount greater than or equal to the bid price.
    #[account(
        mut,
        constraint = bidder_ft_account.amount >= price
    )]
    pub bidder_ft_account: Account<'info, TokenAccount>,
    // The highest bidder's account, which must not be the same as the current bidder.
    #[account(
        mut,
        constraint = highest_bidder.key() != bidder.key()
    )]
    /// CHECK: This is not dangerous, does not need check (ask rich or dean)
    pub highest_bidder: AccountInfo<'info>,
    // The highest bidder's temporary FT account.
    #[account(mut)]
    pub highest_bidder_ft_temp_account: Account<'info, TokenAccount>,
    // The highest bidder's FT returning account.
    #[account(mut)]
    pub highest_bidder_ft_returning_account: Account<'info, TokenAccount>,
    // The escrow account with various constraints.
    #[account(
        mut,
        constraint = escrow_account.highest_bidder_pubkey == highest_bidder.key(),
        constraint = escrow_account.highest_bidder_ft_temp_pubkey == highest_bidder_ft_temp_account.key(),
        constraint = escrow_account.highest_bidder_ft_returning_pubkey == highest_bidder_ft_returning_account.key(),
        constraint = escrow_account.price < price,
        constraint = escrow_account.end_at > clock.unix_timestamp
    )]
    pub escrow_account: Box<Account<'info, Auction>>,
    // The system clock account for getting the current UNIX timestamp.
    pub clock: Sysvar<'info, Clock>,
    // The PDA account.
    /// CHECK: This is not dangerous, does not need check (ask rich or dean)
    pub pda: AccountInfo<'info>,
    // The SPL token program account.
    /// CHECK: This is not dangerous, does not need check (ask rich or dean)
    pub token_program: AccountInfo<'info>,
}

// Define the Close struct with associated accounts.
#[derive(Accounts)]
pub struct Close<'info> {
    // The winning bidder's account, which must be a signer.
    /// CHECK: This is not dangerous, does not need check (ask rich or dean)
    #[account(signer)]
    pub winning_bidder: AccountInfo<'info>,
    // The exhibitor's account.
    /// CHECK: This is not dangerous, does not need check (ask rich or dean)
    #[account(mut)]
    pub exhibitor: AccountInfo<'info>,
    // The exhibitor's temporary NFT account.
    #[account(mut)]
    pub exhibitor_nft_temp_account: Account<'info, TokenAccount>,
    // The exhibitor's FT receiving account.
    #[account(mut)]
    pub exhibitor_ft_receiving_account: Account<'info, TokenAccount>,
    // The highest bidder's temporary FT account.
    #[account(mut)]
    pub highest_bidder_ft_temp_account: Account<'info, TokenAccount>,
    // The highest bidder's NFT receiving account.
    #[account(mut)]
    pub highest_bidder_nft_receiving_account: Account<'info, TokenAccount>,
    // The escrow account with various constraints.
    #[account(
        mut,
        constraint = escrow_account.exhibitor_pubkey == exhibitor.key(),
        constraint = escrow_account.exhibiting_nft_temp_pubkey == exhibitor_nft_temp_account.key(),
        constraint = escrow_account.exhibitor_ft_receiving_pubkey == exhibitor_ft_receiving_account.key(),
        constraint = escrow_account.highest_bidder_pubkey == winning_bidder.key(),
        constraint = escrow_account.highest_bidder_ft_temp_pubkey == highest_bidder_ft_temp_account.key(),
        constraint = escrow_account.end_at <= clock.unix_timestamp,
        close = exhibitor
    )]
    pub escrow_account: Box<Account<'info, Auction>>,
    // The system clock account for getting the current UNIX timestamp.
    pub clock: Sysvar<'info, Clock>,
    // The PDA account.
    /// CHECK: This is not dangerous, does not need check (ask rich or dean)
    pub pda: AccountInfo<'info>,
    // The SPL token program account.
    /// CHECK: This is not dangerous, does not need check (ask rich or dean)
    pub token_program: AccountInfo<'info>,
}

// Implement the Exhibit struct.
impl<'info> Exhibit<'info> {
    // Define a function to create a context for transferring NFTs to the PDA.
    fn to_transfer_to_pda_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self
                .exhibitor_nft_token_account
                .to_account_info()
                .clone(),
            to: self.exhibitor_nft_temp_account.to_account_info().clone(),
            authority: self.exhibitor.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    // Define a function to create a context for setting the authority of the NFT to the PDA.
    fn to_set_authority_context(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.exhibitor_nft_temp_account.to_account_info().clone(),
            current_authority: self.exhibitor.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

// Implement the Cancel struct.
impl<'info> Cancel<'info> {
    // Define a function to create a context for transferring NFTs back to the exhibitor.
    fn to_transfer_to_exhibitor_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.exhibitor_nft_temp_account.to_account_info().clone(),
            to: self
                .exhibitor_nft_token_account
                .to_account_info()
                .clone(),
            authority: self.pda.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    // Define a function to create a context for closing the PDA-controlled escrow account.
    fn to_close_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.exhibitor_nft_temp_account.to_account_info().clone(),
            destination: self.exhibitor.clone(),
            authority: self.pda.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

// Implement the Bid struct.
impl<'info> Bid<'info> {
    // Define a function to create a context for setting the authority of the bidder's FT account to the PDA.
    fn to_set_authority_context(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.bidder_ft_temp_account.to_account_info().clone(),
            current_authority: self.bidder.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    // Define a function to create a context for closing the previous highest bidder's temporary FT account.
    fn to_close_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.highest_bidder_ft_temp_account.to_account_info().clone(),
            destination: self.highest_bidder.clone(),
            authority: self.pda.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    // Define a function to create a context for transferring the current highest bid amount back to the previous highest bidder.
    fn to_transfer_to_previous_bidder_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.highest_bidder_ft_temp_account.to_account_info().clone(),
            to: self
                .highest_bidder_ft_returning_account
                .to_account_info()
                .clone(),
            authority: self.pda.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    // Define a function to create a context for transferring the bid amount from the bidder's FT account to the PDA-controlled escrow account.
    fn to_transfer_to_pda_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.bidder_ft_account.to_account_info().clone(),
            to: self
                .bidder_ft_temp_account
                .to_account_info()
                .clone(),
            authority: self.bidder.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

// Implement the Close struct.
impl<'info> Close<'info> {
    // Define a function to create a context for transferring the NFT from the escrow account to the highest bidder.
    fn to_transfer_to_highest_bidder_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.exhibitor_nft_temp_account.to_account_info().clone(),
            to: self
                .highest_bidder_nft_receiving_account
                .to_account_info()
                .clone(),
            authority: self.pda.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    // Define a function to create a context for transferring the highest bid amount from the escrow account to the exhibitor.
    fn to_transfer_to_exhibitor_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.highest_bidder_ft_temp_account.to_account_info().clone(),
            to: self
                .exhibitor_ft_receiving_account
                .to_account_info()
                .clone(),
            authority: self.pda.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    // Define a function to create a context for closing the highest bidder's temporary FT account.
    fn to_close_ft_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.highest_bidder_ft_temp_account.to_account_info().clone(),
            destination: self.winning_bidder.clone(),
            authority: self.pda.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    // Define a function to create a context for closing the exhibitor's temporary NFT account.
    fn to_close_nft_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.exhibitor_nft_temp_account.to_account_info().clone(),
            destination: self.exhibitor.clone(),
            authority: self.pda.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

// Define the Auction struct to represent the auction state.
#[account]
pub struct Auction {
    // The exhibitor's public key.
    pub exhibitor_pubkey: Pubkey,
    // The exhibitor's FT receiving account public key.
    pub exhibitor_ft_receiving_pubkey: Pubkey,
    // The exhibitor's temporary NFT account public key.
    pub exhibiting_nft_temp_pubkey: Pubkey,
    // The highest bidder's public key.
    pub highest_bidder_pubkey: Pubkey,
    // The highest bidder's FT temporary account public key.
    pub highest_bidder_ft_temp_pubkey: Pubkey,
    // The highest bidder's FT returning account public key.
    pub highest_bidder_ft_returning_pubkey: Pubkey,
    // The current highest bid amount.
    pub price: u64,
    // The auction end time in UNIX timestamp.
    pub end_at: i64,
}
