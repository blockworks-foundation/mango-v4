import { AnchorProvider, BN, Program, Provider } from '@project-serum/anchor';
import { getFeeRates, getFeeTier } from '@project-serum/serum';
import { Order } from '@project-serum/serum/lib/market';
import {
  closeAccount,
  initializeAccount,
  WRAPPED_SOL_MINT,
} from '@project-serum/serum/lib/token-instructions';
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  Token,
  TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import {
  AccountMeta,
  Cluster,
  Keypair,
  LAMPORTS_PER_SOL,
  MemcmpFilter,
  PublicKey,
  Signer,
  SystemProgram,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SYSVAR_RENT_PUBKEY,
  Transaction,
  TransactionInstruction,
  TransactionSignature,
} from '@solana/web3.js';
import bs58 from 'bs58';
import { Bank, MintInfo } from './accounts/bank';
import { Group } from './accounts/group';
import { I80F48 } from './accounts/I80F48';
import { MangoAccount, MangoAccountData } from './accounts/mangoAccount';
import { StubOracle } from './accounts/oracle';
import { OrderType, PerpMarket, Side } from './accounts/perp';
import {
  Serum3Market,
  Serum3OrderType,
  Serum3SelfTradeBehavior,
  Serum3Side,
} from './accounts/serum3';
import { SERUM3_PROGRAM_ID } from './constants';
import { Id } from './ids';
import { IDL, MangoV4 } from './mango_v4';
import { FlashLoanWithdraw } from './types';
import {
  getAssociatedTokenAddress,
  I64_MAX_BN,
  toNativeDecimals,
  toU64,
} from './utils';
import { simulate } from './utils/anchor';

// TODO: replace ui values with native as input wherever possible
// TODO: replace token/market names with token or market indices
export class MangoClient {
  constructor(
    public program: Program<MangoV4>,
    public programId: PublicKey,
    public cluster: Cluster,
    public groupName?: string,
  ) {}

  /// public

  // Group

  public async groupCreate(
    groupNum: number,
    testing: boolean,
    insuranceMintPk: PublicKey,
  ): Promise<TransactionSignature> {
    const adminPk = (this.program.provider as AnchorProvider).wallet.publicKey;
    return await this.program.methods
      .groupCreate(groupNum, testing ? 1 : 0)
      .accounts({
        admin: adminPk,
        payer: adminPk,
        insuranceMint: insuranceMintPk,
      })
      .rpc();
  }

  public async groupClose(group: Group): Promise<TransactionSignature> {
    const adminPk = (this.program.provider as AnchorProvider).wallet.publicKey;
    return await this.program.methods
      .groupClose()
      .accounts({
        group: group.publicKey,
        admin: adminPk,
        solDestination: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .rpc();
  }

  public async getGroup(groupPk: PublicKey): Promise<Group> {
    const groupAccount = await this.program.account.group.fetch(groupPk);
    const group = Group.from(groupPk, groupAccount);
    await group.reloadAll(this);
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
          offset: 40,
        },
      });
    }

    const groups = (await this.program.account.group.all(filters)).map(
      (tuple) => Group.from(tuple.publicKey, tuple.account),
    );
    await groups[0].reloadAll(this);
    return groups[0];
  }

  // Tokens/Banks

  public async tokenRegister(
    group: Group,
    mintPk: PublicKey,
    oraclePk: PublicKey,
    oracleConfFilter: number,
    tokenIndex: number,
    name: string,
    adjustmentFactor: number,
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
      .tokenRegister(
        tokenIndex,
        new BN(0),
        name,
        {
          confFilter: {
            val: I80F48.fromNumber(oracleConfFilter).getData(),
          },
        } as any, // future: nested custom types dont typecheck, fix if possible?
        { adjustmentFactor, util0, rate0, util1, rate1, maxRate },
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

  public async tokenEdit(
    group: Group,
    tokenName: string,
    oracle: PublicKey,
    oracleConfFilter: number,
    adjustmentFactor: number,
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
    const bank = group.banksMap.get(tokenName)!;
    const mintInfo = group.mintInfosMap.get(bank.tokenIndex)!;

    return await this.program.methods
      .tokenEdit(
        new BN(0),
        oracle,
        {
          confFilter: {
            val: I80F48.fromNumber(oracleConfFilter).getData(),
          },
        } as any, // future: nested custom types dont typecheck, fix if possible?
        { adjustmentFactor, util0, rate0, util1, rate1, maxRate },
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
        mintInfo: mintInfo.publicKey,
      })
      .remainingAccounts([
        {
          pubkey: bank.publicKey,
          isWritable: true,
          isSigner: false,
        } as AccountMeta,
      ])
      .rpc({ skipPreflight: true });
  }

  public async tokenDeregister(
    group: Group,
    tokenName: string,
  ): Promise<TransactionSignature> {
    const bank = group.banksMap.get(tokenName)!;
    const adminPk = (this.program.provider as AnchorProvider).wallet.publicKey;

    const dustVaultPk = await getAssociatedTokenAddress(bank.mint, adminPk);
    const ai = await this.program.provider.connection.getAccountInfo(
      dustVaultPk,
    );
    if (!ai) {
      const tx = new Transaction();
      tx.add(
        Token.createAssociatedTokenAccountInstruction(
          ASSOCIATED_TOKEN_PROGRAM_ID,
          TOKEN_PROGRAM_ID,
          bank.mint,
          dustVaultPk,
          adminPk,
          adminPk,
        ),
      );
      await this.program.provider.sendAndConfirm(tx);
    }

    return await this.program.methods
      .tokenDeregister(bank.tokenIndex)
      .accounts({
        group: group.publicKey,
        admin: adminPk,
        mintInfo: group.mintInfosMap.get(bank.tokenIndex)?.publicKey,
        dustVault: dustVaultPk,
        solDestination: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .remainingAccounts(
        [bank.publicKey, bank.vault].map(
          (pk) =>
            ({ pubkey: pk, isWritable: true, isSigner: false } as AccountMeta),
        ),
      )
      .rpc();
  }

  public async getBanksForGroup(group: Group): Promise<Bank[]> {
    return (
      await this.program.account.bank.all([
        {
          memcmp: {
            bytes: group.publicKey.toBase58(),
            offset: 8,
          },
        },
      ])
    ).map((tuple) => Bank.from(tuple.publicKey, tuple.account));
  }

  public async getMintInfosForGroup(group: Group): Promise<MintInfo[]> {
    return (
      await this.program.account.mintInfo.all([
        {
          memcmp: {
            bytes: group.publicKey.toBase58(),
            offset: 8,
          },
        },
      ])
    ).map((tuple) => {
      return MintInfo.from(tuple.publicKey, tuple.account);
    });
  }

  public async getMintInfoForTokenIndex(
    group: Group,
    tokenIndex: number,
  ): Promise<MintInfo[]> {
    const tokenIndexBuf = Buffer.alloc(2);
    tokenIndexBuf.writeUInt16LE(tokenIndex);
    return (
      await this.program.account.mintInfo.all([
        {
          memcmp: {
            bytes: group.publicKey.toBase58(),
            offset: 8,
          },
        },
        {
          memcmp: {
            bytes: bs58.encode(tokenIndexBuf),
            offset: 40,
          },
        },
      ])
    ).map((tuple) => {
      return MintInfo.from(tuple.publicKey, tuple.account);
    });
  }

  // Stub Oracle

  public async stubOracleCreate(
    group: Group,
    mintPk: PublicKey,
    price: number,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .stubOracleCreate({ val: I80F48.fromNumber(price).getData() })
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        tokenMint: mintPk,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .rpc();
  }

  public async stubOracleClose(
    group: Group,
    oracle: PublicKey,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .stubOracleClose()
      .accounts({
        group: group.publicKey,
        oracle: oracle,
        solDestination: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .rpc();
  }

  public async stubOracleSet(
    group: Group,
    oraclePk: PublicKey,
    price: number,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .stubOracleSet({ val: I80F48.fromNumber(price).getData() })
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
    mintPk?: PublicKey,
  ): Promise<StubOracle[]> {
    const filters = [
      {
        memcmp: {
          bytes: group.publicKey.toBase58(),
          offset: 8,
        },
      },
    ];

    if (mintPk) {
      filters.push({
        memcmp: {
          bytes: mintPk.toBase58(),
          offset: 40,
        },
      });
    }

    return (await this.program.account.stubOracle.all(filters)).map((pa) =>
      StubOracle.from(pa.publicKey, pa.account),
    );
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
      .accountCreate(accountNumber, name ?? '')
      .accounts({
        group: group.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .rpc();
  }

  public async expandMangoAccount(
    group: Group,
    account: MangoAccount,
    account2: PublicKey,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .accountExpand()
      .accounts({
        group: group.publicKey,
        account: account.publicKey,
        account2,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .rpc();
  }

  public async editMangoAccount(
    group: Group,
    mangoAccount: MangoAccount,
    name?: string,
    delegate?: PublicKey,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .accountEdit(name, delegate)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
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
            offset: 8,
          },
        },
        {
          memcmp: {
            bytes: ownerPk.toBase58(),
            offset: 40,
          },
        },
      ])
    ).map((pa) => {
      return MangoAccount.from(pa.publicKey, pa.account);
    });
  }

  public async getCFTestccountForOwner(ownerPk: PublicKey): Promise<any> {
    return await this.program.account.mangoAccount2.all([
      {
        memcmp: {
          bytes: ownerPk.toBase58(),
          offset: 8,
        },
      },
    ]);
  }

  public async closeMangoAccount(
    group: Group,
    mangoAccount: MangoAccount,
  ): Promise<TransactionSignature> {
    return await this.program.methods
      .accountClose()
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        solDestination: mangoAccount.owner,
      })
      .rpc();
  }

  public async computeAccountData(
    group: Group,
    mangoAccount: MangoAccount,
  ): Promise<MangoAccountData> {
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(group, mangoAccount);

    // Use our custom simulate fn in utils/anchor.ts so signing the tx is not required
    this.program.provider.simulate = simulate;

    const res = await this.program.methods
      .computeAccountData()
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .simulate();

    return MangoAccountData.from(
      res.events.find((event) => (event.name = 'MangoAccountData')).data as any,
    );
  }

  public async tokenDeposit(
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
      this.buildHealthRemainingAccounts(group, mangoAccount, [bank]);

    return await this.program.methods
      .tokenDeposit(toNativeDecimals(amount, bank.mintDecimals))
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

  /**
   * @deprecated
   */
  public async tokenWithdraw(
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
      this.buildHealthRemainingAccounts(group, mangoAccount, [bank]);

    return await this.program.methods
      .tokenWithdraw(toNativeDecimals(amount, bank.mintDecimals), allowBorrow)
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
      .rpc({ skipPreflight: true });
  }

  public async tokenWithdraw2(
    group: Group,
    mangoAccount: MangoAccount,
    tokenName: string,
    nativeAmount: number,
    allowBorrow: boolean,
  ) {
    const bank = group.banksMap.get(tokenName)!;

    const tokenAccountPk = await getAssociatedTokenAddress(
      bank.mint,
      mangoAccount.owner,
    );

    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(group, mangoAccount, [bank]);

    return await this.program.methods
      .tokenWithdraw(new BN(nativeAmount), allowBorrow)
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
      .rpc({ skipPreflight: true });
  }

  // Serum

  public async serum3RegisterMarket(
    group: Group,
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
        serumProgram: SERUM3_PROGRAM_ID[this.cluster],
        serumMarketExternal: serum3MarketExternalPk,
        baseBank: baseBank.publicKey,
        quoteBank: quoteBank.publicKey,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .rpc();
  }

  public async serum3deregisterMarket(
    group: Group,
    serum3MarketName: string,
  ): Promise<TransactionSignature> {
    const serum3Market = group.serum3MarketsMap.get(serum3MarketName)!;

    return await this.program.methods
      .serum3DeregisterMarket()
      .accounts({
        group: group.publicKey,
        serumMarket: serum3Market.publicKey,
        solDestination: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .rpc();
  }

  public async serum3GetMarkets(
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
          offset: 8,
        },
      },
    ];

    if (baseTokenIndex) {
      const bbuf = Buffer.alloc(2);
      bbuf.writeUInt16LE(baseTokenIndex);
      filters.push({
        memcmp: {
          bytes: bs58.encode(bbuf),
          offset: 40,
        },
      });
    }

    if (quoteTokenIndex) {
      const qbuf = Buffer.alloc(2);
      qbuf.writeUInt16LE(quoteTokenIndex);
      filters.push({
        memcmp: {
          bytes: bs58.encode(qbuf),
          offset: 42,
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

  public async serum3CloseOpenOrders(
    group: Group,
    mangoAccount: MangoAccount,
    serum3MarketName: string,
  ): Promise<TransactionSignature> {
    const serum3Market = group.serum3MarketsMap.get(serum3MarketName)!;

    let openOrders = mangoAccount.serum3.find(
      (account) => account.marketIndex === serum3Market.marketIndex,
    )?.openOrders;

    return await this.program.methods
      .serum3CloseOpenOrders()
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        serumMarket: serum3Market.publicKey,
        serumProgram: serum3Market.serumProgram,
        serumMarketExternal: serum3Market.serumMarketExternal,
        openOrders,
        solDestination: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .rpc();
  }

  public async serum3PlaceOrder(
    group: Group,
    mangoAccount: MangoAccount,
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

    const serum3MarketExternal =
      group.serum3MarketExternalsMap.get(serum3MarketName)!;

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
        SERUM3_PROGRAM_ID[this.cluster],
      );

    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(group, mangoAccount);

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
        serumProgram: SERUM3_PROGRAM_ID[this.cluster],
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

  async serum3CancelAllorders(
    group: Group,
    mangoAccount: MangoAccount,
    serum3MarketName: string,
    limit: number,
  ) {
    const serum3Market = group.serum3MarketsMap.get(serum3MarketName)!;

    const serum3MarketExternal =
      group.serum3MarketExternalsMap.get(serum3MarketName)!;

    return await this.program.methods
      .serum3CancelAllOrders(limit)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        openOrders: mangoAccount.findSerum3Account(serum3Market.marketIndex)
          ?.openOrders,
        serumMarket: serum3Market.publicKey,
        serumProgram: SERUM3_PROGRAM_ID[this.cluster],
        serumMarketExternal: serum3Market.serumMarketExternal,
        marketBids: serum3MarketExternal.bidsAddress,
        marketAsks: serum3MarketExternal.asksAddress,
        marketEventQueue: serum3MarketExternal.decoded.eventQueue,
      })
      .rpc();
  }

  async serum3SettleFunds(
    group: Group,
    mangoAccount: MangoAccount,
    serum3MarketName: string,
  ): Promise<TransactionSignature> {
    const serum3Market = group.serum3MarketsMap.get(serum3MarketName)!;

    const serum3MarketExternal =
      group.serum3MarketExternalsMap.get(serum3MarketName)!;

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
        SERUM3_PROGRAM_ID[this.cluster],
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
        serumProgram: SERUM3_PROGRAM_ID[this.cluster],
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
    serum3MarketName: string,
    side: Serum3Side,
    orderId: BN,
  ): Promise<TransactionSignature> {
    const serum3Market = group.serum3MarketsMap.get(serum3MarketName)!;

    const serum3MarketExternal =
      group.serum3MarketExternalsMap.get(serum3MarketName)!;

    return await this.program.methods
      .serum3CancelOrder(side, orderId)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        openOrders: mangoAccount.findSerum3Account(serum3Market.marketIndex)
          ?.openOrders,
        serumMarket: serum3Market.publicKey,
        serumProgram: SERUM3_PROGRAM_ID[this.cluster],
        serumMarketExternal: serum3Market.serumMarketExternal,
        marketBids: serum3MarketExternal.bidsAddress,
        marketAsks: serum3MarketExternal.asksAddress,
        marketEventQueue: serum3MarketExternal.decoded.eventQueue,
      })
      .rpc();
  }

  async getSerum3Orders(
    group: Group,
    serum3MarketName: string,
  ): Promise<Order[]> {
    const serum3MarketExternal =
      group.serum3MarketExternalsMap.get(serum3MarketName)!;

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
    oracleConfFilter: number,
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
        {
          confFilter: {
            val: I80F48.fromNumber(oracleConfFilter).getData(),
          },
        } as any, // future: nested custom types dont typecheck, fix if possible?
        baseTokenIndex,
        baseTokenDecimals,
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
        // TODO: try to pick up sizes of bookside and eventqueue from IDL, so we can stay in sync with program
        SystemProgram.createAccount({
          programId: this.program.programId,
          space: 8 + 90136,
          lamports:
            await this.program.provider.connection.getMinimumBalanceForRentExemption(
              90144,
            ),
          fromPubkey: (this.program.provider as AnchorProvider).wallet
            .publicKey,
          newAccountPubkey: bids.publicKey,
        }),
        SystemProgram.createAccount({
          programId: this.program.programId,
          space: 8 + 90136,
          lamports:
            await this.program.provider.connection.getMinimumBalanceForRentExemption(
              90144,
            ),
          fromPubkey: (this.program.provider as AnchorProvider).wallet
            .publicKey,
          newAccountPubkey: asks.publicKey,
        }),
        SystemProgram.createAccount({
          programId: this.program.programId,
          space: 8 + 102416,
          lamports:
            await this.program.provider.connection.getMinimumBalanceForRentExemption(
              102424,
            ),
          fromPubkey: (this.program.provider as AnchorProvider).wallet
            .publicKey,
          newAccountPubkey: eventQueue.publicKey,
        }),
      ])
      .signers([bids, asks, eventQueue])
      .rpc();
  }

  async perpEditMarket(
    group: Group,
    perpMarketName: string,
    oracle: PublicKey,
    oracleConfFilter: number,
    baseTokenIndex: number,
    baseTokenDecimals: number,
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
    const perpMarket = group.perpMarketsMap.get(perpMarketName)!;

    return await this.program.methods
      .perpEditMarket(
        oracle,
        {
          confFilter: {
            val: I80F48.fromNumber(oracleConfFilter).getData(),
          },
        } as any, // future: nested custom types dont typecheck, fix if possible?
        baseTokenIndex,
        baseTokenDecimals,
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
        perpMarket: perpMarket.publicKey,
      })
      .rpc();
  }

  async perpCloseMarket(
    group: Group,
    perpMarketName: string,
  ): Promise<TransactionSignature> {
    const perpMarket = group.perpMarketsMap.get(perpMarketName)!;

    return await this.program.methods
      .perpCloseMarket()
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        perpMarket: perpMarket.publicKey,
        asks: perpMarket.asks,
        bids: perpMarket.bids,
        eventQueue: perpMarket.eventQueue,
        solDestination: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .rpc();
  }

  public async perpGetMarkets(
    group: Group,
    baseTokenIndex?: number,
  ): Promise<PerpMarket[]> {
    const bumpfbuf = Buffer.alloc(1);
    bumpfbuf.writeUInt8(255);

    const filters: MemcmpFilter[] = [
      {
        memcmp: {
          bytes: group.publicKey.toBase58(),
          offset: 8,
        },
      },
    ];

    if (baseTokenIndex) {
      const bbuf = Buffer.alloc(2);
      bbuf.writeUInt16LE(baseTokenIndex);
      filters.push({
        memcmp: {
          bytes: bs58.encode(bbuf),
          offset: 40,
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
      this.buildHealthRemainingAccounts(group, mangoAccount);

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

  public async marginTrade({
    group,
    mangoAccount,
    inputToken,
    amountIn,
    outputToken,
    userDefinedInstructions,
  }: {
    group: Group;
    mangoAccount: MangoAccount;
    inputToken: string;
    amountIn: number;
    outputToken: string;
    userDefinedInstructions: TransactionInstruction[];
  }): Promise<TransactionSignature> {
    const inputBank = group.banksMap.get(inputToken);
    const outputBank = group.banksMap.get(outputToken);

    if (!inputBank || !outputBank) throw new Error('Invalid token');

    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(group, mangoAccount, [
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

    /*
     * Find or create associated token accounts
     */
    let inputTokenAccountPk = await getAssociatedTokenAddress(
      inputBank.mint,
      mangoAccount.owner,
    );
    const inputTokenAccExists =
      await this.program.provider.connection.getAccountInfo(
        inputTokenAccountPk,
      );
    let preInstructions = [];
    if (!inputTokenAccExists) {
      preInstructions.push(
        Token.createAssociatedTokenAccountInstruction(
          mangoAccount.owner,
          inputTokenAccountPk,
          mangoAccount.owner,
          inputBank.mint,
          TOKEN_PROGRAM_ID,
          ASSOCIATED_TOKEN_PROGRAM_ID,
        ),
      );
    }

    let outputTokenAccountPk = await getAssociatedTokenAddress(
      outputBank.mint,
      mangoAccount.owner,
    );
    const outputTokenAccExists =
      await this.program.provider.connection.getAccountInfo(
        outputTokenAccountPk,
      );
    if (!outputTokenAccExists) {
      preInstructions.push(
        Token.createAssociatedTokenAccountInstruction(
          mangoAccount.owner,
          outputTokenAccountPk,
          mangoAccount.owner,
          outputBank.mint,
          TOKEN_PROGRAM_ID,
          ASSOCIATED_TOKEN_PROGRAM_ID,
        ),
      );
    }

    /*
     * Transfer input token to users wallet, then concat the passed in instructions
     */
    const nativeInputAmount = toU64(
      amountIn,
      inputBank.mintDecimals,
    ).toNumber();
    const instructions: TransactionInstruction[] = [];

    const transferIx = Token.createTransferInstruction(
      TOKEN_PROGRAM_ID,
      inputBank.vault,
      inputTokenAccountPk,
      inputBank.publicKey,
      [],
      nativeInputAmount,
    );
    const inputBankKey = transferIx.keys[2];
    transferIx.keys[2] = { ...inputBankKey, isWritable: true, isSigner: false };
    instructions.push(transferIx);

    instructions.concat(userDefinedInstructions);

    const transferIx2 = Token.createTransferInstruction(
      TOKEN_PROGRAM_ID,
      outputTokenAccountPk,
      outputBank.vault,
      mangoAccount.owner,
      [],
      0, // todo: use this for testing, this should be the amount to transfer back
    );
    instructions.push(transferIx2);

    /*
     * Create object of amounts that will be withdrawn from bank vaults
     */
    const targetRemainingAccounts = instructions
      .map((ix) => [
        {
          pubkey: ix.programId,
          isWritable: false,
          isSigner: false,
        } as AccountMeta,
        ...ix.keys,
      ])
      .flat();

    const vaultIndex = targetRemainingAccounts
      .map((x) => x.pubkey.toString())
      .lastIndexOf(inputBank.vault.toString());

    const withdraws: FlashLoanWithdraw[] = [
      {
        index: vaultIndex,
        amount: toU64(amountIn, inputBank.mintDecimals),
      },
    ];

    /*
     * Build cpi data objects for instructions
     */
    let cpiDatas = [];
    for (const [index, ix] of instructions.entries()) {
      if (index === 0) {
        cpiDatas.push({
          accountStart: new BN(parsedHealthAccounts.length),
          data: ix.data,
        });
      } else {
        cpiDatas.push({
          accountStart: cpiDatas[index - 1].accountStart.add(
            new BN(instructions[index - 1].keys.length + 1),
          ),
          data: ix.data,
        });
      }
    }

    if (preInstructions.length) {
      const tx = new Transaction();
      for (const ix of preInstructions) {
        tx.add(ix);
      }

      await this.program.provider.sendAndConfirm(tx);
    }

    return await this.program.methods
      .flashLoan(withdraws, cpiDatas)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .remainingAccounts([...parsedHealthAccounts, ...targetRemainingAccounts])
      .rpc({ skipPreflight: true });
  }

  public async marginTrade3({
    group,
    mangoAccount,
    inputToken,
    amountIn,
    outputToken,
    userDefinedInstructions,
  }: {
    group: Group;
    mangoAccount: MangoAccount;
    inputToken: string;
    amountIn: number;
    outputToken: string;
    userDefinedInstructions: TransactionInstruction[];
  }): Promise<TransactionSignature> {
    const inputBank = group.banksMap.get(inputToken);
    const outputBank = group.banksMap.get(outputToken);

    if (!inputBank || !outputBank) throw new Error('Invalid token');

    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(group, mangoAccount, [
        inputBank,
        outputBank,
      ]);
    const parsedHealthAccounts = healthRemainingAccounts.map(
      (pk) =>
        ({
          pubkey: pk,
          isWritable: false,
          isSigner: false,
        } as AccountMeta),
    );
    console.log('1');

    /*
     * Find or create associated token accounts
     */
    let inputTokenAccountPk = await getAssociatedTokenAddress(
      inputBank.mint,
      mangoAccount.owner,
    );
    const inputTokenAccExists =
      await this.program.provider.connection.getAccountInfo(
        inputTokenAccountPk,
      );
    let preInstructions = [];
    if (!inputTokenAccExists) {
      preInstructions.push(
        Token.createAssociatedTokenAccountInstruction(
          mangoAccount.owner,
          inputTokenAccountPk,
          mangoAccount.owner,
          inputBank.mint,
          TOKEN_PROGRAM_ID,
          ASSOCIATED_TOKEN_PROGRAM_ID,
        ),
      );
    }

    let outputTokenAccountPk = await getAssociatedTokenAddress(
      outputBank.mint,
      mangoAccount.owner,
    );
    const outputTokenAccExists =
      await this.program.provider.connection.getAccountInfo(
        outputTokenAccountPk,
      );
    if (!outputTokenAccExists) {
      preInstructions.push(
        Token.createAssociatedTokenAccountInstruction(
          mangoAccount.owner,
          outputTokenAccountPk,
          mangoAccount.owner,
          outputBank.mint,
          TOKEN_PROGRAM_ID,
          ASSOCIATED_TOKEN_PROGRAM_ID,
        ),
      );
    }
    console.log('2');

    if (preInstructions.length) {
      const tx = new Transaction();
      for (const ix of preInstructions) {
        tx.add(ix);
      }
      console.log('preInstructions', preInstructions);

      await this.program.provider.sendAndConfirm(tx);
    }
    console.log('3');

    const inputBankAccount = {
      pubkey: inputBank.publicKey,
      isWritable: true,
      isSigner: false,
    };
    const outputBankAccount = {
      pubkey: outputBank.publicKey,
      isWritable: true,
      isSigner: false,
    };
    const inputBankVault = {
      pubkey: inputBank.vault,
      isWritable: true,
      isSigner: false,
    };
    const outputBankVault = {
      pubkey: outputBank.vault,
      isWritable: true,
      isSigner: false,
    };
    const inputATA = {
      pubkey: inputTokenAccountPk,
      isWritable: true,
      isSigner: false,
    };
    const outputATA = {
      pubkey: outputTokenAccountPk,
      isWritable: false,
      isSigner: false,
    };

    const flashLoanEndIx = await this.program.methods
      .flashLoan3End()
      .accounts({
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .remainingAccounts([
        ...parsedHealthAccounts,
        inputBankVault,
        outputBankVault,
        inputATA,
        {
          isWritable: true,
          pubkey: outputTokenAccountPk,
          isSigner: false,
        },
      ])
      .instruction();
    console.log('4');

    // userDefinedInstructions.push(flashLoanEndIx);

    const flashLoanBeginIx = await this.program.methods
      .flashLoan3Begin([
        toNativeDecimals(amountIn, inputBank.mintDecimals),
        new BN(
          0,
        ) /* we don't care about borrowing the target amount, this is just a dummy */,
      ])
      .accounts({
        group: group.publicKey,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
      })
      .remainingAccounts([
        inputBankAccount,
        outputBankAccount,
        inputBankVault,
        outputBankVault,
        inputATA,
        outputATA,
      ])
      .instruction();

    const tx = new Transaction();
    tx.add(flashLoanBeginIx);
    for (const i of userDefinedInstructions) {
      tx.add(i);
    }
    tx.add(flashLoanEndIx);
    return this.program.provider.sendAndConfirm(tx);
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

  static connect(
    provider: Provider,
    cluster: Cluster,
    programId: PublicKey,
  ): MangoClient {
    // TODO: use IDL on chain or in repository? decide...
    // Alternatively we could fetch IDL from chain.
    // const idl = await Program.fetchIdl(MANGO_V4_ID, provider);
    let idl = IDL;

    return new MangoClient(
      new Program<MangoV4>(idl as MangoV4, programId, provider),
      programId,
      cluster,
    );
  }

  static connectForGroupName(
    provider: Provider,
    groupName: string,
  ): MangoClient {
    // TODO: use IDL on chain or in repository? decide...
    // Alternatively we could fetch IDL from chain.
    // const idl = await Program.fetchIdl(MANGO_V4_ID, provider);
    let idl = IDL;

    const id = Id.fromIds(groupName);

    return new MangoClient(
      new Program<MangoV4>(
        idl as MangoV4,
        new PublicKey(id.mangoProgramId),
        provider,
      ),
      new PublicKey(id.mangoProgramId),
      id.cluster,
      groupName,
    );
  }

  /// private

  public buildHealthRemainingAccounts(
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

    const mintInfos = [...new Set(tokenIndices)].map(
      (tokenIndex) => group.mintInfosMap.get(tokenIndex)!,
    );
    healthRemainingAccounts.push(
      ...mintInfos.map((mintInfo) => mintInfo.firstBank()),
    );
    healthRemainingAccounts.push(
      ...mintInfos.map((mintInfo) => mintInfo.oracle),
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
}
