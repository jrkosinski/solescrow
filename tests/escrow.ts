import * as anchor from '@coral-xyz/anchor';
import { Program } from '@coral-xyz/anchor';
import { Escrow } from '../target/types/escrow';
import { PublicKey } from '@solana/web3.js';
import { expect } from 'chai';
import { EscrowStatus, EscrowTestUtils } from './utils';

describe('AsymEscrow', () => {
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.Escrow as Program<Escrow>;
    let testUtils: EscrowTestUtils;
    let mint: PublicKey;

    beforeEach(async () => {
        testUtils = new EscrowTestUtils(program, provider);
        await testUtils.airdropToAll(3);
        await testUtils.initializeProgramConfig(100); // 1% fee
        mint = await testUtils.createTestToken();
    });

    describe('Deployment', () => {
        it('Program config is initialized correctly', async () => {
            const [programConfig] = testUtils.getProgramConfigPDA();
            const config = await program.account.programConfig.fetch(
                programConfig
            );

            //verify program config exists and has expected structure
            expect(config.authority).to.be.instanceOf(anchor.web3.PublicKey);
            expect(config.feeVault).to.be.instanceOf(anchor.web3.PublicKey);
            expect(config.defaultFeeBps).to.equal(100);
            expect(config.paused).to.equal(false);
            expect(config.bump).to.be.a('number');
        });
    });

    describe('Create Escrows', () => {
        describe('Happy Paths', () => {
            it('can create a new native currency escrow', async () => {
                const amount = 10000000; // 0.01 SOL
                const nonce = Date.now();

                const escrow = await testUtils.createAsymEscrow(
                    testUtils.accounts.admin,
                    testUtils.accounts.payer1.publicKey,
                    testUtils.accounts.receiver1.publicKey,
                    amount,
                    null // native SOL
                );

                await testUtils.verifyAsymEscrow(escrow, {
                    payer: testUtils.accounts.payer1.publicKey,
                    receiver: testUtils.accounts.receiver1.publicKey,
                    amount,
                    currency: PublicKey.default, // native SOL
                    status: EscrowStatus.Pending,
                });
            });

            it('can create a new token currency escrow', async () => {
                const amount = 10000000;

                const escrow = await testUtils.createAsymEscrow(
                    testUtils.accounts.admin,
                    testUtils.accounts.payer1.publicKey,
                    testUtils.accounts.receiver1.publicKey,
                    amount,
                    mint // SPL token
                );

                await testUtils.verifyAsymEscrow(escrow, {
                    payer: testUtils.accounts.payer1.publicKey,
                    receiver: testUtils.accounts.receiver1.publicKey,
                    amount,
                    currency: mint,
                    status: EscrowStatus.Pending,
                });
            });

            it('can create an escrow with start and end times', async () => {
                const amount = 10000000;
                const now = Math.floor(Date.now() / 1000);
                const startTime = now + 3600; // 1 hour from now
                const endTime = now + 7200; // 2 hours from now
                const nonce = Date.now();

                const [escrow] = testUtils.getAsymEscrowPDA(
                    testUtils.accounts.admin.publicKey,
                    nonce
                );
                const [programConfig] = testUtils.getProgramConfigPDA();

                await program.methods
                    .createAsymEscrow({
                        payer: testUtils.accounts.payer1.publicKey,
                        receiver: testUtils.accounts.receiver1.publicKey,
                        currency: PublicKey.default,
                        amount: new anchor.BN(amount),
                        startTime: new anchor.BN(startTime),
                        endTime: new anchor.BN(endTime),
                        nonce: new anchor.BN(nonce),
                    })
                    .accounts({
                        creator: testUtils.accounts.admin.publicKey,
                        escrow,
                        programConfig,
                        tokenMint: null,
                        systemProgram: anchor.web3.SystemProgram.programId,
                    })
                    .signers([testUtils.accounts.admin])
                    .rpc();

                const escrowAccount = await program.account.asymEscrow.fetch(
                    escrow
                );
                expect(escrowAccount.startTime.toNumber()).to.equal(startTime);
                expect(escrowAccount.endTime.toNumber()).to.equal(endTime);
            });
        });
    });
});
