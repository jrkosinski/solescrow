import * as anchor from '@coral-xyz/anchor';
import { Program } from '@coral-xyz/anchor';
import { Escrow } from '../target/types/escrow';
import { LAMPORTS_PER_SOL, PublicKey } from '@solana/web3.js';
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

    describe('Place Payments', () => {
        let escrow: PublicKey;
        const amount = LAMPORTS_PER_SOL; // 1 SOL

        beforeEach(async () => {
            escrow = await testUtils.createAsymEscrow(
                testUtils.accounts.admin,
                testUtils.accounts.payer1.publicKey,
                testUtils.accounts.receiver1.publicKey,
                amount
            );
        });

        describe('Happy Paths', () => {
            it('can make a payment to native currency escrow', async () => {
                await testUtils.placePaymentAsym(
                    testUtils.accounts.payer1,
                    escrow,
                    amount / 2 // Partial payment
                );

                const escrowAccount = await program.account.asymEscrow.fetch(
                    escrow
                );
                expect(escrowAccount.payer.amountPaid.toNumber()).to.equal(
                    amount / 2
                );
                expect(Object.keys(escrowAccount.status)[0]).to.equal('active');
            });

            it('can make multiple payments', async () => {
                // First payment
                await testUtils.placePaymentAsym(
                    testUtils.accounts.payer1,
                    escrow,
                    Math.floor(amount / 3)
                );

                // Second payment
                await testUtils.placePaymentAsym(
                    testUtils.accounts.payer1,
                    escrow,
                    Math.floor(amount / 3)
                );

                const escrowAccount = await program.account.asymEscrow.fetch(
                    escrow
                );
                expect(escrowAccount.payer.amountPaid.toNumber()).to.equal(
                    Math.floor((amount * 2) / 3)
                );
            });

            it('can complete payment and escrow becomes fully paid', async () => {
                await testUtils.placePaymentAsym(
                    testUtils.accounts.payer1,
                    escrow,
                    amount // Full payment
                );

                const escrowAccount = await program.account.asymEscrow.fetch(
                    escrow
                );
                expect(escrowAccount.payer.amountPaid.toNumber()).to.equal(
                    amount
                );
                expect(escrowAccount.payer.amountPaid.toNumber()).to.be.gte(
                    escrowAccount.payer.amount.toNumber()
                );
            });
        });

        describe('Token Payments', () => {
            let tokenEscrow: PublicKey;
            const tokenAmount = 1000000; // 1 token (6 decimals)

            beforeEach(async () => {
                tokenEscrow = await testUtils.createAsymEscrow(
                    testUtils.accounts.admin,
                    testUtils.accounts.payer1.publicKey,
                    testUtils.accounts.receiver1.publicKey,
                    tokenAmount,
                    mint
                );
            });

            it.skip('can make token payment', async () => {
                await testUtils.placePaymentAsym(
                    testUtils.accounts.payer1,
                    tokenEscrow,
                    tokenAmount / 2,
                    mint
                );

                const escrowAccount = await program.account.asymEscrow.fetch(
                    tokenEscrow
                );
                expect(escrowAccount.payer.amountPaid.toNumber()).to.equal(
                    tokenAmount / 2
                );
            });
        });

        describe('Validation Tests', () => {
            it('rejects payment from wrong account', async () => {
                try {
                    await testUtils.placePaymentAsym(
                        testUtils.accounts.payer2, // Wrong payer
                        escrow,
                        amount / 2
                    );
                    expect.fail('Should have thrown an error');
                } catch (error) {
                    expect(error.toString()).to.include('Unauthorized');
                }
            });

            it('rejects zero amount payment', async () => {
                try {
                    await testUtils.placePaymentAsym(
                        testUtils.accounts.payer1,
                        escrow,
                        0 // Invalid amount
                    );
                    expect.fail('Should have thrown an error');
                } catch (error) {
                    expect(error.toString()).to.include('InvalidAmount');
                }
            });
        });
    });

    describe('Release Escrow', () => {
        let escrow: PublicKey;
        const amount = LAMPORTS_PER_SOL;

        beforeEach(async () => {
            escrow = await testUtils.createAsymEscrow(
                testUtils.accounts.admin,
                testUtils.accounts.payer1.publicKey,
                testUtils.accounts.receiver1.publicKey,
                amount
            );

            // Make full payment
            await testUtils.placePaymentAsym(
                testUtils.accounts.payer1,
                escrow,
                amount
            );
        });

        it('payer can give release consent', async () => {
            const [programConfig] = testUtils.getProgramConfigPDA();
            const [escrowVault] = testUtils.getEscrowVaultPDA(escrow);

            await program.methods
                .releaseEscrowAsym()
                .accounts({
                    signer: testUtils.accounts.payer1.publicKey,
                    escrow,
                    programConfig,
                    escrowVault,
                    receiver: testUtils.accounts.receiver1.publicKey,
                    feeVault: testUtils.accounts.feeVault.publicKey,
                    escrowTokenAccount: null,
                    receiverTokenAccount: null,
                    feeTokenAccount: null,
                    tokenProgram: null,
                    systemProgram: anchor.web3.SystemProgram.programId,
                })
                .signers([testUtils.accounts.payer1])
                .rpc();

            const escrowAccount = await program.account.asymEscrow.fetch(
                escrow
            );
            expect(escrowAccount.payer.released).to.equal(true);
        });

        it('receiver can give release consent', async () => {
            const [programConfig] = testUtils.getProgramConfigPDA();
            const [escrowVault] = testUtils.getEscrowVaultPDA(escrow);

            await program.methods
                .releaseEscrowAsym()
                .accounts({
                    signer: testUtils.accounts.receiver1.publicKey,
                    escrow,
                    programConfig,
                    escrowVault,
                    receiver: testUtils.accounts.receiver1.publicKey,
                    feeVault: testUtils.accounts.feeVault.publicKey,
                    escrowTokenAccount: null,
                    receiverTokenAccount: null,
                    feeTokenAccount: null,
                    tokenProgram: null,
                    systemProgram: anchor.web3.SystemProgram.programId,
                })
                .signers([testUtils.accounts.receiver1])
                .rpc();

            const escrowAccount = await program.account.asymEscrow.fetch(
                escrow
            );
            expect(escrowAccount.receiver.released).to.equal(true);
        });

        it('escrow is released when both parties give consent', async () => {
            const [programConfig] = testUtils.getProgramConfigPDA();
            const [escrowVault] = testUtils.getEscrowVaultPDA(escrow);

            const initialReceiverBalance = await provider.connection.getBalance(
                testUtils.accounts.receiver1.publicKey
            );
            const initialFeeBalance = await provider.connection.getBalance(
                testUtils.accounts.feeVault.publicKey
            );

            await program.methods
                .releaseEscrowAsym()
                .accounts({
                    signer: testUtils.accounts.receiver1.publicKey,
                    escrow,
                    programConfig,
                    escrowVault,
                    receiver: testUtils.accounts.receiver1.publicKey,
                    feeVault: testUtils.accounts.feeVault.publicKey,
                    escrowTokenAccount: null,
                    receiverTokenAccount: null,
                    feeTokenAccount: null,
                    tokenProgram: null,
                    systemProgram: anchor.web3.SystemProgram.programId,
                })
                .signers([testUtils.accounts.receiver1])
                .rpc();

            let escrowAccount = await program.account.asymEscrow.fetch(escrow);
            expect(escrowAccount.status).to.not.equal(EscrowStatus.Completed);

            await program.methods
                .releaseEscrowAsym()
                .accounts({
                    signer: testUtils.accounts.payer1.publicKey,
                    escrow,
                    programConfig,
                    escrowVault,
                    receiver: testUtils.accounts.receiver1.publicKey,
                    feeVault: testUtils.accounts.feeVault.publicKey,
                    escrowTokenAccount: null,
                    receiverTokenAccount: null,
                    feeTokenAccount: null,
                    tokenProgram: null,
                    systemProgram: anchor.web3.SystemProgram.programId,
                })
                .signers([testUtils.accounts.payer1])
                .rpc();

            escrowAccount = await program.account.asymEscrow.fetch(escrow);
            expect(escrowAccount.status).to.equal(EscrowStatus.Completed);

            //TODO: finish this test
        });
    });

    describe('Refund Escrow', () => {
        let escrow: PublicKey;
        const amount = LAMPORTS_PER_SOL;

        beforeEach(async () => {
            escrow = await testUtils.createAsymEscrow(
                testUtils.accounts.admin,
                testUtils.accounts.payer1.publicKey,
                testUtils.accounts.receiver1.publicKey,
                amount
            );

            // Make full payment
            await testUtils.placePaymentAsym(
                testUtils.accounts.payer1,
                escrow,
                amount
            );
        });

        it('receiver can do a full refund', async () => {
            const [programConfig] = testUtils.getProgramConfigPDA();
            const [escrowVault] = testUtils.getEscrowVaultPDA(escrow);

            await program.methods
                .refundEscrowAsym(new anchor.BN(amount))
                .accounts({
                    signer: testUtils.accounts.receiver1.publicKey,
                    escrow,
                    programConfig,
                    escrowVault,
                    payer: testUtils.accounts.payer1.publicKey,
                    escrowTokenAccount: null,
                    payerTokenAccount: null,
                    tokenProgram: null,
                    systemProgram: anchor.web3.SystemProgram.programId,
                })
                .signers([testUtils.accounts.receiver1])
                .rpc();
        });
    });
});
