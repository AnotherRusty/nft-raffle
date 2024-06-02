import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { NftRaffle } from "../target/types/nft_raffle";

describe("nft-raffle", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.NftRaffle as Program<NftRaffle>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });
});
