import { Connection, PublicKey } from '@solana/web3.js';
import { expect } from 'chai';

import { USDC_MINT_MAINNET, deriveFallbackOracleQuoteKey, isOrcaOracle, isRaydiumOracle } from './oracle';

describe.only('Oracle', () => {
    const Orca_SOL_USDC_Whirlpool = new PublicKey('83v8iPyZihDEjDdY8RdZddyZNyUtXngz69Lgo9Kt5d6d')
    const Raydium_SOL_USDC_Whirlpool = new PublicKey('Ds33rQ1d4AXwxqyeXX6Pc3G4pFNr6iWb3dd8YfBBQMPr')
    const connection = new Connection('https://api.mainnet-beta.solana.com/')


  it('can decode Orca CLMM oracles', async () => {
    const accInfo = await connection.getAccountInfo(Orca_SOL_USDC_Whirlpool)
    expect(accInfo).not.to.be.null;
    expect(isOrcaOracle(accInfo!)).to.be.true;

    const other = await connection.getAccountInfo(Raydium_SOL_USDC_Whirlpool)
    expect(isOrcaOracle(other!)).to.be.false;

    const quoteKey = deriveFallbackOracleQuoteKey(accInfo!)
    expect(quoteKey.equals(USDC_MINT_MAINNET)).to.be.true
  });

  it('can decode Raydium CLMM oracles', async () => {
    const accInfo = await connection.getAccountInfo(Raydium_SOL_USDC_Whirlpool)
    expect(accInfo).not.to.be.null;
    expect(isRaydiumOracle(accInfo!)).to.be.true;

    const other = await connection.getAccountInfo(Orca_SOL_USDC_Whirlpool)
    expect(isRaydiumOracle(other!)).to.be.false;

    const quoteKey = deriveFallbackOracleQuoteKey(accInfo!)
    expect(quoteKey.equals(USDC_MINT_MAINNET)).to.be.true
  });
});
