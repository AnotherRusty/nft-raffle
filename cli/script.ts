import { Program, web3 } from '@project-serum/anchor';
import * as anchor from '@project-serum/anchor';
import {
    Keypair,
    PublicKey,
    SystemProgram,
    SYSVAR_RENT_PUBKEY,
    Transaction,
    TransactionInstruction,
    sendAndConfirmTransaction
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, AccountLayout, MintLayout, ASSOCIATED_TOKEN_PROGRAM_ID } from "@solana/spl-token";

import fs from 'fs';
import { CollectionPool, GlobalPool, Raffle } from './types';
import { publicKey } from '@project-serum/anchor/dist/cjs/utils';
import { NftRaffle } from '../target/types/nft_raffle';

const METAPLEX = new PublicKey('metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s');
const GLOBAL_AUTHORITY_SEED = "global-authority";
const TREASURY_WALLET = new PublicKey('kVGZXZHFsZKRmR9DPQHaVQppvuD3LB4H8QzHxsrquTG');
const PROGRAM_ID = "5n5DrZywM3VDeWyBpvZmXRK18PeWiZAu6xqe5ffaASQL";

const RAFFLE_SIZE = 64167;
const COLLECTION_SIZE = 6410;
const DECIMALS = 1000000000;

anchor.setProvider(anchor.AnchorProvider.local(web3.clusterApiUrl('devnet')));
const solConnection = anchor.getProvider().connection;
const payer = anchor.AnchorProvider.local().wallet;
console.log(payer.publicKey.toBase58());

const idl = JSON.parse(fs.readFileSync(__dirname + "/../target/idl/nft_raffle.json", "utf8"));
let program: Program = null;
const programId = new anchor.web3.PublicKey(PROGRAM_ID);
program = new anchor.Program(idl, programId);
console.log('ProgramId: ', program.programId.toBase58());

const main = async () => {
    const [globalAuthority, bump] = PublicKey.findProgramAddressSync(
        [Buffer.from(GLOBAL_AUTHORITY_SEED)],
        program.programId
    );
    console.log('GlobalAuthority: ', globalAuthority.toBase58());

//    await initProject();
//    await addCollectionToPool(new PublicKey('kVGZXZHFsZKRmR9DPQHaVQppvuD3LB4H8QzHxsrquTG'));
}

/**
 * @dev Initialize the project - global pool and collection pool
 */
export const initProject = async () => {
    const [globalAuthority, bump] = await PublicKey.findProgramAddress(
        [Buffer.from(GLOBAL_AUTHORITY_SEED)],
        program.programId
    );
    let su = payer.publicKey;
    
    let collectionPool = await PublicKey.createWithSeed(
        su,
        "collection-pool",
        program.programId,
    );
    console.log(su.toBase58());

    let ix = SystemProgram.createAccountWithSeed({
        fromPubkey: su,
        basePubkey: su,
        seed: "collection-pool",
        newAccountPubkey: collectionPool,
        lamports: await solConnection.getMinimumBalanceForRentExemption(COLLECTION_SIZE),
        space: COLLECTION_SIZE,
        programId: program.programId,
    });
    
    const tx = await program.methods.initialize()
        .accounts({
            su,
            globalAuthority,
            collectionPool,
            systemProgram: SystemProgram.programId,
            rent: SYSVAR_RENT_PUBKEY,
        })
        .preInstructions([ix])
        .rpc();

    console.log("txHash =", tx);
}

/**
 * @dev Add collection to the Program collection list
 * @param creator The caller of this function
 * @param collectionId The collection verified creator address to add
 */
export const addCollectionToPool = async (
    collectionId: PublicKey
) => {
    let su = payer.publicKey;
    let collectionPool = await PublicKey.createWithSeed(
        su,
        "collection-pool",
        program.programId,
    );
    console.log('collection pool: ', collectionPool.toBase58());
    const tx = await program.methods.addCollection()
        .accounts({
            su,
            collectionPool,
            collectionId
        })
        .rpc();

    console.log("txHash =", tx);
}

/**
 * @dev Create a raffle
 * @param   su Super user
 * @param   creator The raffle creator's address
 * @param   nftMint The nft_mint address
 * @param   ticketPriceSol The ticket price by SOL 
 * @param   endTimestamp The raffle end timestamp
 * @param   maxEntrants The maximum entrants of this raffle
 */
export const createRaffle = async (
    su: PublicKey,
    creator: PublicKey,
    nftMint: PublicKey,
    ticketPriceSol: number,
    endTimestamp: number,
    maxEntrants: number
) => {

    const [globalAuthority, bump] = PublicKey.findProgramAddressSync(
        [Buffer.from(GLOBAL_AUTHORITY_SEED)],
        program.programId
    );
    let collection = await PublicKey.createWithSeed(
        su,
        "collection-pool",
        program.programId,
    );

    let ataUser = await getAssociatedTokenAccount(creator, nftMint);

    let ix0 = await getATokenAccountsNeedCreate(
        solConnection,
        creator,
        globalAuthority,
        [nftMint]
    );
    console.log("Dest NFT Account = ", ix0.destinationAccounts[0].toBase58());

    let seed = nftMint.toString() + Date.now();

    let raffle = await PublicKey.createWithSeed(
        creator,
        seed,
        program.programId,
    );

    let ix = SystemProgram.createAccountWithSeed({
        fromPubkey: creator,
        basePubkey: creator,
        seed,
        newAccountPubkey: raffle,
        lamports: await solConnection.getMinimumBalanceForRentExemption(RAFFLE_SIZE),
        space: RAFFLE_SIZE,
        programId: program.programId,
    });

    const mintMetadata = await getMetadataAddr(nftMint);

    const tx = await program.methods.createRaffle(
        bump,
        new anchor.BN(ticketPriceSol * DECIMALS),
        new anchor.BN(endTimestamp),
        new anchor.BN(maxEntrants))
        .accounts({
            raffleCreator: creator,
            globalAuthority,
            raffle,
            collection,
            ataUser,
            ataProgram: ix0.destinationAccounts[0],
            nftMint,
            mintMetadata,
            tokenProgram: TOKEN_PROGRAM_ID,
            tokenMetadataProgram: METAPLEX
        })
        .preInstructions([ix, ...ix0.instructions])
        .rpc();

    console.log("txHash =", tx);

}

/**
 * @dev BuyTicket function
 * @param userAddress The use's address
 * @param nft_mint The nft_mint address
 * @param amount The amount of ticket to buy
 */
export const buyTicket = async (
    userAddress: PublicKey,
    nft_mint: PublicKey,
    amount: number,
    creator: PublicKey,
    raffleKey: PublicKey
) => {
    const [globalAuthority, bump] = await PublicKey.findProgramAddress(
        [Buffer.from(GLOBAL_AUTHORITY_SEED)],
        program.programId
    );
    const tx = await program.rpc.buyTickets(
        bump,
        new anchor.BN(amount),
        {
            accounts: {
                buyer: userAddress,
                raffle: raffleKey,
                globalAuthority,
                creator,
                treasuryWallet: TREASURY_WALLET,
                systemProgram: SystemProgram.programId,
            },
            instructions: [],
            signers: [],
        });
    await solConnection.confirmTransaction(tx, "confirmed");

    console.log("txHash =", tx);

}

/**
 * @dev RevealWinner function
 * @param userAddress The user's address to call this function
 * @param raffleKey The raffleKey address
 */
export const revealWinner = async (
    userAddress: PublicKey,
    nft_mint: PublicKey,
    raffleKey: PublicKey
) => {
    console.log(userAddress.toBase58());
    console.log(raffleKey.toBase58());
    const tx = await program.rpc.revealWinner(
        {
            accounts: {
                buyer: userAddress,
                raffle: raffleKey,
            },
            instructions: [],
            signers: [],
        });
    await solConnection.confirmTransaction(tx, "confirmed");

    console.log("txHash =", tx);
}

/**
 * @dev ClaimReward function
 * @param userAddress The winner's address
 * @param nft_mint The nft_mint address
 */
export const claimReward = async (
    userAddress: PublicKey,
    nft_mint: PublicKey,
    raffleKey: PublicKey
) => {
    const [globalAuthority, bump] = await PublicKey.findProgramAddress(
        [Buffer.from(GLOBAL_AUTHORITY_SEED)],
        program.programId
    );

    const srcNftTokenAccount = await getAssociatedTokenAccount(globalAuthority, nft_mint);

    let ix0 = await getATokenAccountsNeedCreate(
        solConnection,
        userAddress,
        userAddress,
        [nft_mint]
    );
    console.log("Claimer's NFT Account: ", ix0.destinationAccounts[0]);

    let tx = await program.rpc.claimReward(
        bump,
        {
            accounts: {
                claimer: userAddress,
                globalAuthority,
                raffle: raffleKey,
                claimerNftTokenAccount: ix0.destinationAccounts[0],
                srcNftTokenAccount,
                nftMintAddress: nft_mint,
                tokenProgram: TOKEN_PROGRAM_ID,
            },
            instructions: [
                ...ix0.instructions
            ],
            signers: [],
        });
    await solConnection.confirmTransaction(tx, "confirmed");

    console.log("txHash =", tx);

}

/**
 * @dev WithdrawNFT function
 * @param userAddress The creator's address
 * @param nft_mint The nft_mint address
 */
export const withdrawNft = async (
    userAddress: PublicKey,
    nft_mint: PublicKey,
    raffleKey: PublicKey,
) => {
    const [globalAuthority, bump] = await PublicKey.findProgramAddress(
        [Buffer.from(GLOBAL_AUTHORITY_SEED)],
        program.programId
    );

    const srcNftTokenAccount = await getAssociatedTokenAccount(globalAuthority, nft_mint);

    let ix0 = await getATokenAccountsNeedCreate(
        solConnection,
        userAddress,
        userAddress,
        [nft_mint]
    );
    console.log("Creator's NFT Account: ", ix0.destinationAccounts[0].toBase58());
    console.log(raffleKey.toBase58());

    let tx = await program.rpc.withdrawNft(
        bump, {
        accounts: {
            claimer: userAddress,
            globalAuthority,
            raffle: raffleKey,
            claimerNftTokenAccount: ix0.destinationAccounts[0],
            srcNftTokenAccount,
            nftMintAddress: nft_mint,
            tokenProgram: TOKEN_PROGRAM_ID,
        },
        instructions: [
            ...ix0.instructions
        ],
        signers: [],
    });
    await solConnection.confirmTransaction(tx, "confirmed");

    console.log("txHash =", tx);

}

const getAssociatedTokenAccount = async (ownerPubkey: PublicKey, mintPk: PublicKey): Promise<PublicKey> => {
    let associatedTokenAccountPubkey = (await PublicKey.findProgramAddress(
        [
            ownerPubkey.toBuffer(),
            TOKEN_PROGRAM_ID.toBuffer(),
            mintPk.toBuffer(), // mint address
        ],
        ASSOCIATED_TOKEN_PROGRAM_ID
    ))[0];
    return associatedTokenAccountPubkey;
}

export const getATokenAccountsNeedCreate = async (
    connection: anchor.web3.Connection,
    walletAddress: anchor.web3.PublicKey,
    owner: anchor.web3.PublicKey,
    nfts: anchor.web3.PublicKey[],
) => {
    let instructions = [], destinationAccounts = [];
    for (const mint of nfts) {
        const destinationPubkey = await getAssociatedTokenAccount(owner, mint);
        let response = await connection.getAccountInfo(destinationPubkey);
        if (!response) {
            const createATAIx = createAssociatedTokenAccountInstruction(
                destinationPubkey,
                walletAddress,
                owner,
                mint,
            );
            instructions.push(createATAIx);
        }
        destinationAccounts.push(destinationPubkey);
        if (walletAddress != owner) {
            const userAccount = await getAssociatedTokenAccount(walletAddress, mint);
            response = await connection.getAccountInfo(userAccount);
            if (!response) {
                const createATAIx = createAssociatedTokenAccountInstruction(
                    userAccount,
                    walletAddress,
                    walletAddress,
                    mint,
                );
                instructions.push(createATAIx);
            }
        }
    }
    return {
        instructions,
        destinationAccounts,
    };
}

export const createAssociatedTokenAccountInstruction = (
    associatedTokenAddress: anchor.web3.PublicKey,
    payer: anchor.web3.PublicKey,
    walletAddress: anchor.web3.PublicKey,
    splTokenMintAddress: anchor.web3.PublicKey
) => {
    const keys = [
        { pubkey: payer, isSigner: true, isWritable: true },
        { pubkey: associatedTokenAddress, isSigner: false, isWritable: true },
        { pubkey: walletAddress, isSigner: false, isWritable: false },
        { pubkey: splTokenMintAddress, isSigner: false, isWritable: false },
        {
            pubkey: anchor.web3.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        {
            pubkey: anchor.web3.SYSVAR_RENT_PUBKEY,
            isSigner: false,
            isWritable: false,
        },
    ];
    return new anchor.web3.TransactionInstruction({
        keys,
        programId: ASSOCIATED_TOKEN_PROGRAM_ID,
        data: Buffer.from([]),
    });
}
export const getMetadataAddr = async (mint: PublicKey): Promise<PublicKey> => {
    return (
        await PublicKey.findProgramAddress([Buffer.from('metadata'), METAPLEX.toBuffer(), mint.toBuffer()], METAPLEX)
    )[0];
};

main()
