use anchor_lang::prelude::*;

use anchor_spl::{
    token::{self, Token, TokenAccount, Transfer},
};
use metaplex_token_metadata::state::Metadata;


pub mod utils;
pub mod error;
pub mod account;
pub mod constants;

use utils::*;
use error::*;
use account::*;
use constants::*;

declare_id!("5n5DrZywM3VDeWyBpvZmXRK18PeWiZAu6xqe5ffaASQL");

#[program]
pub mod nft_raffle {
    use super::*;

    /**
     * @dev Initialize the project
     */
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let global_authority = &mut ctx.accounts.global_authority;
        global_authority.super_user = ctx.accounts.su.key();
        let _ = ctx.accounts.collection_pool.load_init();
        Ok(())
    }

    /**
     * @dev Add collections to this platform
     */
    pub fn add_collection(ctx: Context<AddCollection>) -> Result<()> {
        let mut collection = ctx.accounts.collection_pool.load_mut()?;
        collection.append(ctx.accounts.collection_id.key());
        Ok(())
    }

    /**
     * @dev Create new raffle
     * @Context has user, global_authority, raffle account
     *  and user's nft ata, global_authority's nft ata
     *  and nft_mint, mint_metadata
     * @param   ticket_price: ticket's price by sol
     * @param   timestamp_end: the end time of this raffle
     * @param   ticket_max: the maximum number of tickets
     */
    pub fn create_raffle(ctx: Context<CreateRaffle>,
        ticket_price: u64,
        timestamp_end: i64,
        ticket_max: u16
    ) -> Result<()> {
        let mint_metadata = &mut &ctx.accounts.mint_metadata;

        msg!("Metadata Account: {:?}", ctx.accounts.mint_metadata.key());
        let (metadata, _) = Pubkey::find_program_address(
            &[
                metaplex_token_metadata::state::PREFIX.as_bytes(),
                metaplex_token_metadata::id().as_ref(),
                ctx.accounts.nft_mint.key().as_ref(),
            ],
            &metaplex_token_metadata::id(),
        );
        require!(metadata == mint_metadata.key(), RaffleError::InvalidMetadata);
        
        let collection_pool = ctx.accounts.collection_pool.load_mut()?;

        // verify metadata is legit
        let nft_metadata = Metadata::from_account_info(mint_metadata)?;
        if let Some(creators) = nft_metadata.data.creators {
            let mut valid: u8 = 0;
            for creator in creators {
                for i in 0..collection_pool.cnt_collection  {
                    if creator.address == collection_pool.collections[i as usize] && creator.verified == true {
                        valid = 1;
                        break;
                    }
                }
                if valid == 1 {
                    break;
                }
            }
            if valid != 1 {
                return Err(error!(RaffleError::InvalidCollection));
            }
        } else {
            return Err(error!(RaffleError::MetadataCreatorParseError));
        };

        let mut raffle = ctx.accounts.raffle.load_init()?;
        let timestamp = Clock::get()?.unix_timestamp;

        require!(ticket_max <= 2000, RaffleError::MaxEntrantsTooLarge);
        require!(timestamp + DAY <= timestamp_end, RaffleError::EndTimeTooEarly);

        //  Transfer NFT to the ATA
        let src_token_account_info = &mut &ctx.accounts.ata_user;
        let dest_token_account_info = &mut &ctx.accounts.ata_program;
        let token_program = &mut &ctx.accounts.token_program;

        let cpi_accounts = Transfer {
            from: src_token_account_info.to_account_info().clone(),
            to: dest_token_account_info.to_account_info().clone(),
            authority: ctx.accounts.raffle_creator.to_account_info().clone(),
        };
        token::transfer(
            CpiContext::new(token_program.clone().to_account_info(), cpi_accounts),
            1,
        )?;

        raffle.creator = ctx.accounts.raffle_creator.key();
        raffle.nft_mint = ctx.accounts.nft_mint.key();
        raffle.ticket_price = ticket_price;
        raffle.timestamp_start = timestamp;
        raffle.timestamp_end = timestamp_end;
        raffle.ticket_max = ticket_max;
        Ok(())
    }

    /**
     * @dev Buy tickets
     * @Context has buyer and raffle account
     *  global authority, creator, atas
     * @param   ticket_demand: the amount of tickets user want to buy
     */
    pub fn buy_tickets(ctx: Context<BuyTickets>,
        ticket_demand: u16
    ) -> Result<()> {
        let timestamp = Clock::get()?.unix_timestamp;
        let mut raffle = ctx.accounts.raffle.load_mut()?;
        let accounts: &&mut BuyTickets = &ctx.accounts;
        let buyer = &accounts.buyer;

        require!(timestamp < raffle.timestamp_end, RaffleError::RaffleEnded);
        require!(raffle.ticket_max >= raffle.ticket_cnt + ticket_demand, RaffleError::NotEnoughTicketLeft);

        let total_price: u64 = raffle.ticket_price * (ticket_demand as u64);
        require!(total_price < buyer.to_account_info().lamports(), RaffleError::NotEnoughSOL);

        raffle.append(buyer.key(), ticket_demand);

        //  Transfer SOL from buyer to raffle_creator
        let creator_amount = total_price * (100 - COMMISSION_FEE as u64) / 100;
        sol_transfer_user(
            accounts.buyer.to_account_info(),
            accounts.raffle_creator.to_account_info(),
            accounts.system_program.to_account_info(),
            creator_amount,
        )?;

        //  Transfer COMMISSION_FEE SOL from buyer to program wallet
        let fee_amount = total_price * COMMISSION_FEE as u64 / 100;
        sol_transfer_user(
            accounts.buyer.to_account_info(),
            accounts.program_wallet.to_account_info(),
            accounts.system_program.to_account_info(),
            fee_amount,
        )?;

        Ok(())
    }

    /**
     * @dev Generate a random number and reveal winner
     * @Context has raffle account
     */
    pub fn reveal_winner(ctx: Context<RevealWinner>) -> Result<()> {
        let mut raffle = ctx.accounts.raffle.load_mut()?;
        let timestamp = Clock::get()?.unix_timestamp;

        require!(timestamp >= raffle.timestamp_end, RaffleError::RaffleNotEnded);
        require!(raffle.cnt_entrants > 0, RaffleError::RaffleHasNoEntrants);

        // Generate a random number less than raffle.ticket_cnt
        let (player_address, _bump) = Pubkey::find_program_address(
            &[
                b"random-seed",
                timestamp.to_string().as_bytes(),
            ],
            &nft_raffle::ID,
        );
        let char_vec: Vec<char> = player_address.to_string().chars().collect();
        let mut mul = 1;
        for i in 0..7 {
            mul *= u64::from(char_vec[i as usize]);
        }
        mul += u64::from(char_vec[7]);
        let rand_idx = mul % raffle.ticket_cnt as u64;

        msg!("Rand: {:?}", rand_idx);
        raffle.winner = raffle.entrants[rand_idx as usize];
        raffle.claimed = 1;
        Ok(())
    }

    /**
     * @dev Claim reward function
     * @Context has claimer and global_authority
     *  raffle and ata of claimer and global_authority
     * @param   global_bump: global_authority's bump
     */
    pub fn claim_reward(ctx: Context<ClaimReward>, global_bump: u8) -> Result<()> {
        let mut raffle = ctx.accounts.raffle.load_mut()?;
        let timestamp = Clock::get()?.unix_timestamp;

        require!(raffle.winner == ctx.accounts.ata_claimer.key(), RaffleError::NotWinner);
        require!(timestamp >= raffle.timestamp_end, RaffleError::RaffleNotEnded);
        require!(raffle.claimed == 1, RaffleError::NoRewards);

        //  Transfer NFT to winner's wallet
        let src_token_account = &mut &ctx.accounts.ata_program;
        let dest_token_account = &mut &ctx.accounts.ata_claimer;
        let token_program = &mut &ctx.accounts.token_program;
        let seeds = &[GLOBAL_AUTHORITY_SEED.as_bytes(), &[global_bump]];
        let signer = &[&seeds[..]];
        let cpi_accounts = Transfer {
            from: src_token_account.to_account_info().clone(),
            to: dest_token_account.to_account_info().clone(),
            authority: ctx.accounts.global_authority.to_account_info(),
        };
        token::transfer(
            CpiContext::new_with_signer(
                token_program.clone().to_account_info(),
                cpi_accounts,
                signer,
            ),
            1,
        )?;
        raffle.claimed = 2;

        Ok(())
    }

    /**
     * @dev Withdraw NFT when there's no entrants
     * @Context has withdrawer and global_authority
     *  raffle and ata of withdrawer and global_authority
     * @param   global_bump: global_authority's bump
     */
    pub fn withdraw_nft(ctx: Context<WithdrawNft>, global_bump: u8) -> Result<()> {
        let timestamp = Clock::get()?.unix_timestamp;
        let mut raffle = ctx.accounts.raffle.load_mut()?;

        require!(raffle.creator == ctx.accounts.ata_withdrawer.key(), RaffleError::NotCreator);
        require!(timestamp >= raffle.timestamp_end, RaffleError::RaffleNotEnded);
        require!(raffle.cnt_entrants == 0, RaffleError::HasEntrants);

        
        // Transfer NFT to the creator's wallet
        // after the raffle ends or 
        // creator wants to withdraw nft because no tickets sold
        let src_token_account = &mut &ctx.accounts.ata_program;
        let dest_token_account = &mut &ctx.accounts.ata_withdrawer;
        let token_program = &mut &ctx.accounts.token_program;
        let seeds = &[GLOBAL_AUTHORITY_SEED.as_bytes(), &[global_bump]];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: src_token_account.to_account_info().clone(),
            to: dest_token_account.to_account_info().clone(),
            authority: ctx.accounts.global_authority.to_account_info(),
        };
        token::transfer(
            CpiContext::new_with_signer(
                token_program.clone().to_account_info(),
                cpi_accounts,
                signer,
            ),
            1,
        )?;
        raffle.claimed = 3;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub su: Signer<'info>,

    #[account(
        init,
        seeds = [GLOBAL_AUTHORITY_SEED.as_ref()],
        bump,
        payer = su,
        space = GlobalPool::LEN
    )]
    pub global_authority: Account<'info, GlobalPool>,

    #[account(zero)]
    pub collection_pool: AccountLoader<'info, CollectionPool>,
    
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>
}

#[derive(Accounts)]
pub struct AddCollection<'info> {
    #[account(mut)]
    pub su: Signer<'info>,

    #[account(mut)]
    pub collection_pool: AccountLoader<'info, CollectionPool>,
    
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub collection_id: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CreateRaffle<'info> {
    pub raffle_creator: Signer<'info>,
    
    pub global_authority: Account<'info, GlobalPool>,
    
    #[account(zero)]
    pub raffle: AccountLoader<'info, Raffle>,
    
    #[account(zero)]
    pub collection_pool: AccountLoader<'info, CollectionPool>,

    #[account(
        mut,
        constraint = ata_user.mint == *nft_mint.to_account_info().key,
        constraint = ata_user.owner == *raffle_creator.key,
    )]
    pub ata_user: Account<'info, TokenAccount>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        mut,
        constraint = ata_program.mint == *nft_mint.to_account_info().key,
        constraint = ata_program.owner == *raffle_creator.to_account_info().key
    )]
    pub ata_program: Account<'info, TokenAccount>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    pub nft_mint: AccountInfo<'info>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        constraint = nft_mint.owner == &metaplex_token_metadata::ID
    )]
    pub mint_metadata: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        constraint = token_metadata_program.key == &metaplex_token_metadata::ID
    )]
    pub token_metadata_program: AccountInfo<'info>
}

#[derive(Accounts)]
pub struct BuyTickets<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(mut)]
    pub raffle: AccountLoader<'info, Raffle>,

    #[account(mut)]
    pub global_authority: Account<'info, GlobalPool>,
    
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub raffle_creator: AccountInfo<'info>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        mut,
        constraint = program_wallet.key() == PROGRAM_WALLET_ADDRESS.parse::<Pubkey>().unwrap()
    )]
    pub program_wallet: AccountInfo<'info>,

    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct RevealWinner<'info> {
    #[account(mut)]
    pub raffle: AccountLoader<'info, Raffle>
}

#[derive(Accounts)]
pub struct ClaimReward<'info> {
    #[account(mut)]
    pub claimer: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_AUTHORITY_SEED.as_ref()],
        bump
    )]
    pub global_authority: Account<'info, GlobalPool>,

    #[account(mut)]
    pub raffle: AccountLoader<'info, Raffle>,

    #[account(
        mut,
        constraint = ata_claimer.mint == *nft_mint.to_account_info().key,
        constraint = ata_claimer.owner == *claimer.key,
    )]
    pub ata_claimer: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = ata_program.mint == *nft_mint.to_account_info().key,
        constraint = ata_program.owner == *global_authority.to_account_info().key,
    )]
    pub ata_program: Account<'info, TokenAccount>,
    
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub nft_mint: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}


#[derive(Accounts)]
pub struct WithdrawNft<'info> {
    #[account(mut)]
    pub withdrawer: Signer<'info>,

    #[account(
        mut,
        seeds = [GLOBAL_AUTHORITY_SEED.as_ref()],
        bump
    )]
    pub global_authority: Account<'info, GlobalPool>,
    
    #[account(mut)]
    pub raffle: AccountLoader<'info, Raffle>,
    
    #[account(
        mut,
        constraint = ata_withdrawer.mint == *nft_mint.to_account_info().key,
        constraint = ata_withdrawer.owner == *withdrawer.key,
    )]
    pub ata_withdrawer: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = ata_program.mint == *nft_mint.to_account_info().key,
        constraint = ata_program.owner == *global_authority.to_account_info().key,
    )]
    pub ata_program: Account<'info, TokenAccount>,
    
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub nft_mint: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}
