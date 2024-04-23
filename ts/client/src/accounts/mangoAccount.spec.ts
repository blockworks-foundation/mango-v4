import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { expect } from 'chai';
import { assert } from 'console';
import { I80F48, ONE_I80F48, ZERO_I80F48 } from '../numbers/I80F48';
import { deepClone, toNative, toUiDecimals } from '../utils';
import { Bank, TokenIndex } from './bank';
import { Group } from './group';
import { MangoAccount, TokenPosition } from './mangoAccount';

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
    0,
    new BN(0),
    [],
    [],
    [],
    [],
    [],
    [],
    new Map(),
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

describe('maxWithdraw', () => {
  const protoAccount = new MangoAccount(
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
    new BN(0),
    new BN(0),
    new BN(0),
    new BN(0),
    new BN(0),
    0,
    0,
    new BN(0),
    [],
    [],
    [],
    [],
    [],
    [],
    new Map(),
    new Map(),
  );
  protoAccount.tokens.push(
    new TokenPosition(ZERO_I80F48(), 0 as TokenIndex, 0, ZERO_I80F48(), 0, 0),
  );
  protoAccount.tokens.push(
    new TokenPosition(ZERO_I80F48(), 1 as TokenIndex, 0, ZERO_I80F48(), 0, 0),
  );

  const protoBank = {
    vault: PublicKey.default,
    mint: PublicKey.default,
    tokenIndex: 0,
    price: ONE_I80F48(),
    getAssetPrice() {
      return this.price;
    },
    getLiabPrice() {
      return this.price;
    },
    stablePriceModel: { stablePrice: ONE_I80F48() },
    initAssetWeight: I80F48.fromNumber(0.8),
    initLiabWeight: I80F48.fromNumber(1.2),
    maintWeights() {
      return [I80F48.fromNumber(0.9), I80F48.fromNumber(1.1)];
    },
    scaledInitAssetWeight(price) {
      return this.initAssetWeight;
    },
    scaledInitLiabWeight(price) {
      return this.initLiabWeight;
    },
    loanOriginationFeeRate: I80F48.fromNumber(0.001),
    minVaultToDepositsRatio: I80F48.fromNumber(0.1),
    depositIndex: I80F48.fromNumber(1000000),
    borrowIndex: I80F48.fromNumber(1000000),
    indexedDeposits: I80F48.fromNumber(0),
    indexedBorrows: I80F48.fromNumber(0),
    nativeDeposits() {
      return this.depositIndex.mul(this.indexedDeposits);
    },
    nativeBorrows() {
      return this.borrowIndex.mul(this.indexedBorrows);
    },
    areBorrowsReduceOnly() {
      return false;
    },
  } as any as Bank;

  function makeGroup(bank0, bank1, vaultAmount) {
    return {
      getFirstBankByMint(mint) {
        if (mint.equals(bank0.mint)) {
          return bank0;
        } else if (mint.equals(bank1.mint)) {
          return bank1;
        }
      },
      getFirstBankByTokenIndex(tokenIndex) {
        return [bank0, bank1][tokenIndex];
      },
      getFirstBankForPerpSettlement() {
        return bank0;
      },
      vaultAmountsMap: new Map<string, BN>([
        [bank0.vault.toBase58(), new BN(vaultAmount)],
      ]),
    } as any as Group;
  }

  function setup(vaultAmount): [Group, Bank, Bank, MangoAccount] {
    const account = deepClone<MangoAccount>(protoAccount);
    const bank0 = deepClone(protoBank);
    const bank1 = deepClone(protoBank);
    bank1.tokenIndex = 1 as TokenIndex;
    bank1.mint = PublicKey.unique();
    bank1.initAssetWeight = ONE_I80F48();
    bank1.initLiabWeight = ONE_I80F48();
    const group = makeGroup(bank0, bank1, vaultAmount);
    return [group, bank0, bank1, account];
  }

  function deposit(bank, account, amount) {
    const amountV = I80F48.fromNumber(amount);
    const indexedAmount = amountV.div(bank.depositIndex);
    if (indexedAmount.mul(bank.depositIndex).lt(amountV)) {
      const delta = new I80F48(new BN(1));
      indexedAmount.iadd(delta);
    }
    bank.indexedDeposits.iadd(indexedAmount);
    const tp = account.tokens[bank.tokenIndex];
    assert(!tp.indexedPosition.isNeg());
    tp.indexedPosition.iadd(indexedAmount);
  }

  function borrow(bank, account, amount) {
    const indexedAmount = I80F48.fromNumber(amount).div(bank.borrowIndex);
    bank.indexedBorrows.iadd(indexedAmount);
    const tp = account.tokens[bank.tokenIndex];
    assert(!tp.indexedPosition.isPos());
    tp.indexedPosition.isub(indexedAmount);
  }

  function maxWithdraw(group, account) {
    return account
      .getMaxWithdrawWithBorrowForToken(group, PublicKey.default)
      .toNumber();
  }

  it('full withdraw', (done) => {
    const [group, bank0, bank1, account] = setup(1000000);
    deposit(bank0, account, 100);
    expect(maxWithdraw(group, account)).equal(100);
    done();
  });

  it('full withdraw limited vault', (done) => {
    const [group, bank0, bank1, account] = setup(90);
    deposit(bank0, account, 100);
    expect(maxWithdraw(group, account)).equal(90);
    done();
  });

  it('full withdraw limited utilization', (done) => {
    const [group, bank0, bank1, account] = setup(1000000);
    const other = deepClone(account);
    deposit(bank0, account, 100);
    borrow(bank0, other, 50);
    expect(maxWithdraw(group, account)).equal(50);
    done();
  });

  it('withdraw limited health', (done) => {
    const [group, bank0, bank1, account] = setup(1000000);
    deposit(bank0, account, 100);
    borrow(bank1, account, 50);
    expect(maxWithdraw(group, account)).equal(Math.floor(100 - 50 / 0.8));
    done();
  });

  it('pure borrow', (done) => {
    const [group, bank0, bank1, account] = setup(1000000);
    const other = deepClone(account);
    deposit(bank0, other, 1000); // so there's something to borrow
    deposit(bank1, account, 100);
    expect(maxWithdraw(group, account)).equal(Math.floor(100 / 1.2));
    done();
  });

  it('pure borrow limited utilization', (done) => {
    const [group, bank0, bank1, account] = setup(1000000);
    const other = deepClone(account);
    deposit(bank0, other, 50);
    deposit(bank1, account, 100);
    expect(maxWithdraw(group, account)).equal(44); // due to origination fees!

    bank0.loanOriginationFeeRate = ZERO_I80F48();
    expect(maxWithdraw(group, account)).equal(45);

    done();
  });

  it('withdraw and borrow', (done) => {
    const [group, bank0, bank1, account] = setup(1000000);
    const other = deepClone(account);
    deposit(bank0, account, 100);
    deposit(bank1, account, 100);
    deposit(bank0, other, 10000);
    expect(maxWithdraw(group, account)).equal(100 + Math.floor(100 / 1.2));
    done();
  });

  it('withdraw limited health and scaling', (done) => {
    const [group, bank0, bank1, account] = setup(1000000);
    bank0.scaledInitAssetWeight = function (price) {
      const startScale = I80F48.fromNumber(50);
      if (this.nativeDeposits().gt(startScale)) {
        return this.initAssetWeight.div(this.nativeDeposits().div(startScale));
      }
      return this.initAssetWeight;
    };
    const other = deepClone(account);
    deposit(bank0, other, 100);
    deposit(bank0, account, 200);
    borrow(bank1, account, 20);
    // initial account health = 200 * 0.8 * 50 / 300 - 20 = 6.66
    // zero account health = 100 * 0.8 * 50 / 200 - 20 = 0
    // so can withdraw 100
    expect(maxWithdraw(group, account)).equal(100);
    done();
  });

  it('borrow limited health and scaling', (done) => {
    const [group, bank0, bank1, account] = setup(1000000);
    bank0.scaledInitLiabWeight = function (price) {
      const startScale = I80F48.fromNumber(50);
      if (this.nativeBorrows().gt(startScale)) {
        return this.initLiabWeight.mul(this.nativeBorrows().div(startScale));
      }
      return this.initLiabWeight;
    };
    const other = deepClone(account);
    deposit(bank0, other, 100);
    deposit(bank1, account, 100);
    // -64*1.2*64/50+100 = 1.69
    // -65*1.2*65/50+100 = -1.4
    expect(maxWithdraw(group, account)).equal(64);
    done();
  });
});
