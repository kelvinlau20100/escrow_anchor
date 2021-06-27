const anchor = require("@project-serum/anchor");
const serumCmn = require("@project-serum/common");
const TokenInstructions = require("@project-serum/serum").TokenInstructions;
const assert = require("assert");

describe('escrow_kl', () => {

  // Configure the client to use the local cluster.
  const provider = anchor.Provider.local();
  anchor.setProvider(provider);

  const takerAmount = 100;
  const initializerAmount = 200;

  const program = anchor.workspace.EscrowKl
  const escrowAcc = anchor.web3.Keypair.generate();

  it ("Test Setup", async() => {
    const [mintX, _tX] = await serumCmn.createMintAndVault(
      provider,
      new anchor.BN(initializerAmount)
    );
    initializerTokenAccX = _tX;

    const [mintY, _tY] = await serumCmn.createMintAndVault(
      provider,
      new anchor.BN(takerAmount)
    );
    takerTokenAccY = _tY;

    initializerTokenAccY = await serumCmn.createTokenAccount(
      provider,
      mintY,
      provider.wallet.publicKey
    );

    takerTokenAccX = await serumCmn.createTokenAccount(
      provider,
      mintX,
      provider.wallet.publicKey
    );

    // Get the PDA that is assigned authority to token account.
    const [_pda, _nonce] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(anchor.utils.bytes.utf8.encode("escrow"))],
      program.programId
    );
    pda = _pda;
    
    // Make sure the initializer Accounts created properly
    _initializerTokenAccX = await serumCmn.getTokenAccount(provider, initializerTokenAccX);
    _initializerTokenAccY = await serumCmn.getTokenAccount(provider, initializerTokenAccY);
    _takerTokenAccX = await serumCmn.getTokenAccount(provider, takerTokenAccX);
    _takerTokenAccY = await serumCmn.getTokenAccount(provider, takerTokenAccY);
    assert.ok(_initializerTokenAccX.amount.eq(new anchor.BN(initializerAmount)));
    assert.ok(_initializerTokenAccY.amount.eq(new anchor.BN(0)));
    assert.ok(_takerTokenAccX.amount.eq(new anchor.BN(0)));
    assert.ok(_takerTokenAccY.amount.eq(new anchor.BN(takerAmount)));
    assert.ok(_initializerTokenAccX.owner.equals(provider.wallet.publicKey));

  });
  
  it('Test Init Escrow', async () => {
    await program.rpc.initEscrow(
      new anchor.BN(takerAmount),
      {
        accounts: {
          initializerAcc: provider.wallet.publicKey,
          tempTokenAcc: initializerTokenAccX,
          tokenToRxAcc: initializerTokenAccY,
          escrowAcc: escrowAcc.publicKey,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID
        },
        instructions: [
          await program.account.escrowAcc.createInstruction(escrowAcc),
        ],
        signers: [escrowAcc],
      }
    );

    _initializerTokenAccX = await serumCmn.getTokenAccount(provider, initializerTokenAccX);
    _escrowAcc = await program.account.escrowAcc.fetch(escrowAcc.publicKey);
    // Verify the PDA owns the initializer token account X
    assert.ok(_initializerTokenAccX.owner.equals(pda));
    // Verify all the escrowAcc fields are correct
    assert.ok(_escrowAcc.isInitialized);
    assert.ok(_escrowAcc.initializerPubkey.equals(provider.wallet.publicKey));
    assert.ok(_escrowAcc.tempTokenAccPubkey.equals(initializerTokenAccX));
    assert.ok(_escrowAcc.initializerTokenToReceiveAccPubkey.equals(initializerTokenAccY));
    assert.ok(_escrowAcc.expectedAmount.toNumber() == takerAmount);

  


  });    

  it('Test Exchange', async() => {

    try {
      await program.rpc.exchange(
        new anchor.BN(initializerAmount),
        {
          accounts: {
            takerAcc: provider.wallet.publicKey,
            takerTokenAccY: takerTokenAccY,
            takerTokenAccX: takerTokenAccX,
            initializerTokenAccX: initializerTokenAccX,
            initializerMainAcc: provider.wallet.publicKey,
            initializerTokenAccY: initializerTokenAccY,
            escrowAcc: escrowAcc.publicKey,
            tokenProgram: TokenInstructions.TOKEN_PROGRAM_ID,
            pdaAcc: pda,
          },
        }
      )
    } catch (err) {
      throw new Error(err.toString());
    }
    
    _initializerTokenAccY = await serumCmn.getTokenAccount(provider, initializerTokenAccY);
    _initializerTokenAccX = await serumCmn.getTokenAccount(provider, initializerTokenAccX);
    _takerTokenAccY = await serumCmn.getTokenAccount(provider, takerTokenAccY);
    _takerTokenAccX = await serumCmn.getTokenAccount(provider, takerTokenAccX);
    
    // Verify the closing balances make sense
    assert.ok(_initializerTokenAccX.amount.eq(new anchor.BN(0)));
    assert.ok(_initializerTokenAccY.amount.eq(new anchor.BN(takerAmount)));
    assert.ok(_takerTokenAccX.amount.eq(new anchor.BN(initializerAmount)));
    assert.ok(_takerTokenAccY.amount.eq(new anchor.BN(0)));
    // Verify the provider owns the initialiser_token_account_x again
    assert.ok(_initializerTokenAccX.owner.equals(provider.wallet.publicKey));


  });
});
