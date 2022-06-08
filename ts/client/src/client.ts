import { ORCA_TOKEN_SWAP_ID_DEVNET } from '@orca-so/sdk';
import { orcaDevnetPoolConfigs } from '@orca-so/sdk/dist/constants/devnet/pools';
import { OrcaPoolConfig as OrcaDevnetPoolConfig } from '@orca-so/sdk/dist/public/devnet/pools';
import { AnchorProvider, BN, Program, Provider } from '@project-serum/anchor';
import { getFeeRates, getFeeTier, Market } from '@project-serum/serum';
import { Order } from '@project-serum/serum/lib/market';
import {
  closeAccount,
  initializeAccount,
  WRAPPED_SOL_MINT,
} from '@project-serum/serum/lib/token-instructions';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { TokenSwap } from '@solana/spl-token-swap';
import {
  AccountMeta,
  Keypair,
  LAMPORTS_PER_SOL,
  MemcmpFilter,
  PublicKey,
  Signer,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  TransactionInstruction,
  TransactionSignature,
} from '@solana/web3.js';
import bs58 from 'bs58';
import { Bank, getMintInfoForTokenIndex } from './accounts/bank';
import { Group } from './accounts/group';
import { I80F48 } from './accounts/I80F48';
import { MangoAccount } from './accounts/mangoAccount';
import { StubOracle } from './accounts/oracle';
import { OrderType, PerpMarket, Side } from './accounts/perp';
import {
  Serum3Market,
  Serum3OrderType,
  Serum3SelfTradeBehavior,
  Serum3Side,
} from './accounts/serum3';
import { IDL, MangoV4 } from './mango_v4';
import { MarginTradeWithdraw } from './types';
import {
  getAssociatedTokenAddress,
  I64_MAX_BN,
  toNativeDecimals,
  toU64,
} from './utils';

export const MANGO_V4_ID = new PublicKey(
  'm43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD',
);

export class MangoClient {
  constructor(public program: Program<MangoV4>, public devnet?: boolean) {}

  /// public

  // Group

  public async createGroup(groupNum: number): Promise<TransactionSignature> {
    const adminPk = (this.program.provider as AnchorProvider).wallet.publicKey;
    return await this.program.methods
      .createGroup(groupNum)
      .accounts({
        admin: adminPk,
        payer: adminPk,
      })
      .rpc();
  }

  public async getGroup(groupPk: PublicKey): Promise<Group> {
    const groupAccount = await this.program.account.group.fetch(groupPk);
    const group = Group.from(groupPk, groupAccount);
    await group.reload(this);
    return group;
  }

  public async getGroupForAdmin(
    adminPk: PublicKey,
    groupNum?: number,
  ): Promise<Group> {
    const filters: MemcmpFilter[] = [
      {
        memcmp: {
          bytes: adminPk.toBase58(),
          offset: 8,
        },
      },
    ];

    if (groupNum) {
      const bbuf = Buffer.alloc(4);
      bbuf.writeUInt32LE(groupNum);
      filters.push({
        memcmp: {
          bytes: bs58.encode(bbuf),
          offset: 44,
        },
      });
    }

    const groups = (await this.program.account.group.all(filters)).map(
      (tuple) => Group.from(tuple.publicKey, tuple.account),
    );
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
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        mint: mintPk,
        oracle: oraclePk,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
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
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        tokenMint: mintPk,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .rpc();
  }

  public async setStubOracle(
    group: Group,
    oraclePk: PublicKey,
    price: number,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .setStubOracle({ val: I80F48.fromNumber(price).getData() })
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        oracle: oraclePk,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
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
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
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
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        solDestination: (this.program.provider as AnchorProvider).wallet
          .publicKey,
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

    const tokenAccountPk = await getAssociatedTokenAddress(
      bank.mint,
      mangoAccount.owner,
    );

    let wrappedSolAccount: Keypair | undefined;
    let preInstructions: TransactionInstruction[] = [];
    let postInstructions: TransactionInstruction[] = [];
    let additionalSigners: Signer[] = [];
    if (bank.mint.equals(WRAPPED_SOL_MINT)) {
      wrappedSolAccount = new Keypair();
      const lamports = Math.round(amount * LAMPORTS_PER_SOL) + 1e7;

      preInstructions = [
        SystemProgram.createAccount({
          fromPubkey: mangoAccount.owner,
          newAccountPubkey: wrappedSolAccount.publicKey,
          lamports,
          space: 165,
          programId: TOKEN_PROGRAM_ID,
        }),
        initializeAccount({
          account: wrappedSolAccount.publicKey,
          mint: WRAPPED_SOL_MINT,
          owner: mangoAccount.owner,
        }),
      ];
      postInstructions = [
        closeAccount({
          source: wrappedSolAccount.publicKey,
          destination: mangoAccount.owner,
          owner: mangoAccount.owner,
        }),
      ];
      additionalSigners.push(wrappedSolAccount);
    }

    const healthRemainingAccounts: PublicKey[] =
      await this.buildHealthRemainingAccounts(group, mangoAccount, [bank]);

    return await this.program.methods
      .deposit(toNativeDecimals(amount, bank.mintDecimals))
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        bank: bank.publicKey,
        vault: bank.vault,
        tokenAccount: wrappedSolAccount?.publicKey ?? tokenAccountPk,
        tokenAuthority: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .preInstructions(preInstructions)
      .postInstructions(postInstructions)
      .signers(additionalSigners)
      .rpc({ skipPreflight: true });
  }

  public async withdraw(
    group: Group,
    mangoAccount: MangoAccount,
    tokenName: string,
    amount: number,
    allowBorrow: boolean,
  ) {
    const bank = group.banksMap.get(tokenName)!;

    const tokenAccountPk = await getAssociatedTokenAddress(
      bank.mint,
      mangoAccount.owner,
    );

    const healthRemainingAccounts: PublicKey[] =
      await this.buildHealthRemainingAccounts(group, mangoAccount, [bank]);

    return await this.program.methods
      .withdraw(toNativeDecimals(amount, bank.mintDecimals), allowBorrow)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        bank: bank.publicKey,
        vault: bank.vault,
        tokenAccount: tokenAccountPk,
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
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        serumProgram: serum3ProgramId,
        serumMarketExternal: serum3MarketExternalPk,
        baseBank: baseBank.publicKey,
        quoteBank: quoteBank.publicKey,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
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
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
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
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
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
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
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
    // TODO: filter for mango account
    return await serum3MarketExternal.loadOrdersForOwner(
      this.program.provider.connection,
      group.publicKey,
    );
  }

  /// perps

  async perpCreateMarket(
    group: Group,
    oraclePk: PublicKey,
    perpMarketIndex: number,
    name: string,
    baseTokenIndex: number,
    baseTokenDecimals: number,
    quoteTokenIndex: number,
    quoteLotSize: number,
    baseLotSize: number,
    maintAssetWeight: number,
    initAssetWeight: number,
    maintLiabWeight: number,
    initLiabWeight: number,
    liquidationFee: number,
    makerFee: number,
    takerFee: number,
    minFunding: number,
    maxFunding: number,
    impactQuantity: number,
  ): Promise<TransactionSignature> {
    const bids = new Keypair();
    const asks = new Keypair();
    const eventQueue = new Keypair();

    return await this.program.methods
      .perpCreateMarket(
        perpMarketIndex,
        name,
        baseTokenIndex,
        baseTokenDecimals,
        quoteTokenIndex,
        new BN(quoteLotSize),
        new BN(baseLotSize),
        maintAssetWeight,
        initAssetWeight,
        maintLiabWeight,
        initLiabWeight,
        liquidationFee,
        makerFee,
        takerFee,
        minFunding,
        maxFunding,
        new BN(impactQuantity),
      )
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        oracle: oraclePk,
        bids: bids.publicKey,
        asks: asks.publicKey,
        eventQueue: eventQueue.publicKey,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .preInstructions([
        SystemProgram.createAccount({
          programId: this.program.programId,
          space: 8 + 90152,
          lamports:
            await this.program.provider.connection.getMinimumBalanceForRentExemption(
              90160,
            ),
          fromPubkey: (this.program.provider as AnchorProvider).wallet
            .publicKey,
          newAccountPubkey: bids.publicKey,
        }),
        SystemProgram.createAccount({
          programId: this.program.programId,
          space: 8 + 90152,
          lamports:
            await this.program.provider.connection.getMinimumBalanceForRentExemption(
              90160,
            ),
          fromPubkey: (this.program.provider as AnchorProvider).wallet
            .publicKey,
          newAccountPubkey: asks.publicKey,
        }),
        SystemProgram.createAccount({
          programId: this.program.programId,
          space: 8 + 102424,
          lamports:
            await this.program.provider.connection.getMinimumBalanceForRentExemption(
              102432,
            ),
          fromPubkey: (this.program.provider as AnchorProvider).wallet
            .publicKey,
          newAccountPubkey: eventQueue.publicKey,
        }),
      ])
      .signers([bids, asks, eventQueue])
      .rpc();
  }

  public async perpGetMarket(
    group: Group,
    baseTokenIndex?: number,
    quoteTokenIndex?: number,
  ): Promise<PerpMarket[]> {
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
          offset: 428,
        },
      });
    }

    if (quoteTokenIndex) {
      const qbuf = Buffer.alloc(2);
      qbuf.writeUInt16LE(quoteTokenIndex);
      filters.push({
        memcmp: {
          bytes: bs58.encode(qbuf),
          offset: 430,
        },
      });
    }

    return (await this.program.account.perpMarket.all(filters)).map((tuple) =>
      PerpMarket.from(tuple.publicKey, tuple.account),
    );
  }

  async perpPlaceOrder(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketName: string,
    side: Side,
    price: number,
    quantity: number,
    maxQuoteQuantity: number,
    clientOrderId: number,
    orderType: OrderType,
    expiryTimestamp: number,
    limit: number,
  ) {
    const perpMarket = group.perpMarketsMap.get(perpMarketName)!;

    const healthRemainingAccounts: PublicKey[] =
      await this.buildHealthRemainingAccounts(group, mangoAccount);

    let [nativePrice, nativeQuantity] = perpMarket.uiToNativePriceQuantity(
      price,
      quantity,
    );

    const maxQuoteQuantityLots = maxQuoteQuantity
      ? perpMarket.uiQuoteToLots(maxQuoteQuantity)
      : I64_MAX_BN;

    await this.program.methods
      .perpPlaceOrder(
        side,
        nativePrice,
        nativeQuantity,
        maxQuoteQuantityLots,
        new BN(clientOrderId),
        orderType,
        new BN(expiryTimestamp),
        limit,
      )
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        perpMarket: perpMarket.publicKey,
        asks: perpMarket.asks,
        bids: perpMarket.bids,
        eventQueue: perpMarket.eventQueue,
        oracle: perpMarket.oracle,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .rpc();
  }

  /// margin trade (orca)

  public async marginTrade({
    group,
    mangoAccount,
    inputToken,
    amountIn,
    outputToken,
    minimumAmountOut,
  }: {
    group: Group;
    mangoAccount: MangoAccount;
    inputToken: string;
    amountIn: number;
    outputToken: string;
    minimumAmountOut: number;
  }): Promise<TransactionSignature> {
    const inputBank = group.banksMap.get(inputToken);
    const outputBank = group.banksMap.get(outputToken);

    if (!inputBank || !outputBank) throw new Error('Invalid token');

    const healthRemainingAccounts: PublicKey[] =
      await this.buildHealthRemainingAccounts(group, mangoAccount, [
        inputBank,
        outputBank,
      ]);
    const parsedHealthAccounts = healthRemainingAccounts.map(
      (pk) =>
        ({
          pubkey: pk,
          isWritable:
            pk.equals(inputBank.publicKey) || pk.equals(outputBank.publicKey)
              ? true
              : false,
          isSigner: false,
        } as AccountMeta),
    );

    const targetProgramId = ORCA_TOKEN_SWAP_ID_DEVNET;

    const { instruction, signers } = await this.buildOrcaInstruction(
      targetProgramId,
      inputBank,
      outputBank,
      toU64(amountIn, 9),
      toU64(minimumAmountOut, 6),
    );
    const targetRemainingAccounts = instruction.keys;

    const withdraws: MarginTradeWithdraw[] = [
      { index: 3, amount: toU64(amountIn, 9) },
    ];
    const cpiData = instruction.data;

    return await this.program.methods
      .marginTrade(new BN(parsedHealthAccounts.length), withdraws, cpiData)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .remainingAccounts([
        ...parsedHealthAccounts,
        {
          pubkey: targetProgramId,
          isWritable: false,
          isSigner: false,
        } as AccountMeta,
        ...targetRemainingAccounts,
      ])
      .signers(signers)
      .rpc({ skipPreflight: true });
  }

  /// liquidations

  // TODO
  // async liqTokenWithToken(
  //   assetTokenIndex: number,
  //   liabTokenIndex: number,
  //   maxLiabTransfer: number,
  // ): Promise<TransactionSignature> {
  //   return await this.program.methods
  //     .liqTokenWithToken(assetTokenIndex, liabTokenIndex, {
  //       val: I80F48.fromNumber(maxLiabTransfer).getData(),
  //     })
  //     .rpc();
  // }

  /// static

  static connect(provider?: Provider, devnet?: boolean): MangoClient {
    // TODO: use IDL on chain or in repository? decide...
    // Alternatively we could fetch IDL from chain.
    // const idl = await Program.fetchIdl(MANGO_V4_ID, provider);
    let idl = IDL;

    return new MangoClient(
      new Program<MangoV4>(idl as MangoV4, MANGO_V4_ID, provider),
      devnet,
    );
  }

  /// private

  private async buildHealthRemainingAccounts(
    group: Group,
    mangoAccount: MangoAccount,
    banks?: Bank[] /** TODO for serum3PlaceOrder we are just ingoring this atm */,
  ) {
    const healthRemainingAccounts: PublicKey[] = [];

    const tokenIndices = mangoAccount.tokens
      .filter((token) => token.tokenIndex !== 65535)
      .map((token) => token.tokenIndex);

    if (banks?.length) {
      for (const bank of banks) {
        tokenIndices.push(bank.tokenIndex);
      }
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
    healthRemainingAccounts.push(
      ...mangoAccount.perps
        .filter((perp) => perp.marketIndex !== 65535)
        .map(
          (perp) =>
            Array.from(group.perpMarketsMap.values()).filter(
              (perpMarket) => perpMarket.perpMarketIndex === perp.marketIndex,
            )[0].publicKey,
        ),
    );

    return healthRemainingAccounts;
  }

  /*
    Orca ix references:
      swap fn: https://github.com/orca-so/typescript-sdk/blob/main/src/model/orca/pool/orca-pool.ts#L162
      swap ix: https://github.com/orca-so/typescript-sdk/blob/main/src/public/utils/web3/instructions/pool-instructions.ts#L41
  */
  private async buildOrcaInstruction(
    orcaTokenSwapId: PublicKey,
    inputBank: Bank,
    outputBank: Bank,
    amountInU64: BN,
    minimumAmountOutU64: BN,
  ) {
    const poolParams = orcaDevnetPoolConfigs[OrcaDevnetPoolConfig.ORCA_SOL];

    const [authorityForPoolAddress] = await PublicKey.findProgramAddress(
      [poolParams.address.toBuffer()],
      orcaTokenSwapId,
    );

    const instruction = TokenSwap.swapInstruction(
      poolParams.address,
      authorityForPoolAddress,
      inputBank.publicKey, // userTransferAuthority
      inputBank.vault, // inputTokenUserAddress
      poolParams.tokens[inputBank.mint.toString()].addr, // inputToken.addr
      poolParams.tokens[outputBank.mint.toString()].addr, // outputToken.addr
      outputBank.vault, // outputTokenUserAddress
      poolParams.poolTokenMint,
      poolParams.feeAccount,
      null, // hostFeeAccount
      orcaTokenSwapId,
      TOKEN_PROGRAM_ID,
      amountInU64,
      minimumAmountOutU64,
    );

    instruction.keys[2].isSigner = false;
    instruction.keys[2].isWritable = true;

    return { instruction, signers: [] };
  }
}
