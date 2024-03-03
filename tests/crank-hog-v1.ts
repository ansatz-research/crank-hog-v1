import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { CrankHogV1 } from "../target/types/crank_hog_v1";

describe("crank-hog-v1", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.CrankHogV1 as Program<CrankHogV1>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });
});
