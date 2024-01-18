import { PublicKey } from '@solana/web3.js';
import { MangoAccount } from './mangoAccount';
import BN from 'bn.js';
import { Bank } from './bank';
import { toNative } from '../utils';
import { expect } from 'chai';
import { I80F48 } from '../numbers/I80F48';

describe('Mango Account', () => {
  const mangoAccount = new MangoAccount(
    PublicKey.default,
    PublicKey.default,
    PublicKey.default,
    [],
    PublicKey.default,
    0,
    false,
    false,
    new BN(0),
    new BN(0),
    new BN(100),
    new BN(0),
    new BN(0),
    new BN(0),
    new BN(0),
    0,
    [],
    [],
    [],
    [],
    [],
    new Map(),
  );

  it('test calculateEquivalentSourceAmount', (done) => {
    const mockedSOLBank = {
      mintDecimals: 9,
      price: I80F48.fromNumber(0.09870201707999726),
    };
    const SOLDepositLimitLeft = new BN(
      toNative(0.5, mockedSOLBank.mintDecimals),
    );

    const mockedUSDCBank = {
      mintDecimals: 6,
      price: I80F48.fromNumber(1),
    };
    const USDCDepositLimitLeft = new BN(
      toNative(1, mockedUSDCBank.mintDecimals),
    );

    const maxSourceForUSDCTarget = mangoAccount.calculateEquivalentSourceAmount(
      mockedSOLBank as Bank,
      mockedUSDCBank as Bank,
      USDCDepositLimitLeft,
    );

    const maxSourceForSOLTarget = mangoAccount.calculateEquivalentSourceAmount(
      mockedUSDCBank as Bank,
      mockedSOLBank as Bank,
      SOLDepositLimitLeft,
    );

    // Expected u can sell max 0.01 sol for 1 USDC
    expect(
      maxSourceForUSDCTarget.eq(toNative(0.01, mockedSOLBank.mintDecimals)),
    ).to.be.true;

    // Expected u can buy max of 0.5 SOL for 50 USDC
    expect(maxSourceForSOLTarget.eq(toNative(50, mockedUSDCBank.mintDecimals)))
      .to.be.true;

    done();
  });
});
