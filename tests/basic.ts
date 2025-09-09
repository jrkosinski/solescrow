import * as anchor from '@coral-xyz/anchor';
import { Program } from '@coral-xyz/anchor';
import { Escrow } from '../target/types/escrow';
import { SolanaEscrowTestUtils, EscrowStatus } from './utils';

describe('Basic Escrow Test', () => {
    let program: Program<Escrow>;
    let provider: anchor.AnchorProvider;
    let utils: SolanaEscrowTestUtils;

    before(async () => {
        // Configure the client to use the local cluster
        anchor.setProvider(anchor.AnchorProvider.env());
        provider = anchor.AnchorProvider.env();
        program = anchor.workspace.Escrow as Program<Escrow>;
        utils = new SolanaEscrowTestUtils(program, provider);

        // Airdrop SOL to test accounts
        await utils.airdropToAll(2);

        // Initialize program config
        await utils.initializeProgramConfig(100);
    });

    it('can initialize program config', async () => {
        const [programConfig] = utils.getProgramConfigPDA();
        const config = await program.account.programConfig.fetch(programConfig);
        console.log('Program config initialized:', config);
    });
});
