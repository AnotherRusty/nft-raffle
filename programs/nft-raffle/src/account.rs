use anchor_lang::prelude::*;
use std::clone::Clone;

use crate::constants::*;

#[account]
#[derive(Default)]
pub struct GlobalPool {
    pub super_user: Pubkey              //  32
}

impl GlobalPool {
    pub const LEN: usize = 8 + 32;
}

#[account(zero_copy)]
pub struct CollectionPool {                         //  6410    = 8 + 2 + 6400
    pub cnt_collection: u16,                        //  2
    pub collections: [Pubkey; MAX_COLLECTIONS]      //  32 * 200
}

#[account(zero_copy)]
pub struct Raffle {                                 //  64167   = 8 + 32 * 4 + 2 * 3 + 8 * 3 + 1 + 32 * 2000
    pub creator: Pubkey,                            //  32
    pub nft_mint: Pubkey,                           //  32
    pub ticket_cnt: u16,                            //  2
    pub ticket_max: u16,                            //  2
    pub timestamp_start: i64,                       //  8
    pub timestamp_end: i64,                         //  8
    pub ticket_price: u64,                          //  8
    pub winner: Pubkey,                             //  32
    pub claimed: u8,                                //  1
    pub cnt_entrants: u16,                          //  2
    pub entrants: [Pubkey; MAX_ENTRANTS]            //  32 * 2000
}

impl Default for CollectionPool {
    #[inline]
    fn default() -> CollectionPool {
        CollectionPool {
            cnt_collection: 0,
            collections: [Pubkey::default(); MAX_COLLECTIONS]
        }
    }
}

impl Raffle {
    pub fn append(&mut self, buyer: Pubkey, amount: u16) {
        for i in self.ticket_cnt..(self.ticket_cnt + amount) {
            self.entrants[i as usize] = buyer;
        }
        self.entrants[self.cnt_entrants as usize] = buyer;
        self.cnt_entrants += amount;
    }
}

impl CollectionPool {
    pub fn append(&mut self, collection: Pubkey) {
        if self.cnt_collection != 0 {
            for i in 0..(self.cnt_collection - 1) {
                if self.collections[i as usize] == collection {
                    return;
                }
            }
        }
        self.collections[self.cnt_collection as usize] = collection;
        self.cnt_collection += 1;
    }
}
