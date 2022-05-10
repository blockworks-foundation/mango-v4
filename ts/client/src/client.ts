import { BN, Program, Provider } from '@project-serum/anchor';
import { getFeeRates, getFeeTier, Market } from '@project-serum/serum';
import { Order } from '@project-serum/serum/lib/market';
import * as spl from '@solana/spl-token';
import {
  AccountMeta,
  MemcmpFilter,
  PublicKey,
  SYSVAR_RENT_PUBKEY,
  TransactionSignature,
} from '@solana/web3.js';
import bs58 from 'bs58';
import { Bank, getMintInfoForTokenIndex } from './accounts/bank';
import { Group } from './accounts/group';
import { I80F48 } from './accounts/I80F48';
import { MangoAccount } from './accounts/mangoAccount';
import { StubOracle } from './accounts/oracle';
import {
  Serum3Market,
  Serum3OrderType,
  Serum3SelfTradeBehavior,
  Serum3Side,
} from './accounts/serum3';
import { IDL, MangoV4 } from './mango_v4';

export const MANGO_V4_ID = new PublicKey(
  'm43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD',
);

export class MangoClient {
  constructor(public program: Program<MangoV4>, public devnet?: boolean) {}

  /// public

  // Group

  public async createGroup(): Promise<TransactionSignature> {
    const adminPk = this.program.provider.wallet.publicKey;
    return await this.program.methods
      .createGroup()
      .accounts({
        admin: adminPk,
        payer: adminPk,
      })
      .rpc();
  }

  public async getGroup(groupPk: PublicKey): Promise<Group> {
    const group = Group.from(
      groupPk,
      await this.program.account.group.fetch(groupPk),
    );
    await group.reload(this);
    return group;
  }

  public async getGroupForAdmin(adminPk: PublicKey): Promise<Group> {
    const groups = (
      await this.program.account.group.all([
        {
          memcmp: {
            bytes: adminPk.toBase58(),
            offset: 8,
          },
        },
      ])
    ).map((tuple) => Group.from(tuple.publicKey, tuple.account));
    await groups[0].reload(this);
    return groups[0];
  }

  // Tokens/Banks

  public async registerToken(
    group: Group,
    mintPk: PublicKey,
    oraclePk: PublicKey,
    tokenIndex: number,
    name: string,
    util0: number,
    rate0: number,
    util1: number,
    rate1: number,
    maxRate: number,
    loanFeeRate: number,
    loanOriginationFeeRate: number,
    maintAssetWeight: number,
    initAssetWeight: number,
    maintLiabWeight: number,
    initLiabWeight: number,
    liquidationFee: number,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .registerToken(
        tokenIndex,
        name,
        { util0, rate0, util1, rate1, maxRate },
        loanFeeRate,
        loanOriginationFeeRate,
        maintAssetWeight,
        initAssetWeight,
        maintLiabWeight,
        initLiabWeight,
        liquidationFee,
      )
      .accounts({
        group: group.publicKey,
        admin: this.program.provider.wallet.publicKey,
        mint: mintPk,
        oracle: oraclePk,
        payer: this.program.provider.wallet.publicKey,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .rpc();
  }

  public async getBanksForGroup(group: Group): Promise<Bank[]> {
    return (
      await this.program.account.bank.all([
        {
          memcmp: {
            bytes: group.publicKey.toBase58(),
            offset: 24,
          },
        },
      ])
    ).map((tuple) => Bank.from(tuple.publicKey, tuple.account));
  }

  // Stub Oracle

  public async createStubOracle(
    group: Group,
    mintPk: PublicKey,
    price: number,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .createStubOracle({ val: I80F48.fromNumber(price).getData() })
      .accounts({
        group: group.publicKey,
        admin: this.program.provider.wallet.publicKey,
        tokenMint: mintPk,
        payer: this.program.provider.wallet.publicKey,
      })
      .rpc();
  }

  public async setStubOracle(
    group: Group,
    mintPk: PublicKey,
    price: number,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .setStubOracle({ val: I80F48.fromNumber(price).getData() })
      .accounts({
        group: group.publicKey,
        admin: this.program.provider.wallet.publicKey,
        tokenMint: mintPk,
        payer: this.program.provider.wallet.publicKey,
      })
      .rpc();
  }

  public async getStubOracle(
    group: Group,
    mintPk: PublicKey,
  ): Promise<StubOracle> {
    const stubOracles = (
      await this.program.account.stubOracle.all([
        {
          memcmp: {
            bytes: group.publicKey.toBase58(),
            offset: 8,
          },
        },
        {
          memcmp: {
            bytes: mintPk.toBase58(),
            offset: 40,
          },
        },
      ])
    ).map((pa) => StubOracle.from(pa.publicKey, pa.account));
    return stubOracles[0];
  }

  // MangoAccount

  public async getOrCreateMangoAccount(
    group: Group,
    ownerPk: PublicKey,
    accountNumber?: number,
    name?: string,
  ): Promise<MangoAccount> {
    let mangoAccounts = await this.getMangoAccountForOwner(group, ownerPk);
    if (mangoAccounts.length === 0) {
      await this.createMangoAccount(group, accountNumber ?? 0, name ?? '');
      mangoAccounts = await this.getMangoAccountForOwner(group, ownerPk);
    }
    return mangoAccounts[0];
  }

  public async createMangoAccount(
    group: Group,
    accountNumber: number,
    name?: string,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .createAccount(accountNumber, name ?? '')
      .accounts({
        group: group.publicKey,
        owner: this.program.provider.wallet.publicKey,
        payer: this.program.provider.wallet.publicKey,
      })
      .rpc();
  }

  public async getMangoAccount(mangoAccount: MangoAccount) {
    return MangoAccount.from(
      mangoAccount.publicKey,
      await this.program.account.mangoAccount.fetch(mangoAccount.publicKey),
    );
  }

  public async getMangoAccountForOwner(
    group: Group,
    ownerPk: PublicKey,
  ): Promise<MangoAccount[]> {
    return (
      await this.program.account.mangoAccount.all([
        {
          memcmp: {
            bytes: group.publicKey.toBase58(),
            offset: 40,
          },
        },
        {
          memcmp: {
            bytes: ownerPk.toBase58(),
            offset: 72,
          },
        },
      ])
    ).map((pa) => {
      return MangoAccount.from(pa.publicKey, pa.account);
    });
  }

  public async closeMangoAccount(
    mangoAccount: MangoAccount,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .closeAccount()
      .accounts({
        account: mangoAccount.publicKey,
        owner: this.program.provider.wallet.publicKey,
        solDestination: this.program.provider.wallet.publicKey,
      })
      .rpc();
  }

  public async deposit(
    group: Group,
    mangoAccount: MangoAccount,
    tokenName: string,
    amount: number,
  ) {
    const bank = group.banksMap.get(tokenName)!;

    const tokenAccountPk = await spl.getAssociatedTokenAddress(
      bank.mint,
      mangoAccount.owner,
    );

    const healthRemainingAccounts: PublicKey[] =
      await this.buildHealthRemainingAccounts(group, mangoAccount, bank);

    return await this.program.methods
      .deposit(new BN(amount))
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        bank: bank.publicKey,
        vault: bank.vault,
        tokenAccount: tokenAccountPk,
        tokenAuthority: this.program.provider.wallet.publicKey,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .rpc();
  }

  public async withdraw(
    group: Group,
    mangoAccount: MangoAccount,
    tokenName: string,
    amount: number,
    allowBorrow: boolean,
  ) {
    const bank = group.banksMap.get(tokenName)!;

    const tokenAccountPk = await spl.getAssociatedTokenAddress(
      bank.mint,
      mangoAccount.owner,
    );

    const healthRemainingAccounts: PublicKey[] =
      await this.buildHealthRemainingAccounts(group, mangoAccount, bank);

    return await this.program.methods
      .withdraw(new BN(amount), allowBorrow)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        bank: bank.publicKey,
        vault: bank.vault,
        tokenAccount: tokenAccountPk,
        tokenAuthority: this.program.provider.wallet.publicKey,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .rpc();
  }

  // Serum

  public async serum3RegisterMarket(
    group: Group,
    serum3ProgramId: PublicKey,
    serum3MarketExternalPk: PublicKey,
    baseBank: Bank,
    quoteBank: Bank,
    marketIndex: number,
    name: string,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .serum3RegisterMarket(marketIndex, name)
      .accounts({
        group: group.publicKey,
        admin: this.program.provider.wallet.publicKey,
        serumProgram: serum3ProgramId,
        serumMarketExternal: serum3MarketExternalPk,
        baseBank: baseBank.publicKey,
        quoteBank: quoteBank.publicKey,
        payer: this.program.provider.wallet.publicKey,
      })
      .rpc();
  }

  public async serum3GetMarket(
    group: Group,
    baseTokenIndex?: number,
    quoteTokenIndex?: number,
  ): Promise<Serum3Market[]> {
    const bumpfbuf = Buffer.alloc(1);
    bumpfbuf.writeUInt8(255);

    const filters: MemcmpFilter[] = [
      {
        memcmp: {
          bytes: group.publicKey.toBase58(),
          offset: 24,
        },
      },
    ];

    if (baseTokenIndex) {
      const bbuf = Buffer.alloc(2);
      bbuf.writeUInt16LE(baseTokenIndex);
      filters.push({
        memcmp: {
          bytes: bs58.encode(bbuf),
          offset: 122,
        },
      });
    }

    if (quoteTokenIndex) {
      const qbuf = Buffer.alloc(2);
      qbuf.writeUInt16LE(quoteTokenIndex);
      filters.push({
        memcmp: {
          bytes: bs58.encode(qbuf),
          offset: 124,
        },
      });
    }

    return (await this.program.account.serum3Market.all(filters)).map((tuple) =>
      Serum3Market.from(tuple.publicKey, tuple.account),
    );
  }

  public async serum3CreateOpenOrders(
    group: Group,
    mangoAccount: MangoAccount,
    marketName: string,
  ): Promise<TransactionSignature> {
    const serum3Market: Serum3Market = group.serum3MarketsMap.get(marketName)!;

    return await this.program.methods
      .serum3CreateOpenOrders()
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        serumMarket: serum3Market.publicKey,
        serumProgram: serum3Market.serumProgram,
        serumMarketExternal: serum3Market.serumMarketExternal,
        owner: this.program.provider.wallet.publicKey,
        payer: this.program.provider.wallet.publicKey,
      })
      .rpc();
  }

  public async serum3PlaceOrder(
    group: Group,
    mangoAccount: MangoAccount,
    serum3ProgramId: PublicKey,
    serum3MarketName: string,
    side: Serum3Side,
    price: number,
    size: number,
    selfTradeBehavior: Serum3SelfTradeBehavior,
    orderType: Serum3OrderType,
    clientOrderId: number,
    limit: number,
  ) {
    const serum3Market = group.serum3MarketsMap.get(serum3MarketName)!;

    if (!mangoAccount.findSerum3Account(serum3Market.marketIndex)) {
      await this.serum3CreateOpenOrders(group, mangoAccount, 'BTC/USDC');
      mangoAccount = await this.getMangoAccount(mangoAccount);
    }

    const serum3MarketExternal = await Market.load(
      this.program.provider.connection,
      serum3Market.serumMarketExternal,
      { commitment: this.program.provider.connection.commitment },
      serum3ProgramId,
    );

    const serum3MarketExternalVaultSigner =
      await PublicKey.createProgramAddress(
        [
          serum3Market.serumMarketExternal.toBuffer(),
          serum3MarketExternal.decoded.vaultSignerNonce.toArrayLike(
            Buffer,
            'le',
            8,
          ),
        ],
        serum3ProgramId,
      );

    const healthRemainingAccounts: PublicKey[] =
      await this.buildHealthRemainingAccounts(group, mangoAccount);

    const limitPrice = serum3MarketExternal.priceNumberToLots(price);
    const maxBaseQuantity = serum3MarketExternal.baseSizeNumberToLots(size);
    const feeTier = getFeeTier(0, 0 /** TODO: fix msrm/srm balance */);
    const rates = getFeeRates(feeTier);
    const maxQuoteQuantity = new BN(
      serum3MarketExternal.decoded.quoteLotSize.toNumber() *
        (1 + rates.taker) /** TODO: fix taker/maker */,
    ).mul(
      serum3MarketExternal
        .baseSizeNumberToLots(size)
        .mul(serum3MarketExternal.priceNumberToLots(price)),
    );

    return await this.program.methods
      .serum3PlaceOrder(
        side,
        limitPrice,
        maxBaseQuantity,
        maxQuoteQuantity,
        selfTradeBehavior,
        orderType,
        new BN(clientOrderId),
        limit,
      )
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: this.program.provider.wallet.publicKey,
        openOrders: mangoAccount.findSerum3Account(serum3Market.marketIndex)
          ?.openOrders,
        serumMarket: serum3Market.publicKey,
        serumProgram: serum3ProgramId,
        serumMarketExternal: serum3Market.serumMarketExternal,
        marketBids: serum3MarketExternal.bidsAddress,
        marketAsks: serum3MarketExternal.asksAddress,
        marketEventQueue: serum3MarketExternal.decoded.eventQueue,
        marketRequestQueue: serum3MarketExternal.decoded.requestQueue,
        marketBaseVault: serum3MarketExternal.decoded.baseVault,
        marketQuoteVault: serum3MarketExternal.decoded.quoteVault,
        marketVaultSigner: serum3MarketExternalVaultSigner,
        quoteBank: group.findBank(serum3Market.quoteTokenIndex)?.publicKey,
        quoteVault: group.findBank(serum3Market.quoteTokenIndex)?.vault,
        baseBank: group.findBank(serum3Market.baseTokenIndex)?.publicKey,
        baseVault: group.findBank(serum3Market.baseTokenIndex)?.vault,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )

      .rpc();
  }

  async serum3SettleFunds(
    group: Group,
    mangoAccount: MangoAccount,
    serum3ProgramId: PublicKey,
    serum3MarketName: string,
  ): Promise<TransactionSignature> {
    const serum3Market = group.serum3MarketsMap.get(serum3MarketName)!;

    const serum3MarketExternal = await Market.load(
      this.program.provider.connection,
      serum3Market.serumMarketExternal,
      { commitment: this.program.provider.connection.commitment },
      serum3ProgramId,
    );

    const serum3MarketExternalVaultSigner =
      // TODO: put into a helper method, and remove copy pasta
      await PublicKey.createProgramAddress(
        [
          serum3Market.serumMarketExternal.toBuffer(),
          serum3MarketExternal.decoded.vaultSignerNonce.toArrayLike(
            Buffer,
            'le',
            8,
          ),
        ],
        serum3ProgramId,
      );

    return await this.program.methods
      .serum3SettleFunds()
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: this.program.provider.wallet.publicKey,
        openOrders: mangoAccount.findSerum3Account(serum3Market.marketIndex)
          ?.openOrders,
        serumMarket: serum3Market.publicKey,
        serumProgram: serum3ProgramId,
        serumMarketExternal: serum3Market.serumMarketExternal,
        marketBaseVault: serum3MarketExternal.decoded.baseVault,
        marketQuoteVault: serum3MarketExternal.decoded.quoteVault,
        marketVaultSigner: serum3MarketExternalVaultSigner,
        quoteBank: group.findBank(serum3Market.quoteTokenIndex)?.publicKey,
        quoteVault: group.findBank(serum3Market.quoteTokenIndex)?.vault,
        baseBank: group.findBank(serum3Market.baseTokenIndex)?.publicKey,
        baseVault: group.findBank(serum3Market.baseTokenIndex)?.vault,
      })
      .rpc();
  }

  async serum3CancelOrder(
    group: Group,
    mangoAccount: MangoAccount,
    serum3ProgramId: PublicKey,
    serum3MarketName: string,
    side: Serum3Side,
    orderId: BN,
  ): Promise<TransactionSignature> {
    const serum3Market = group.serum3MarketsMap.get(serum3MarketName)!;

    const serum3MarketExternal = await Market.load(
      this.program.provider.connection,
      serum3Market.serumMarketExternal,
      { commitment: this.program.provider.connection.commitment },
      serum3ProgramId,
    );
    return await this.program.methods
      .serum3CancelOrder(side, orderId)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        openOrders: mangoAccount.findSerum3Account(serum3Market.marketIndex)
          ?.openOrders,
        serumMarket: serum3Market.publicKey,
        serumProgram: serum3ProgramId,
        serumMarketExternal: serum3Market.serumMarketExternal,
        marketBids: serum3MarketExternal.bidsAddress,
        marketAsks: serum3MarketExternal.asksAddress,
        marketEventQueue: serum3MarketExternal.decoded.eventQueue,
      })
      .rpc();
  }

  async getSerum3Orders(
    group: Group,
    serum3ProgramId: PublicKey,
    serum3MarketName: string,
  ): Promise<Order[]> {
    const serum3Market = group.serum3MarketsMap.get(serum3MarketName)!;

    const serum3MarketExternal = await Market.load(
      this.program.provider.connection,
      serum3Market.serumMarketExternal,
      { commitment: this.program.provider.connection.commitment },
      serum3ProgramId,
    );
    return await serum3MarketExternal.loadOrdersForOwner(
      this.program.provider.connection,
      group.publicKey,
    );
  }

  /// static

  static async connect(
    provider: Provider,
    devnet?: boolean,
  ): Promise<MangoClient> {
    // TODO: use IDL on chain or in repository? decide...
    // Alternatively we could fetch IDL from chain.
    // const idl = await Program.fetchIdl(MANGO_V4_ID, provider);
    let idl = IDL;

    // TODO: remove...
    // Temporarily add missing (dummy) type definitions, so we can do new Program(...) below
    // without anchor throwing errors. These types come from part of the code we don't yet care about
    // in the client.
    function addDummyType(idl: MangoV4, typeName: string) {
      if (idl.types.find((type) => type.name === typeName)) {
        return;
      }
      (idl.types as any).push({
        name: typeName,
        type: {
          kind: 'struct',
          fields: [],
        },
      });
    }
    addDummyType(idl, 'usize');
    addDummyType(idl, 'AnyNode');
    addDummyType(idl, 'EventQueueHeader');
    addDummyType(idl, 'AnyEvent');
    addDummyType(idl, 'H');
    addDummyType(idl, 'H::Item');
    addDummyType(idl, 'NodeHandle');

    return new MangoClient(
      new Program<MangoV4>(idl as MangoV4, MANGO_V4_ID, provider),
      devnet,
    );
  }

  /// private

  private async buildHealthRemainingAccounts(
    group: Group,
    mangoAccount: MangoAccount,
    bank?: Bank /** TODO for serum3PlaceOrde we are just ingoring this atm */,
  ) {
    const healthRemainingAccounts: PublicKey[] = [];

    const tokenIndices = mangoAccount.tokens
      .filter((token) => token.tokenIndex !== 65535)
      .map((token) => token.tokenIndex);

    if (bank) {
      tokenIndices.push(bank.tokenIndex);
    }

    const mintInfos = await Promise.all(
      [...new Set(tokenIndices)].map(async (tokenIndex) =>
        getMintInfoForTokenIndex(this, group.publicKey, tokenIndex),
      ),
    );
    healthRemainingAccounts.push(
      ...mintInfos.flatMap((mintinfos) => {
        return mintinfos.flatMap((mintinfo) => {
          return mintinfo.bank;
        });
      }),
    );
    healthRemainingAccounts.push(
      ...mintInfos.flatMap((mintinfos) => {
        return mintinfos.flatMap((mintinfo) => {
          return mintinfo.oracle;
        });
      }),
    );
    healthRemainingAccounts.push(
      ...mangoAccount.serum3
        .filter((serum3Account) => serum3Account.marketIndex !== 65535)
        .map((serum3Account) => serum3Account.openOrders),
    );

    return healthRemainingAccounts;
  }
}
