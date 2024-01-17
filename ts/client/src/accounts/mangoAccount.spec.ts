import { PublicKey } from '@solana/web3.js';
import { MangoAccount } from './mangoAccount';
import BN from 'bn.js';
import { Bank } from './bank';
import { toNative } from '../utils';
import { expect } from 'chai';

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
      uiPrice: 100,
    };
    const SOLDepositLimitLeft = new BN(
      toNative(0.5, mockedSOLBank.mintDecimals),
    );

    const mockedUSDCBank = {
      mintDecimals: 6,
      uiPrice: 1,
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

    // Expected: 1 USDC can buy 0.01 SOL
    expect(
      maxSourceForUSDCTarget.eq(toNative(0.01, mockedSOLBank.mintDecimals)),
    ).to.be.true;

    // Expected: 0.5 SOL can be converted to 50 USDC
    expect(maxSourceForSOLTarget.eq(toNative(50, mockedUSDCBank.mintDecimals)))
      .to.be.true;

    done();
  });
});
