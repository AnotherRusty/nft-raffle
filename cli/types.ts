import * as anchor from '@project-serum/anchor';
import { PublicKey } from '@solana/web3.js';

export interface GlobalPool {
    super_user: PublicKey
}

export interface CollectionPool {
    cnt_collection: anchor.BN,
    collections: PublicKey[]
}

export interface Raffle {
    creator: PublicKey,
    nft_mint: PublicKey,
    ticket_cnt: anchor.BN,
    ticket_max: anchor.BN,
    timestamp_start: anchor.BN,
    timestamp_end: anchor.BN,
    ticket_price: anchor.BN,
    winner: PublicKey,
    claimed: anchor.BN,
    cnt_entrants: anchor.BN,
    entrants: PublicKey[]
}
