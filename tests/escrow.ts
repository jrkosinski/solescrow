import * as anchor from '@coral-xyz/anchor';
import { Program } from '@coral-xyz/anchor';
import { Escrow } from '../target/types/escrow';
import { PublicKey } from '@solana/web3.js';
import { expect } from 'chai';
import { SolanaEscrowTestUtils } from './utils';

describe('AsymEscrow', () => {
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.Escrow as Program<Escrow>;
    let testUtils: SolanaEscrowTestUtils;
    let mint: PublicKey;

    beforeEach(async () => {
        testUtils = new SolanaEscrowTestUtils(program, provider);
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

            expect(config.authority.toString()).to.equal(
                testUtils.accounts.admin.publicKey.toString()
            );
            expect(config.feeVault.toString()).to.equal(
                testUtils.accounts.feeVault.publicKey.toString()
            );
            expect(config.defaultFeeBps).to.equal(100);
            expect(config.paused).to.equal(false);
        });
    });
});
