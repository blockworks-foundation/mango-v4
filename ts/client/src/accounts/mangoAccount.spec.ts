import { PublicKey } from '@solana/web3.js';
import { MangoAccount } from './mangoAccount';
import BN from 'bn.js';
import { Bank } from './bank';
import { toNative, toUiDecimals } from '../utils';
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
      price: I80F48.fromNumber(1.0000536899999979),
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
      toUiDecimals(maxSourceForUSDCTarget, mockedSOLBank.mintDecimals).toFixed(
        2,
      ) === '0.01',
    ).to.be.true;

    // Expected u can buy max of 0.49 SOL for 49 USDC
    expect(
      toUiDecimals(maxSourceForSOLTarget, mockedUSDCBank.mintDecimals).toFixed(
        0,
      ) === '49',
    ).to.be.true;

    done();
  });
});
