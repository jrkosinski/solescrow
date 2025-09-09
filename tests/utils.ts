import * as anchor from '@coral-xyz/anchor';
import { Program } from '@coral-xyz/anchor';
import { Escrow } from '../target/types/escrow';
import {
    PublicKey,
    Keypair,
    SystemProgram,
    LAMPORTS_PER_SOL,
    Transaction,
    sendAndConfirmTransaction,
} from '@solana/web3.js';
import {
    TOKEN_PROGRAM_ID,
    createMint,
    createAccount,
    mintTo,
    getAccount,
} from '@solana/spl-token';
import { expect } from 'chai';

export const ESCROW_SEED = 'asym_escrow';
export const SYM_ESCROW_SEED = 'sym_escrow';
export const ARBITRATION_PROPOSAL_SEED = 'arbitration_proposal';
export const PROGRAM_CONFIG_SEED = 'program_config';
export const ESCROW_VAULT_SEED = 'escrow_vault';

export enum EscrowStatus {
    Pending = 'pending',
    Active = 'active',
    Completed = 'completed',
    Arbitration = 'arbitration',
}

export interface TestAccounts {
    admin: Keypair;
    payer1: Keypair;
    payer2: Keypair;
    receiver1: Keypair;
    receiver2: Keypair;
    feeVault: Keypair;
}

export class EscrowTestUtils {
    program: Program<Escrow>;
    provider: anchor.AnchorProvider;
    accounts: TestAccounts;
    mint: PublicKey | null = null;

    constructor(program: Program<Escrow>, provider: anchor.AnchorProvider) {
        this.program = program;
        this.provider = provider;
        this.accounts = this.generateTestAccounts();
    }

    generateTestAccounts(): TestAccounts {
        return {
            admin: Keypair.generate(),
            payer1: Keypair.generate(),
            payer2: Keypair.generate(),
            receiver1: Keypair.generate(),
            receiver2: Keypair.generate(),
            feeVault: Keypair.generate(),
        };
    }

    async airdropSol(pubkey: PublicKey, amount: number = 1) {
        const signature = await this.provider.connection.requestAirdrop(
            pubkey,
            amount * LAMPORTS_PER_SOL
        );
        await this.provider.connection.confirmTransaction(signature);
    }

    async airdropToAll(amount: number = 2) {
        const accounts = Object.values(this.accounts);
        for (const account of accounts) {
            await this.airdropSol(account.publicKey, amount);
        }
    }

    async createTestToken(): Promise<PublicKey> {
        // Create mint
        this.mint = await createMint(
            this.provider.connection,
            this.accounts.admin,
            this.accounts.admin.publicKey,
            null,
            9
        );
        return this.mint;
    }

    async createTokenAccount(owner: PublicKey): Promise<PublicKey> {
        if (!this.mint) throw new Error('Test token not created');

        return await createAccount(
            this.provider.connection,
            this.accounts.admin,
            this.mint,
            owner
        );
    }

    async mintTokens(account: PublicKey, amount: number) {
        if (!this.mint) throw new Error('Test token not created');

        await mintTo(
            this.provider.connection,
            this.accounts.admin,
            this.mint,
            account,
            this.accounts.admin,
            amount
        );
    }

    async getTokenBalance(account: PublicKey): Promise<number> {
        const accountInfo = await getAccount(this.provider.connection, account);
        return Number(accountInfo.amount);
    }

    // PDA derivation helpers
    getAsymEscrowPDA(creator: PublicKey, nonce: number): [PublicKey, number] {
        const nonceBytes = Buffer.alloc(8);
        nonceBytes.writeBigUInt64LE(BigInt(nonce));

        return PublicKey.findProgramAddressSync(
            [Buffer.from(ESCROW_SEED), creator.toBuffer(), nonceBytes],
            this.program.programId
        );
    }

    getProgramConfigPDA(): [PublicKey, number] {
        return PublicKey.findProgramAddressSync(
            [Buffer.from(PROGRAM_CONFIG_SEED)],
            this.program.programId
        );
    }

    getEscrowVaultPDA(escrow: PublicKey): [PublicKey, number] {
        return PublicKey.findProgramAddressSync(
            [Buffer.from(ESCROW_VAULT_SEED), escrow.toBuffer()],
            this.program.programId
        );
    }

    // Helper to generate unique escrow ID
    generateEscrowId(creator: PublicKey, nonce: number): Buffer {
        const hash = anchor.web3.Keypair.generate().publicKey.toBuffer();
        return hash.slice(0, 32);
    }

    // Initialize program config
    async initializeProgramConfig(feeBps: number = 100) {
        const [programConfig] = this.getProgramConfigPDA();

        try {
            //check if program config already exists
            await this.program.account.programConfig.fetch(programConfig);
            //if it exists, skip initialization
            return programConfig;
        } catch (error) {
            //account doesn't exist, proceed with initialization
        }

        await this.program.methods
            .initializeProgram({
                feeVault: this.accounts.feeVault.publicKey,
                defaultFeeBps: feeBps,
            })
            .accounts({
                authority: this.accounts.admin.publicKey,
                programConfig,
                systemProgram: SystemProgram.programId,
            } as any)
            .signers([this.accounts.admin])
            .rpc();

        return programConfig;
    }

    // Create asymmetric escrow helper
    async createAsymEscrow(
        creator: Keypair,
        payer: PublicKey,
        receiver: PublicKey,
        amount: number,
        currency: PublicKey | null = null,
        nonce: number = Date.now()
    ) {
        const [escrow] = this.getAsymEscrowPDA(creator.publicKey, nonce);
        const [programConfig] = this.getProgramConfigPDA();

        const params = {
            payer,
            receiver,
            currency: currency || PublicKey.default,
            amount: new anchor.BN(amount),
            startTime: new anchor.BN(0),
            endTime: new anchor.BN(0),
            nonce: new anchor.BN(nonce),
        };

        await this.program.methods
            .createAsymEscrow(params)
            .accounts({
                creator: creator.publicKey,
                escrow,
                programConfig,
                tokenMint: currency,
                systemProgram: SystemProgram.programId,
            })
            .signers([creator])
            .rpc();

        return escrow;
    }

    // Verification helpers
    async verifyAsymEscrow(
        escrowPubkey: PublicKey,
        expected: {
            payer?: PublicKey;
            receiver?: PublicKey;
            amount?: number;
            currency?: PublicKey;
            status?: EscrowStatus;
        }
    ) {
        const escrow = await this.program.account.asymEscrow.fetch(
            escrowPubkey
        );

        if (expected.payer) {
            expect(escrow.payer.addr.toString()).to.equal(
                expected.payer.toString()
            );
        }
        if (expected.receiver) {
            expect(escrow.receiver.addr.toString()).to.equal(
                expected.receiver.toString()
            );
        }
        if (expected.amount !== undefined) {
            expect(escrow.payer.amount.toNumber()).to.equal(expected.amount);
        }
        if (expected.currency) {
            expect(escrow.payer.currency.toString()).to.equal(
                expected.currency.toString()
            );
        }
        if (expected.status !== undefined) {
            expect(Object.keys(escrow.status)[0]).to.equal(
                expected.status.toLowerCase()
            );
        }
    }
}
