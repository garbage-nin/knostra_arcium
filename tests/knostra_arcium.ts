import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import { KnostraArcium } from "../target/types/knostra_arcium";
import { randomBytes } from "crypto";
import {
  awaitComputationFinalization,
  getArciumEnv,
  getCompDefAccOffset,
  getArciumAccountBaseSeed,
  getArciumProgAddress,
  uploadCircuit,
  buildFinalizeCompDefTx,
  RescueCipher,
  deserializeLE,
  getMXEPublicKey,
  getMXEAccAddress,
  getMempoolAccAddress,
  getCompDefAccAddress,
  getExecutingPoolAccAddress,
  getComputationAccAddress,
  x25519,
} from "@arcium-hq/client";
import * as fs from "fs";
import * as os from "os";
import { expect } from "chai";

describe("KnostraArcium", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.KnostraArcium as Program<KnostraArcium>;
  const provider = anchor.getProvider();

  type Event = anchor.IdlEvents<(typeof program)["idl"]>;
  const awaitEvent = async <E extends keyof Event>(
    eventName: E
  ): Promise<Event[E]> => {
    let listenerId: number;
    const event = await new Promise<Event[E]>((res) => {
      listenerId = program.addEventListener(eventName, (event) => {
        res(event);
      });
    });
    await program.removeEventListener(listenerId);

    return event;
  };

  const arciumEnv = getArciumEnv();

  const confirm = async (signature: string): Promise<string> => {
    const block = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction({
      signature,
      ...block,
    });
    return signature;
  };

  const log = async (signature: string): Promise<string> => {
    console.log(
      `Your transaction signature: https://explorer.solana.com/transaction/${signature}?cluster=custom&customUrl=http%3A%2F%2Flocalhost%3A8899`
    );
    return signature;
  };

  it("Is initialized!", async () => {
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

    console.log("Initializing add together computation definition");
    const initATSig = await initAddTogetherCompDef(
      program,
      owner,
      false,
      false
    );
    console.log(
      "Add together computation definition initialized with signature",
      initATSig
    );

    const mxePublicKey = await getMXEPublicKeyWithRetry(
      provider as anchor.AnchorProvider,
      program.programId
    );

    console.log("MXE x25519 pubkey is", mxePublicKey);

    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);

    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);

    const val1 = BigInt(1);
    const val2 = BigInt(2);
    const plaintext = [val1, val2];

    const nonce = randomBytes(16);
    const ciphertext = cipher.encrypt(plaintext, nonce);

    const sumEventPromise = awaitEvent("sumEvent");
    const computationOffset = new anchor.BN(randomBytes(8), "hex");

    const queueSig = await program.methods
      .addTogether(
        computationOffset,
        Array.from(ciphertext[0]),
        Array.from(ciphertext[1]),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(nonce).toString())
      )
      .accountsPartial({
        computationAccount: getComputationAccAddress(
          program.programId,
          computationOffset
        ),
        clusterAccount: arciumEnv.arciumClusterPubkey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(program.programId),
        executingPool: getExecutingPoolAccAddress(program.programId),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("add_together")).readUInt32LE()
        ),
      })
      .rpc({ skipPreflight: true, commitment: "confirmed" });
    console.log("Queue sig is ", queueSig);

    const finalizeSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      computationOffset,
      program.programId,
      "confirmed"
    );
    console.log("Finalize sig is ", finalizeSig);

    const sumEvent: any = await sumEventPromise;
    const decrypted = cipher.decrypt([sumEvent.sum], sumEvent.nonce)[0];
    expect(decrypted).to.equal(val1 + val2);
  });

  const marketCreator = Keypair.generate();
  const bettorYes = Keypair.generate();
  const bettorNo = Keypair.generate();

  it("Airdrop", async () => {
    await Promise.all([
      await provider.connection
        .requestAirdrop(marketCreator.publicKey, LAMPORTS_PER_SOL * 10)
        .then(confirm),
      await provider.connection
        .requestAirdrop(bettorYes.publicKey, LAMPORTS_PER_SOL * 10)
        .then(confirm),
      await provider.connection
        .requestAirdrop(bettorNo.publicKey, LAMPORTS_PER_SOL * 10)
        .then(confirm),
    ]);
  });

  const seed = new anchor.BN(1);
  const seedBuffer = seed.toArrayLike(Buffer, "le", 8);
  const seed2 = new anchor.BN(2);
  const seedBuffer2 = seed2.toArrayLike(Buffer, "le", 8);
  console.log("Program ID:", program.programId.toBase58());

  // Derive Market PDA
  const [marketAccount, marketBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("market"), marketCreator.publicKey.toBuffer(), seedBuffer],
    program.programId
  );

  // Derive Treasury PDA
  const [treasuryAccount, treasuryBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("treasury"), marketAccount.toBuffer()],
    program.programId
  );

  // Derive Treasury Vault PDA
  const [treasuryVault, treasuryVaultBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("treasury_vault"), marketAccount.toBuffer()],
    program.programId
  );

  // Derive Market Account 2 PDA
  const [marketAccount2, marketBump2] = PublicKey.findProgramAddressSync(
    [Buffer.from("market"), marketCreator.publicKey.toBuffer(), seedBuffer2],
    program.programId
  );

  // Derive Treasury Account 2 PDA
  const [treasuryAccount2, treasuryBump2] = PublicKey.findProgramAddressSync(
    [Buffer.from("treasury"), marketAccount2.toBuffer()],
    program.programId
  );

  // Derive Treasury Vault 2 PDA
  const [treasuryVault2, treasuryVaultBump2] = PublicKey.findProgramAddressSync(
    [Buffer.from("treasury_vault"), marketAccount2.toBuffer()],
    program.programId
  );

  it("Create market", async () => {
    console.log("Market Account:", marketAccount.toBase58());
    console.log("Market Bump:", marketBump);
    console.log("Treasury Account:", treasuryAccount.toBase58());
    console.log("Treasury Bump:", treasuryBump);

    console.log("Market Creator:", marketCreator.publicKey.toBase58());
    console.log(
      "Market Creator Balance:",
      await provider.connection.getBalance(marketCreator.publicKey)
    );
    console.log("Bettor YES:", bettorYes.publicKey.toBase58());
    console.log(
      "Bettor YES Balance:",
      await provider.connection.getBalance(bettorYes.publicKey)
    );
    console.log("Bettor NO:", bettorNo.publicKey.toBase58());
    console.log(
      "Bettor NO Balance:",
      await provider.connection.getBalance(bettorNo.publicKey)
    );

    console.log("Treasury Vault:", treasuryVault.toBase58());
    const createMarketParams = {
      name: "BTC market",
      description: "Will BTC >= 120k today?",
      token: "BTC",
      marketStart: new anchor.BN(Math.floor(Date.now() / 1000)),
      marketEnd: new anchor.BN(Math.floor(Date.now() / 1000) + 3600), // +1 hour
      relationalValue: ">=",
      targetValue: new anchor.BN(120000),
      requiredBetAmount: new anchor.BN(1_000_000_000), // 1 SOL
      maxPlayerCount: new anchor.BN(1),
    };

    // Call the create instruction
    const tx = await program.methods
      .create(seed, createMarketParams, marketBump, treasuryBump)
      .accountsPartial({
        marketAccount: marketAccount,
        treasuryAccount: treasuryAccount,
        systemProgram: SystemProgram.programId,
        treasuryVault: treasuryVault,
        user: marketCreator.publicKey,
      }) // Cast to any to bypass TS account type issues
      .signers([marketCreator])
      .rpc()
      .then(confirm)
      .then(log);

    console.log(
      "Market creator after balance:",
      await provider.connection.getBalance(marketCreator.publicKey)
    );
  });

  // Derive Bettor YES PDA
  const [bettorYesAccount, bettorYesBump] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("bet"),
      marketAccount.toBuffer(),
      bettorYes.publicKey.toBuffer(),
    ],
    program.programId
  );

  it("Place bet not required amount", async () => {
    console.log("Bettor YES Account:", bettorYesAccount.toBase58());
    const tx = await program.methods
      .bet(new anchor.BN(100_000_000), true, bettorYesBump) // 1 SOL
      .accountsPartial({
        marketAccount: marketAccount,
        treasuryAccount: treasuryAccount,
        betAccount: bettorYesAccount,
        user: bettorYes.publicKey,
        systemProgram: SystemProgram.programId,
      }) // Cast to any to bypass TS account type issues
      .signers([bettorYes])
      .rpc()
      .then(confirm)
      .then(log);

    console.log(
      "Bettor yes after balance:",
      await provider.connection.getBalance(bettorYes.publicKey)
    );

    console.log(
      "Treasury after balance:",
      await provider.connection.getBalance(treasuryAccount)
    );
  });

  it("Place bet YES", async () => {
    console.log("Bettor YES Account:", bettorYesAccount.toBase58());
    const tx = await program.methods
      .bet(new anchor.BN(1_000_000_000), true, bettorYesBump) // 1 SOL
      .accountsPartial({
        marketAccount: marketAccount,
        treasuryAccount: treasuryAccount,
        betAccount: bettorYesAccount,
        user: bettorYes.publicKey,
        treasuryVault: treasuryVault,
        systemProgram: SystemProgram.programId,
      }) // Cast to any to bypass TS account type issues
      .signers([bettorYes])
      .rpc()
      .then(confirm)
      .then(log);

    console.log(
      "Bettor yes after balance:",
      await provider.connection.getBalance(bettorYes.publicKey)
    );

    console.log(
      "Treasury after balance:",
      await provider.connection.getBalance(treasuryAccount)
    );
  });

  // Derive Bettor NO PDA
  const [bettorNoAccount, bettorNoBump] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("bet"),
      marketAccount.toBuffer(),
      bettorNo.publicKey.toBuffer(),
    ],
    program.programId
  );

  it("Place bet No", async () => {
    console.log("Bettor No Account:", bettorNoAccount.toBase58());
    const tx = await program.methods
      .bet(new anchor.BN(1_000_000_000), false, bettorNoBump) // 1 SOL
      .accountsPartial({
        marketAccount: marketAccount,
        treasuryAccount: treasuryAccount,
        betAccount: bettorNoAccount,
        user: bettorNo.publicKey,
        treasuryVault: treasuryVault,
        systemProgram: SystemProgram.programId,
      }) // Cast to any to bypass TS account type issues
      .signers([bettorNo])
      .rpc()
      .then(confirm)
      .then(log);

    console.log(
      "Bettor no after balance:",
      await provider.connection.getBalance(bettorNo.publicKey)
    );

    console.log(
      "Treasury after balance:",
      await provider.connection.getBalance(treasuryAccount)
    );
  });

  it("Trying to claim market before resolved", async () => {
    const tx = await program.methods
      .claim()
      .accountsPartial({
        marketAccount: marketAccount,
        treasuryAccount: treasuryAccount,
        betAccount: bettorYesAccount,
        user: bettorYes.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([bettorYes])
      .rpc()
      .then(confirm)
      .then(log);

    console.log(
      "Bettor yes after balance:",
      await provider.connection.getBalance(bettorYes.publicKey)
    );

    console.log(
      "Treasury after balance:",
      await provider.connection.getBalance(treasuryAccount)
    );
  });

  it("Resolve market - Yes", async () => {
    const [resolverAuthority, resolverBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("resolver_authority")],
      program.programId
    );

    const tx = await program.methods
      .resolve(new anchor.BN(130000))
      .accountsPartial({
        marketAccount: marketAccount,
        treasuryAccount: treasuryAccount,
        resolverAuthority, // must be included
        owner: marketCreator.publicKey, // checked only, not signer
        systemProgram: SystemProgram.programId,
      })
      .rpc({})
      .then(confirm)
      .then(log);
  });

  it("Claim winnings No", async () => {
    const tx = await program.methods
      .claim()
      .accountsPartial({
        marketAccount: marketAccount,
        treasuryAccount: treasuryAccount,
        betAccount: bettorNoAccount,
        treasuryVault: treasuryVault,
        user: bettorNo.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([bettorNo])
      .rpc()
      .then(confirm)
      .then(log);

    console.log(
      "Bettor no after balance:",
      await provider.connection.getBalance(bettorNo.publicKey)
    );

    console.log(
      "Treasury after balance:",
      await provider.connection.getBalance(treasuryAccount)
    );
  });

  it("Claim winnings yes", async () => {
    const tx = await program.methods
      .claim()
      .accountsPartial({
        marketAccount: marketAccount,
        treasuryAccount: treasuryAccount,
        betAccount: bettorYesAccount,
        treasuryVault: treasuryVault,
        user: bettorYes.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([bettorYes])
      .rpc()
      .then(confirm)
      .then(log);

    console.log(
      "Bettor yes after balance:",
      await provider.connection.getBalance(bettorYes.publicKey)
    );

    console.log(
      "Treasury after balance:",
      await provider.connection.getBalance(treasuryAccount)
    );
  });

  it("Try claiming twice", async () => {
    const tx = await program.methods
      .claim()
      .accountsPartial({
        marketAccount: marketAccount,
        treasuryAccount: treasuryAccount,
        betAccount: bettorYesAccount,
        treasuryVault: treasuryVault,
        user: bettorYes.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([bettorYes])
      .rpc()
      .then(confirm)
      .then(log);

    console.log(
      "Bettor yes after balance:",
      await provider.connection.getBalance(bettorYes.publicKey)
    );

    console.log(
      "Treasury after balance:",
      await provider.connection.getBalance(treasuryAccount)
    );
  });

  it("Claim fees", async () => {
    const tx = await program.methods
      .claimFees()
      .accountsPartial({
        marketAccount: marketAccount,
        treasuryAccount: treasuryAccount,
        treasuryVault: treasuryVault,
        user: marketCreator.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([marketCreator])
      .rpc()
      .then(confirm)
      .then(log);

    console.log(
      "Market creator after balance:",
      await provider.connection.getBalance(marketCreator.publicKey)
    );
  });

  // MARKET CANCELATION FLOW
  it("Create market to cancel", async () => {
    const createMarketParams = {
      name: "BTC market",
      description: "Will BTC >= 120k today?",
      token: "BTC",
      marketStart: new anchor.BN(Math.floor(Date.now() / 1000)),
      marketEnd: new anchor.BN(Math.floor(Date.now() / 1000) + 3600), // +1 hour
      relationalValue: ">=",
      targetValue: new anchor.BN(120000),
      requiredBetAmount: new anchor.BN(1_000_000_000), // 1 SOL
      maxPlayerCount: new anchor.BN(1),
    };

    // Call the create instruction
    const tx = await program.methods
      .create(seed2, createMarketParams, marketBump2, treasuryBump2)
      .accountsPartial({
        marketAccount: marketAccount2,
        treasuryAccount: treasuryAccount2,
        systemProgram: SystemProgram.programId,
        treasuryVault: treasuryVault2,
        user: marketCreator.publicKey,
      }) // Cast to any to bypass TS account type issues
      .signers([marketCreator])
      .rpc()
      .then(confirm)
      .then(log);

    console.log(
      "Market creator after balance:",
      await provider.connection.getBalance(marketCreator.publicKey)
    );
  });

  const [bettorYesAccount2, bettorYesBump2] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("bet"),
      marketAccount2.toBuffer(),
      bettorYes.publicKey.toBuffer(),
    ],
    program.programId
  );

  it("Place bet YES to market account 2", async () => {
    console.log("Bettor YES Account:", bettorYesAccount2.toBase58());
    const tx = await program.methods
      .bet(new anchor.BN(1_000_000_000), true, bettorYesBump2) // 1 SOL
      .accountsPartial({
        marketAccount: marketAccount2,
        treasuryAccount: treasuryAccount2,
        betAccount: bettorYesAccount2,
        user: bettorYes.publicKey,
        treasuryVault: treasuryVault2,
        systemProgram: SystemProgram.programId,
      }) // Cast to any to bypass TS account type issues
      .signers([bettorYes])
      .rpc()
      .then(confirm)
      .then(log);

    console.log(
      "Bettor yes after balance:",
      await provider.connection.getBalance(bettorYes.publicKey)
    );
  });

  it("Cancel market", async () => {
    const tx = await program.methods
      .cancel()
      .accountsPartial({
        marketAccount: marketAccount2,
        treasuryAccount: treasuryAccount2,
        creator: marketCreator.publicKey, // Just for validation, not signer
        systemProgram: SystemProgram.programId,
      }) // Cast to any to bypass TS account type issues
      .rpc()
      .then(confirm)
      .then(log);
  });

  it("Claim winnings yes after cancel", async () => {
    const tx = await program.methods
      .claim()
      .accountsPartial({
        marketAccount: marketAccount2,
        treasuryAccount: treasuryAccount2,
        betAccount: bettorYesAccount2,
        treasuryVault: treasuryVault2,
        user: bettorYes.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([bettorYes])
      .rpc()
      .then(confirm)
      .then(log);

    console.log(
      "Bettor yes after balance:",
      await provider.connection.getBalance(bettorYes.publicKey)
    );
  });

  it("Initialize init game computation definition", async () => {
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

    console.log("Initializing init game computation definition");
    const initIGSig = await initInitGameCompDef(program, owner, false, false);
    console.log(
      "Init game computation definition initialized with signature",
      initIGSig
    );

    const mxePublicKey = await getMXEPublicKeyWithRetry(
      provider as anchor.AnchorProvider,
      program.programId
    );

    console.log("MXE x25519 pubkey is", mxePublicKey);

    console.log("initialize a new game");
    const gameId = 1;
    const nonce = randomBytes(16);

    const initComputationOffset = new anchor.BN(randomBytes(8), "hex");

    const initGameTx = await program.methods
      .initGame(
        initComputationOffset,
        new anchor.BN(gameId),
        new anchor.BN(deserializeLE(nonce).toString())
      )
      .accountsPartial({
        computationAccount: getComputationAccAddress(
          program.programId,
          initComputationOffset
        ),
        payer: owner.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(program.programId),
        executingPool: getExecutingPoolAccAddress(program.programId),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("init_game")).readUInt32LE()
        ),
        clusterAccount: arciumEnv.arciumClusterPubkey,
      })
      .signers([owner])
      .rpc({ skipPreflight: true, commitment: "confirmed" })
      .then(confirm)
      .then(log);

    console.log("Init game queue tx sig is ", initGameTx);

    // Wait for initGame computation finalization
    const initGameFinalizeSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      initComputationOffset,
      program.programId,
      "confirmed"
    );
    console.log("Init game finalize signature:", initGameFinalizeSig);
  });

  async function initAddTogetherCompDef(
    program: Program<KnostraArcium>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean,
    offchainSource: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("add_together");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgAddress()
    )[0];

    console.log("Comp def pda is ", compDefPDA);

    const sig = await program.methods
      .initAddTogetherCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
      })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init add together computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/add_together.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "add_together",
        program.programId,
        rawCircuit,
        true
      );
    } else if (!offchainSource) {
      const finalizeTx = await buildFinalizeCompDefTx(
        provider as anchor.AnchorProvider,
        Buffer.from(offset).readUInt32LE(),
        program.programId
      );

      const latestBlockhash = await provider.connection.getLatestBlockhash();
      finalizeTx.recentBlockhash = latestBlockhash.blockhash;
      finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;

      finalizeTx.sign(owner);

      await provider.sendAndConfirm(finalizeTx);
    }
    return sig;
  }
});

async function getMXEPublicKeyWithRetry(
  provider: anchor.AnchorProvider,
  programId: PublicKey,
  maxRetries: number = 10,
  retryDelayMs: number = 500
): Promise<Uint8Array> {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      const mxePublicKey = await getMXEPublicKey(provider, programId);
      if (mxePublicKey) {
        return mxePublicKey;
      }
    } catch (error) {
      console.log(`Attempt ${attempt} failed to fetch MXE public key:`, error);
    }

    if (attempt < maxRetries) {
      console.log(
        `Retrying in ${retryDelayMs}ms... (attempt ${attempt}/${maxRetries})`
      );
      await new Promise((resolve) => setTimeout(resolve, retryDelayMs));
    }
  }

  throw new Error(
    `Failed to fetch MXE public key after ${maxRetries} attempts`
  );
}

// Separate functions for each computation definition type
async function initInitGameCompDef(
  program: Program<KnostraArcium>,
  owner: anchor.web3.Keypair,
  uploadRawCircuit: boolean,
  offchainSource: boolean
): Promise<string> {
  const baseSeedCompDefAcc = getArciumAccountBaseSeed(
    "ComputationDefinitionAccount"
  );
  const offset = getCompDefAccOffset("init_game");

  const compDefPDA = PublicKey.findProgramAddressSync(
    [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
    getArciumProgAddress()
  )[0];

  console.log(`Comp def PDA for init_game:`, compDefPDA.toBase58());

  const sig = await program.methods
    .initInitGameCompDef()
    .accounts({
      compDefAccount: compDefPDA,
      payer: owner.publicKey,
      mxeAccount: getMXEAccAddress(program.programId),
    })
    .signers([owner])
    .rpc({
      commitment: "confirmed",
    });

  console.log(`Init init_game computation definition transaction`, sig);

  if (uploadRawCircuit) {
    const rawCircuit = fs.readFileSync(`build/init_game.arcis`);
    await uploadCircuit(
      program.provider as anchor.AnchorProvider,
      "init_game",
      program.programId,
      rawCircuit,
      true
    );
  } else if (!offchainSource) {
    const finalizeTx = await buildFinalizeCompDefTx(
      program.provider as anchor.AnchorProvider,
      Buffer.from(offset).readUInt32LE(),
      program.programId
    );

    const latestBlockhash =
      await program.provider.connection.getLatestBlockhash();
    finalizeTx.recentBlockhash = latestBlockhash.blockhash;
    finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;

    finalizeTx.sign(owner);
    await program.provider.sendAndConfirm(finalizeTx);
  }
  return sig;
}
function readKpJson(path: string): anchor.web3.Keypair {
  const file = fs.readFileSync(path);
  return anchor.web3.Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(file.toString()))
  );
}
