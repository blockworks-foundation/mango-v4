import { AnchorProvider, BN, Program, Wallet } from '@coral-xyz/anchor';
import { OpenOrders, decodeEventQueue } from '@project-serum/serum';
import {
  createAccount,
  createCloseAccountInstruction,
  createInitializeAccount3Instruction,
  unpackAccount,
} from '@solana/spl-token';
import {
  AccountInfo,
  AccountMeta,
  AddressLookupTableAccount,
  Cluster,
  Commitment,
  ComputeBudgetProgram,
  Connection,
  Keypair,
  MemcmpFilter,
  PublicKey,
  RecentPrioritizationFees,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SYSVAR_RENT_PUBKEY,
  Signer,
  SystemProgram,
  TransactionInstruction,
} from '@solana/web3.js';
import bs58 from 'bs58';
import copy from 'fast-copy';
import chunk from 'lodash/chunk';
import groupBy from 'lodash/groupBy';
import mapValues from 'lodash/mapValues';
import maxBy from 'lodash/maxBy';
import uniq from 'lodash/uniq';
import { Bank, MintInfo, TokenIndex } from './accounts/bank';
import { Group } from './accounts/group';
import {
  MangoAccount,
  PerpPosition,
  Serum3Orders,
  TokenConditionalSwap,
  TokenConditionalSwapDisplayPriceStyle,
  TokenConditionalSwapIntention,
  TokenPosition,
} from './accounts/mangoAccount';
import { StubOracle } from './accounts/oracle';
import {
  FillEvent,
  OutEvent,
  PerpEventQueue,
  PerpMarket,
  PerpMarketIndex,
  PerpOrderSide,
  PerpOrderType,
  PerpSelfTradeBehavior,
} from './accounts/perp';
import {
  MarketIndex,
  Serum3Market,
  Serum3OrderType,
  Serum3SelfTradeBehavior,
  Serum3Side,
  generateSerum3MarketExternalVaultSignerAddress,
} from './accounts/serum3';
import {
  IxGateParams,
  TokenEditParams,
  TokenRegisterParams,
  buildIxGate,
} from './clientIxParamBuilder';
import {
  MANGO_V4_ID,
  MAX_RECENT_PRIORITY_FEE_ACCOUNTS,
  OPENBOOK_PROGRAM_ID,
  RUST_U64_MAX,
} from './constants';
import { Id } from './ids';
import { IDL, MangoV4 } from './mango_v4';
import { I80F48 } from './numbers/I80F48';
import { FlashLoanType, OracleConfigParams } from './types';
import {
  I64_MAX_BN,
  U64_MAX_BN,
  createAssociatedTokenAccountIdempotentInstruction,
  getAssociatedTokenAddress,
  toNative,
  toNativeSellPerBuyTokenPrice,
} from './utils';
import {
  LatestBlockhash,
  MangoSignatureStatus,
  SendTransactionOpts,
  sendTransaction,
} from './utils/rpc';
import { NATIVE_MINT, TOKEN_PROGRAM_ID } from './utils/spl';

export const DEFAULT_TOKEN_CONDITIONAL_SWAP_COUNT = 8;
export const PERP_SETTLE_PNL_CU_LIMIT = 400000;
export const PERP_SETTLE_FEES_CU_LIMIT = 20000;
export const SERUM_SETTLE_FUNDS_CU_LIMIT = 65000;

export enum AccountRetriever {
  Scanning,
  Fixed,
}

export type IdsSource = 'api' | 'static' | 'get-program-accounts';

export type MangoClientOptions = {
  idsSource?: IdsSource;
  postSendTxCallback?: (callbackOpts: TxCallbackOptions) => void;
  postTxConfirmationCallback?: (callbackOpts: TxCallbackOptions) => void;
  prioritizationFee?: number;
  estimateFee?: boolean;
  txConfirmationCommitment?: Commitment;
  openbookFeesToDao?: boolean;
  prependedGlobalAdditionalInstructions?: TransactionInstruction[];
  multipleConnections?: Connection[];
};

export type TxCallbackOptions = {
  txid: string;
  txSignatureBlockHash: LatestBlockhash;
};

export class MangoClient {
  private idsSource: IdsSource;
  private postSendTxCallback?: (callbackOpts: TxCallbackOptions) => void;
  postTxConfirmationCallback?: (callbackOpts: TxCallbackOptions) => void;
  private prioritizationFee: number;
  private estimateFee: boolean;
  private txConfirmationCommitment: Commitment;
  private openbookFeesToDao: boolean;
  private prependedGlobalAdditionalInstructions: TransactionInstruction[] = [];
  multipleConnections: Connection[] = [];

  constructor(
    public program: Program<MangoV4>,
    public programId: PublicKey,
    public cluster: Cluster,
    public opts: MangoClientOptions = {},
  ) {
    this.idsSource = opts?.idsSource || 'get-program-accounts';
    this.prioritizationFee = opts?.prioritizationFee || 0;
    this.estimateFee = opts?.estimateFee || false;
    this.postSendTxCallback = opts?.postSendTxCallback;
    this.postTxConfirmationCallback = opts?.postTxConfirmationCallback;
    this.openbookFeesToDao = opts?.openbookFeesToDao ?? true;
    this.prependedGlobalAdditionalInstructions =
      opts.prependedGlobalAdditionalInstructions ?? [];
    this.txConfirmationCommitment =
      opts?.txConfirmationCommitment ??
      (program.provider as AnchorProvider).opts.commitment ??
      'processed';
    // TODO: evil side effect, but limited backtraces are a nightmare
    Error.stackTraceLimit = 1000;
    this.multipleConnections = opts?.multipleConnections ?? [];
  }

  /// Convenience accessors
  public get connection(): Connection {
    return this.program.provider.connection;
  }

  public get walletPk(): PublicKey {
    return (this.program.provider as AnchorProvider).wallet.publicKey;
  }

  public async sendAndConfirmTransaction(
    ixs: TransactionInstruction[],
    opts?: SendTransactionOpts,
  ): Promise<MangoSignatureStatus>;

  public async sendAndConfirmTransaction(
    ixs: TransactionInstruction[],
    opts?: { confirmInBackground: true } & SendTransactionOpts,
  ): Promise<MangoSignatureStatus>;

  /// Transactions
  public async sendAndConfirmTransaction(
    ixs: TransactionInstruction[],
    opts: SendTransactionOpts = {},
  ): Promise<MangoSignatureStatus> {
    let prioritizationFee: number;
    if (opts.prioritizationFee) {
      prioritizationFee = opts.prioritizationFee;
    } else if (this.estimateFee || opts.estimateFee) {
      prioritizationFee = await this.estimatePrioritizationFee(ixs);
    } else {
      prioritizationFee = this.prioritizationFee;
    }

    const status = await sendTransaction(
      this.program.provider as AnchorProvider,
      [...this.prependedGlobalAdditionalInstructions, ...ixs],
      opts.alts ?? [],
      {
        postSendTxCallback: this.postSendTxCallback,
        postTxConfirmationCallback: this.postTxConfirmationCallback,
        prioritizationFee,
        txConfirmationCommitment: this.txConfirmationCommitment,
        multipleConnections: this.multipleConnections,
        ...opts,
      },
    );
    return status;
  }

  public async sendAndConfirmTransactionForGroup(
    group: Group,
    ixs: TransactionInstruction[],
    opts?: SendTransactionOpts,
  ): Promise<MangoSignatureStatus>;

  public async sendAndConfirmTransactionForGroup(
    group: Group,
    ixs: TransactionInstruction[],
    opts?: { confirmInBackground: true } & SendTransactionOpts,
  ): Promise<MangoSignatureStatus>;

  public async sendAndConfirmTransactionForGroup(
    group: Group,
    ixs: TransactionInstruction[],
    opts: SendTransactionOpts = {},
  ): Promise<MangoSignatureStatus> {
    return await this.sendAndConfirmTransaction(ixs, {
      alts: group.addressLookupTablesList,
      ...opts,
    });
  }

  public async adminTokenWithdrawFees(
    group: Group,
    bank: Bank,
    tokenAccountPk: PublicKey,
  ): Promise<MangoSignatureStatus> {
    const admin = (this.program.provider as AnchorProvider).wallet.publicKey;
    const ix = await this.program.methods
      .adminTokenWithdrawFees()
      .accounts({
        group: group.publicKey,
        bank: bank.publicKey,
        vault: bank.vault,
        tokenAccount: tokenAccountPk,
        admin,
      })
      .instruction();
    return await this.sendAndConfirmTransaction([ix]);
  }

  public async adminPerpWithdrawFees(
    group: Group,
    perpMarket: PerpMarket,
    tokenAccountPk: PublicKey,
  ): Promise<MangoSignatureStatus> {
    const bank = group.getFirstBankByTokenIndex(perpMarket.settleTokenIndex);
    const admin = (this.program.provider as AnchorProvider).wallet.publicKey;
    const ix = await this.program.methods
      .adminPerpWithdrawFees()
      .accounts({
        group: group.publicKey,
        perpMarket: perpMarket.publicKey,
        bank: bank.publicKey,
        vault: bank.vault,
        tokenAccount: tokenAccountPk,
        admin,
      })
      .instruction();
    return await this.sendAndConfirmTransaction([ix]);
  }

  // Group
  public async groupCreate(
    groupNum: number,
    testing: boolean,
    version: number,
    insuranceMintPk: PublicKey,
  ): Promise<MangoSignatureStatus> {
    const adminPk = (this.program.provider as AnchorProvider).wallet.publicKey;
    const ix = await this.program.methods
      .groupCreate(groupNum, testing ? 1 : 0, version)
      .accounts({
        creator: adminPk,
        payer: adminPk,
        insuranceMint: insuranceMintPk,
      })
      .instruction();
    return await this.sendAndConfirmTransaction([ix]);
  }

  public async groupEdit(
    group: Group,
    admin?: PublicKey,
    fastListingAdmin?: PublicKey,
    securityAdmin?: PublicKey,
    testing?: number,
    version?: number,
    depositLimitQuote?: BN,
    feesPayWithMngo?: boolean,
    feesMngoBonusRate?: number,
    feesSwapMangoAccount?: PublicKey,
    feesMngoTokenIndex?: TokenIndex,
    feesExpiryInterval?: BN,
    allowedFastListingsPerInterval?: number,
    collateralFeeInterval?: BN,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.program.methods
      .groupEdit(
        admin ?? null,
        fastListingAdmin ?? null,
        securityAdmin ?? null,
        testing ?? null,
        version ?? null,
        depositLimitQuote !== undefined ? depositLimitQuote : null,
        feesPayWithMngo ?? null,
        feesMngoBonusRate ?? null,
        feesSwapMangoAccount ?? null,
        feesMngoTokenIndex ?? null,
        feesExpiryInterval ?? null,
        allowedFastListingsPerInterval ?? null,
        collateralFeeInterval ?? null,
      )
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async ixGateSet(
    group: Group,
    ixGateParams: IxGateParams,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.program.methods
      .ixGateSet(buildIxGate(ixGateParams))
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async groupClose(group: Group): Promise<MangoSignatureStatus> {
    const adminPk = (this.program.provider as AnchorProvider).wallet.publicKey;
    const ix = await this.program.methods
      .groupClose()
      .accounts({
        group: group.publicKey,
        insuranceVault: group.insuranceVault,
        admin: adminPk,
        solDestination: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async getGroup(groupPk: PublicKey): Promise<Group> {
    const groupAccount = await this.program.account.group.fetch(groupPk);
    const group = Group.from(groupPk, groupAccount);
    await group.reloadAll(this);
    return group;
  }

  public async getGroupsForCreator(creatorPk: PublicKey): Promise<Group[]> {
    const filters: MemcmpFilter[] = [
      {
        memcmp: {
          bytes: creatorPk.toBase58(),
          offset: 8,
        },
      },
    ];

    const groups = (await this.program.account.group.all(filters)).map(
      (tuple) => Group.from(tuple.publicKey, tuple.account),
    );
    groups.forEach((group) => group.reloadAll(this));
    return groups;
  }

  public async getGroupForCreator(
    creatorPk: PublicKey,
    groupNum: number,
  ): Promise<Group> {
    const bbuf = Buffer.alloc(4);
    bbuf.writeUInt32LE(groupNum);
    const filters: MemcmpFilter[] = [
      {
        memcmp: {
          bytes: creatorPk.toBase58(),
          offset: 8,
        },
      },
      {
        memcmp: {
          bytes: bs58.encode(bbuf),
          offset: 40,
        },
      },
    ];
    const groups = (await this.program.account.group.all(filters)).map(
      (tuple) => Group.from(tuple.publicKey, tuple.account),
    );
    await groups[0].reloadAll(this);
    return groups[0];
  }

  public async getIds(groupPk: PublicKey): Promise<Id | undefined> {
    switch (this.idsSource) {
      case 'api':
        return await Id.fromApi(groupPk);
      case 'get-program-accounts':
        return undefined;
      case 'static':
        return Id.fromIdsByPk(groupPk);
    }
  }

  // Tokens/Banks

  public async tokenRegister(
    group: Group,
    mintPk: PublicKey,
    oraclePk: PublicKey,
    fallbackOraclePk: PublicKey,
    tokenIndex: number,
    name: string,
    params: TokenRegisterParams,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.program.methods
      .tokenRegister(
        tokenIndex,
        name,
        params.oracleConfig,
        params.interestRateParams,
        params.loanFeeRate,
        params.loanOriginationFeeRate,
        params.maintAssetWeight,
        params.initAssetWeight,
        params.maintLiabWeight,
        params.initLiabWeight,
        params.liquidationFee,
        params.stablePriceDelayIntervalSeconds,
        params.stablePriceDelayGrowthLimit,
        params.stablePriceGrowthLimit,
        params.minVaultToDepositsRatio,
        new BN(params.netBorrowLimitWindowSizeTs),
        new BN(params.netBorrowLimitPerWindowQuote),
        params.borrowWeightScaleStartQuote,
        params.depositWeightScaleStartQuote,
        params.reduceOnly,
        params.tokenConditionalSwapTakerFeeRate,
        params.tokenConditionalSwapMakerFeeRate,
        params.flashLoanSwapFeeRate,
        params.interestCurveScaling,
        params.interestTargetUtilization,
        params.groupInsuranceFund,
        params.depositLimit,
        params.zeroUtilRate,
        params.platformLiquidationFee,
        params.disableAssetLiquidation,
        params.collateralFeePerDay,
      )
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        mint: mintPk,
        oracle: oraclePk,
        fallbackOracle: fallbackOraclePk,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async tokenRegisterTrustless(
    group: Group,
    mintPk: PublicKey,
    oraclePk: PublicKey,
    tokenIndex: number,
    name: string,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.program.methods
      .tokenRegisterTrustless(tokenIndex, name)
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        mint: mintPk,
        oracle: oraclePk,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async tokenEdit(
    group: Group,
    mintPk: PublicKey,
    params: TokenEditParams,
  ): Promise<MangoSignatureStatus> {
    const bank = group.getFirstBankByMint(mintPk);
    const mintInfo = group.mintInfosMapByTokenIndex.get(bank.tokenIndex)!;

    const ix = await this.program.methods
      .tokenEdit(
        params.oracle,
        params.oracleConfig,
        params.groupInsuranceFund,
        params.interestRateParams,
        params.loanFeeRate,
        params.loanOriginationFeeRate,
        params.maintAssetWeight,
        params.initAssetWeight,
        params.maintLiabWeight,
        params.initLiabWeight,
        params.liquidationFee,
        params.stablePriceDelayIntervalSeconds,
        params.stablePriceDelayGrowthLimit,
        params.stablePriceGrowthLimit,
        params.minVaultToDepositsRatio,
        params.netBorrowLimitPerWindowQuote !== null
          ? new BN(params.netBorrowLimitPerWindowQuote)
          : null,
        params.netBorrowLimitWindowSizeTs !== null
          ? new BN(params.netBorrowLimitWindowSizeTs)
          : null,
        params.borrowWeightScaleStartQuote,
        params.depositWeightScaleStartQuote,
        params.resetStablePrice ?? false,
        params.resetNetBorrowLimit ?? false,
        params.reduceOnly,
        params.name,
        params.forceClose,
        params.tokenConditionalSwapTakerFeeRate,
        params.tokenConditionalSwapMakerFeeRate,
        params.flashLoanSwapFeeRate,
        params.interestCurveScaling,
        params.interestTargetUtilization,
        params.maintWeightShiftStart,
        params.maintWeightShiftEnd,
        params.maintWeightShiftAssetTarget,
        params.maintWeightShiftLiabTarget,
        params.maintWeightShiftAbort ?? false,
        params.fallbackOracle !== null, // setFallbackOracle
        params.depositLimit,
        params.zeroUtilRate,
        params.platformLiquidationFee,
        params.disableAssetLiquidation,
        params.collateralFeePerDay,
        params.forceWithdraw,
      )
      .accounts({
        group: group.publicKey,
        oracle: params.oracle ?? bank.oracle,
        fallbackOracle: params.fallbackOracle ?? bank.fallbackOracle,
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
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async tokenForceCloseBorrowsWithToken(
    group: Group,
    liqor: MangoAccount,
    liqee: MangoAccount,
    assetTokenIndex: TokenIndex,
    liabTokenIndex: TokenIndex,
    maxLiabTransfer?: number,
  ): Promise<MangoSignatureStatus> {
    const assetBank = group.getFirstBankByTokenIndex(assetTokenIndex);
    const liabBank = group.getFirstBankByTokenIndex(liabTokenIndex);
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        group,
        [liqor, liqee],
        [assetBank, liabBank],
        [],
      );
    const parsedHealthAccounts = healthRemainingAccounts.map(
      (pk) =>
        ({
          pubkey: pk,
          isWritable:
            pk.equals(assetBank.publicKey) || pk.equals(liabBank.publicKey)
              ? true
              : false,
          isSigner: false,
        } as AccountMeta),
    );
    const ix = await this.program.methods
      .tokenForceCloseBorrowsWithToken(
        assetTokenIndex,
        liabTokenIndex,
        maxLiabTransfer
          ? toNative(maxLiabTransfer, liabBank.mintDecimals)
          : U64_MAX_BN,
      )
      .accounts({
        group: group.publicKey,
        liqor: liqor.publicKey,
        liqorOwner: (this.program.provider as AnchorProvider).wallet.publicKey,
        liqee: liqee.publicKey,
      })
      .remainingAccounts(parsedHealthAccounts)
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async tokenForceWithdraw(
    group: Group,
    mangoAccount: MangoAccount,
    tokenIndex: TokenIndex,
  ): Promise<MangoSignatureStatus> {
    const bank = group.getFirstBankByTokenIndex(tokenIndex);
    if (!bank.forceWithdraw) {
      throw new Error('Bank is not in force-withdraw mode');
    }

    const ownerAtaTokenAccount = await getAssociatedTokenAddress(
      bank.mint,
      mangoAccount.owner,
      true,
    );
    let alternateOwnerTokenAccount = PublicKey.default;
    const preInstructions: TransactionInstruction[] = [];
    const postInstructions: TransactionInstruction[] = [];

    const ai = await this.connection.getAccountInfo(ownerAtaTokenAccount);

    // ensure withdraws don't fail with missing ATAs
    if (ai == null) {
      preInstructions.push(
        await createAssociatedTokenAccountIdempotentInstruction(
          (this.program.provider as AnchorProvider).wallet.publicKey,
          mangoAccount.owner,
          bank.mint,
        ),
      );

      // wsol case
      if (bank.mint.equals(NATIVE_MINT)) {
        postInstructions.push(
          createCloseAccountInstruction(
            ownerAtaTokenAccount,
            mangoAccount.owner,
            mangoAccount.owner,
          ),
        );
      }
    } else {
      const account = await unpackAccount(ownerAtaTokenAccount, ai);
      // if owner is not same as mango account's owner on the ATA (for whatever reason)
      // then create another token account
      if (!account.owner.equals(mangoAccount.owner)) {
        const kp = Keypair.generate();
        alternateOwnerTokenAccount = kp.publicKey;
        await createAccount(
          this.connection,
          (this.program.provider as AnchorProvider).wallet as any as Signer,
          bank.mint,
          mangoAccount.owner,
          kp,
        );

        // wsol case
        if (bank.mint.equals(NATIVE_MINT)) {
          postInstructions.push(
            createCloseAccountInstruction(
              alternateOwnerTokenAccount,
              mangoAccount.owner,
              mangoAccount.owner,
            ),
          );
        }
      }
    }

    const ix = await this.program.methods
      .tokenForceWithdraw()
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        bank: bank.publicKey,
        vault: bank.vault,
        oracle: bank.oracle,
        ownerAtaTokenAccount,
        alternateOwnerTokenAccount,
      })
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [
      ...preInstructions,
      ix,
      ...postInstructions,
    ]);
  }

  public async tokenDeregister(
    group: Group,
    mintPk: PublicKey,
  ): Promise<MangoSignatureStatus> {
    const bank = group.getFirstBankByMint(mintPk);
    const adminPk = (this.program.provider as AnchorProvider).wallet.publicKey;

    const dustVaultPk = await getAssociatedTokenAddress(bank.mint, adminPk);
    const ai = await this.program.provider.connection.getAccountInfo(
      dustVaultPk,
    );
    const preInstructions: TransactionInstruction[] = [];
    if (!ai) {
      preInstructions.push(
        await createAssociatedTokenAccountIdempotentInstruction(
          adminPk,
          adminPk,
          bank.mint,
        ),
      );
    }

    const ix = await this.program.methods
      .tokenDeregister()
      .accounts({
        group: group.publicKey,
        admin: adminPk,
        mintInfo: group.mintInfosMapByTokenIndex.get(bank.tokenIndex)
          ?.publicKey,
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
      .instruction();

    return await this.sendAndConfirmTransactionForGroup(group, [
      ...preInstructions,
      ix,
    ]);
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
    tokenIndex: TokenIndex,
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
  ): Promise<MangoSignatureStatus> {
    const stubOracle = Keypair.generate();
    const ix = await this.program.methods
      .stubOracleCreate({ val: I80F48.fromNumber(price).getData() })
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        oracle: stubOracle.publicKey,
        mint: mintPk,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [ix], {
      additionalSigners: [stubOracle],
    });
  }

  public async stubOracleClose(
    group: Group,
    oracle: PublicKey,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.program.methods
      .stubOracleClose()
      .accounts({
        group: group.publicKey,
        oracle: oracle,
        solDestination: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async stubOracleSet(
    group: Group,
    oraclePk: PublicKey,
    price: number,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.program.methods
      .stubOracleSet({ val: I80F48.fromNumber(price).getData() })
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        oracle: oraclePk,
      })
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
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

  public async createMangoAccount(
    group: Group,
    accountNumber?: number,
    name?: string,
    tokenCount?: number,
    serum3Count?: number,
    perpCount?: number,
    perpOoCount?: number,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.program.methods
      .accountCreate(
        accountNumber ?? 0,
        tokenCount ?? 8,
        serum3Count ?? 4,
        perpCount ?? 4,
        perpOoCount ?? 32,
        name ?? '',
      )
      .accounts({
        group: group.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .instruction();

    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async expandMangoAccount(
    group: Group,
    account: MangoAccount,
    tokenCount: number,
    serum3Count: number,
    perpCount: number,
    perpOoCount: number,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.program.methods
      .accountExpand(tokenCount, serum3Count, perpCount, perpOoCount)
      .accounts({
        group: group.publicKey,
        account: account.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async accountExpandV2(
    group: Group,
    account: MangoAccount,
    tokenCount: number,
    serum3Count: number,
    perpCount: number,
    perpOoCount: number,
    tokenConditionalSwapCount: number,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.accountExpandV2Ix(
      group,
      account,
      tokenCount,
      serum3Count,
      perpCount,
      perpOoCount,
      tokenConditionalSwapCount,
    );
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async accountExpandV2Ix(
    group: Group,
    account: MangoAccount,
    tokenCount: number,
    serum3Count: number,
    perpCount: number,
    perpOoCount: number,
    tokenConditionalSwapCount: number,
  ): Promise<TransactionInstruction> {
    return await this.program.methods
      .accountExpandV2(
        tokenCount,
        serum3Count,
        perpCount,
        perpOoCount,
        tokenConditionalSwapCount,
      )
      .accounts({
        group: group.publicKey,
        account: account.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .instruction();
  }

  public async editMangoAccount(
    group: Group,
    mangoAccount: MangoAccount,
    name?: string,
    delegate?: PublicKey,
    temporaryDelegate?: PublicKey,
    delegateExpiry?: number,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.program.methods
      .accountEdit(
        name ?? null,
        delegate ?? null,
        temporaryDelegate ?? null,
        delegateExpiry ? new BN(delegateExpiry) : null,
      )
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .instruction();

    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async toggleMangoAccountFreeze(
    group: Group,
    mangoAccount: MangoAccount,
    freeze: boolean,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.program.methods
      .accountToggleFreeze(freeze)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async getMangoAccount(
    mangoAccountPk: PublicKey,
    loadSerum3Oo = false,
  ): Promise<MangoAccount> {
    const mangoAccount = await this.getMangoAccountFromPk(mangoAccountPk);
    if (loadSerum3Oo) {
      await mangoAccount?.reloadSerum3OpenOrders(this);
    }
    return mangoAccount;
  }

  private async getMangoAccountFromPk(
    mangoAccountPk: PublicKey,
  ): Promise<MangoAccount> {
    return this.getMangoAccountFromAi(
      mangoAccountPk,
      (await this.program.provider.connection.getAccountInfo(
        mangoAccountPk,
      )) as AccountInfo<Buffer>,
    );
  }

  public getMangoAccountFromAi(
    mangoAccountPk: PublicKey,
    ai: AccountInfo<Buffer>,
  ): MangoAccount {
    const decodedMangoAccount = this.program.coder.accounts.decode(
      'mangoAccount',
      ai.data,
    );

    return MangoAccount.from(mangoAccountPk, decodedMangoAccount);
  }

  public async getMangoAccountWithSlot(
    mangoAccountPk: PublicKey,
    loadSerum3Oo = false,
  ): Promise<{ slot: number; value: MangoAccount } | undefined> {
    const resp =
      await this.program.provider.connection.getAccountInfoAndContext(
        mangoAccountPk,
      );
    if (!resp?.value) return;
    const mangoAccount = await this.getMangoAccountFromAi(
      mangoAccountPk,
      resp.value,
    );
    if (loadSerum3Oo) {
      await mangoAccount?.reloadSerum3OpenOrders(this);
    }
    return { slot: resp.context.slot, value: mangoAccount };
  }

  public async getMangoAccountForOwner(
    group: Group,
    ownerPk: PublicKey,
    accountNumber: number,
    loadSerum3Oo = false,
  ): Promise<MangoAccount | undefined> {
    const mangoAccounts = await this.getMangoAccountsForOwner(
      group,
      ownerPk,
      loadSerum3Oo,
    );
    const foundMangoAccount = mangoAccounts.find(
      (a) => a.accountNum == accountNumber,
    );

    return foundMangoAccount;
  }

  public async getMangoAccountsForOwner(
    group: Group,
    ownerPk: PublicKey,
    loadSerum3Oo = false,
  ): Promise<MangoAccount[]> {
    const discriminatorMemcmp: {
      offset: number;
      bytes: string;
    } = this.program.account.mangoAccount.coder.accounts.memcmp(
      'mangoAccount',
      undefined,
    );

    const accounts = await Promise.all(
      (
        await this.program.provider.connection.getProgramAccounts(
          this.programId,
          {
            filters: [
              {
                memcmp: {
                  bytes: discriminatorMemcmp.bytes,
                  offset: discriminatorMemcmp.offset,
                },
              },
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
            ],
          },
        )
      ).map((account) => {
        return this.getMangoAccountFromAi(account.pubkey, account.account);
      }),
    );

    if (loadSerum3Oo) {
      await Promise.all(
        accounts.map(async (a) => await a.reloadSerum3OpenOrders(this)),
      );
    }

    return accounts;
  }

  public async getMangoAccountsForDelegate(
    group: Group,
    delegate: PublicKey,
    loadSerum3Oo = false,
  ): Promise<MangoAccount[]> {
    const discriminatorMemcmp: {
      offset: number;
      bytes: string;
    } = this.program.account.mangoAccount.coder.accounts.memcmp(
      'mangoAccount',
      undefined,
    );

    const accounts = await Promise.all(
      (
        await this.program.provider.connection.getProgramAccounts(
          this.programId,
          {
            filters: [
              {
                memcmp: {
                  bytes: discriminatorMemcmp.bytes,
                  offset: discriminatorMemcmp.offset,
                },
              },
              {
                memcmp: {
                  bytes: group.publicKey.toBase58(),
                  offset: 8,
                },
              },
              {
                memcmp: {
                  bytes: delegate.toBase58(),
                  offset: 104,
                },
              },
            ],
          },
        )
      ).map((account) => {
        return this.getMangoAccountFromAi(account.pubkey, account.account);
      }),
    );

    if (loadSerum3Oo) {
      await Promise.all(
        accounts.map(async (a) => await a.reloadSerum3OpenOrders(this)),
      );
    }

    return accounts;
  }

  public async getAllMangoAccounts(
    group: Group,
    loadSerum3Oo = false,
  ): Promise<MangoAccount[]> {
    const discriminatorMemcmp: {
      offset: number;
      bytes: string;
    } = this.program.account.mangoAccount.coder.accounts.memcmp(
      'mangoAccount',
      undefined,
    );

    const accounts = await Promise.all(
      (
        await this.program.provider.connection.getProgramAccounts(
          this.programId,
          {
            filters: [
              {
                memcmp: {
                  bytes: discriminatorMemcmp.bytes,
                  offset: discriminatorMemcmp.offset,
                },
              },
              {
                memcmp: {
                  bytes: group.publicKey.toBase58(),
                  offset: 8,
                },
              },
            ],
          },
        )
      ).map((account) => {
        return this.getMangoAccountFromAi(account.pubkey, account.account);
      }),
    );

    if (loadSerum3Oo) {
      const ooPks = accounts
        .map((a) => a.serum3Active().map((serum3) => serum3.openOrders))
        .flat();

      const ais: AccountInfo<Buffer>[] = (
        await Promise.all(
          chunk(ooPks, 100).map(
            async (ooPksChunk) =>
              await this.program.provider.connection.getMultipleAccountsInfo(
                ooPksChunk,
              ),
          ),
        )
      ).flat();

      if (ooPks.length != ais.length) {
        throw new Error(`Error in fetch all open orders accounts!`);
      }

      const serum3OosMapByOo = new Map(
        Array.from(
          ais.map((ai, i) => {
            if (ai == null) {
              throw new Error(`Undefined AI for open orders ${ooPks[i]}!`);
            }
            const oo = OpenOrders.fromAccountInfo(
              ooPks[i],
              ai,
              OPENBOOK_PROGRAM_ID[this.cluster],
            );
            return [ooPks[i].toBase58(), oo];
          }),
        ),
      );

      accounts.forEach(
        async (a) => await a.loadSerum3OpenOrders(serum3OosMapByOo),
      );
    }

    return accounts;
  }

  /**
   * Note: this ix doesn't settle liabs, reduce open positions, or withdraw tokens to wallet,
   * it simply closes the account. To close successfully ensure all positions are closed, or
   * use forceClose flag
   * @param group
   * @param mangoAccount
   * @param forceClose
   * @returns
   */
  public async closeMangoAccount(
    group: Group,
    mangoAccount: MangoAccount,
    forceClose = false,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.program.methods
      .accountClose(forceClose)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        solDestination: mangoAccount.owner,
      })
      .instruction();

    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async emptyAndCloseMangoAccount(
    group: Group,
    mangoAccount: MangoAccount,
  ): Promise<MangoSignatureStatus> {
    // Work on a deep cloned mango account, since we would deactivating positions
    // before deactivation reaches on-chain state in order to simplify building a fresh list
    // of healthRemainingAccounts to each subsequent ix
    const clonedMangoAccount = copy(mangoAccount);
    const instructions: TransactionInstruction[] = [];

    for (const serum3Account of clonedMangoAccount.serum3Active()) {
      const serum3Market = group.serum3MarketsMapByMarketIndex.get(
        serum3Account.marketIndex,
      )!;

      const closeOOIx = await this.serum3CloseOpenOrdersIx(
        group,
        clonedMangoAccount,
        serum3Market.serumMarketExternal,
      );
      instructions.push(closeOOIx);
      serum3Account.marketIndex =
        Serum3Orders.Serum3MarketIndexUnset as MarketIndex;
    }

    for (const pp of clonedMangoAccount.perpActive()) {
      const perpMarketIndex = pp.marketIndex;
      const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
      const deactivatingPositionIx = await this.perpDeactivatePositionIx(
        group,
        clonedMangoAccount,
        perpMarketIndex,
      );
      instructions.push(deactivatingPositionIx);
      pp.marketIndex = PerpPosition.PerpMarketIndexUnset as PerpMarketIndex;
    }

    for (const tp of clonedMangoAccount.tokensActive()) {
      const bank = group.getFirstBankByTokenIndex(tp.tokenIndex);
      const withdrawIx = await this.tokenWithdrawNativeIx(
        group,
        clonedMangoAccount,
        bank.mint,
        U64_MAX_BN,
        false,
      );
      instructions.push(...withdrawIx);
      tp.tokenIndex = TokenPosition.TokenIndexUnset as TokenIndex;
    }

    const closeIx = await this.program.methods
      .accountClose(false)
      .accounts({
        group: group.publicKey,
        account: clonedMangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        solDestination: clonedMangoAccount.owner,
      })
      .instruction();
    instructions.push(closeIx);

    return await this.sendAndConfirmTransactionForGroup(group, instructions);
  }

  public async accountBuybackFeesWithMngoIx(
    group: Group,
    mangoAccount: MangoAccount,
    maxBuybackUsd?: number,
  ): Promise<TransactionInstruction> {
    maxBuybackUsd = maxBuybackUsd ?? mangoAccount.getMaxFeesBuybackUi(group);
    return await this.program.methods
      .accountBuybackFeesWithMngo(toNative(maxBuybackUsd, 6))
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        daoAccount: group.buybackFeesSwapMangoAccount,
        mngoBank: group.getFirstBankForMngo().publicKey,
        mngoOracle: group.getFirstBankForMngo().oracle,
        feesBank: group.getFirstBankByTokenIndex(0 as TokenIndex).publicKey,
        feesOracle: group.getFirstBankByTokenIndex(0 as TokenIndex).oracle,
      })
      .instruction();
  }

  public async accountBuybackFeesWithMngo(
    group: Group,
    mangoAccount: MangoAccount,
    maxBuyback?: number,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.accountBuybackFeesWithMngoIx(
      group,
      mangoAccount,
      maxBuyback,
    );
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async tokenDeposit(
    group: Group,
    mangoAccount: MangoAccount,
    mintPk: PublicKey,
    amount: number,
    reduceOnly = false,
  ): Promise<MangoSignatureStatus> {
    const decimals = group.getMintDecimals(mintPk);
    const nativeAmount = toNative(amount, decimals);
    return await this.tokenDepositNative(
      group,
      mangoAccount,
      mintPk,
      nativeAmount,
      reduceOnly,
    );
  }

  public async tokenDepositNative(
    group: Group,
    mangoAccount: MangoAccount,
    mintPk: PublicKey,
    nativeAmount: BN,
    reduceOnly = false,
    intoExisting = false,
  ): Promise<MangoSignatureStatus> {
    const bank = group.getFirstBankByMint(mintPk);

    const walletPk = this.walletPk;
    const tokenAccountPk = await getAssociatedTokenAddress(
      mintPk,
      walletPk,
      // Allow ATA authority to be a PDA
      true,
    );

    let wrappedSolAccount: PublicKey | undefined;
    let preInstructions: TransactionInstruction[] = [];
    let postInstructions: TransactionInstruction[] = [];
    if (mintPk.equals(NATIVE_MINT)) {
      // Generate a random seed for wrappedSolAccount.
      const seed = Keypair.generate().publicKey.toBase58().slice(0, 32);
      // Calculate a publicKey that will be controlled by the current wallet.
      wrappedSolAccount = await PublicKey.createWithSeed(
        walletPk,
        seed,
        TOKEN_PROGRAM_ID,
      );

      const lamports = nativeAmount.add(new BN(1e7));

      preInstructions = [
        SystemProgram.createAccountWithSeed({
          fromPubkey: walletPk,
          basePubkey: walletPk,
          seed,
          newAccountPubkey: wrappedSolAccount,
          lamports: lamports.toNumber(),
          space: 165,
          programId: TOKEN_PROGRAM_ID,
        }),
        createInitializeAccount3Instruction(
          wrappedSolAccount,
          NATIVE_MINT,
          walletPk,
        ),
      ];
      postInstructions = [
        createCloseAccountInstruction(wrappedSolAccount, walletPk, walletPk),
      ];
    }

    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(group, [mangoAccount], [bank], []);

    const sharedAccounts = {
      group: group.publicKey,
      account: mangoAccount.publicKey,
      bank: bank.publicKey,
      vault: bank.vault,
      oracle: bank.oracle,
      tokenAccount: wrappedSolAccount ?? tokenAccountPk,
      tokenAuthority: walletPk,
    };

    let ixBase;
    if (!intoExisting) {
      ixBase = this.program.methods
        .tokenDeposit(new BN(nativeAmount), reduceOnly)
        .accounts({
          owner: mangoAccount.owner,
          ...sharedAccounts,
        });
    } else {
      ixBase = this.program.methods
        .tokenDepositIntoExisting(new BN(nativeAmount), reduceOnly)
        .accounts(sharedAccounts);
    }

    const ix = await ixBase
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .instruction();

    return await this.sendAndConfirmTransactionForGroup(group, [
      ...preInstructions,
      ix,
      ...postInstructions,
    ]);
  }

  public async tokenWithdrawAllDepositForAllUnconfidentOrStaleOracles(
    group: Group,
    mangoAccount: MangoAccount,
  ): Promise<MangoSignatureStatus> {
    const nowSlot = await this.connection.getSlot();

    const banksToWithdrawFrom = Array.from(group.banksMapByTokenIndex.values())
      .map((banks) => banks[0])
      .filter((bank) => bank.isOracleStaleOrUnconfident(nowSlot))
      .filter((b) => mangoAccount.getTokenBalanceUi(b) > 0);

    if (banksToWithdrawFrom.length === 0) {
      throw new Error(`No stale oracle or bad confidence oracle deposits!`);
    }

    const ixs = await Promise.all(
      banksToWithdrawFrom.map((bank) => {
        return this.tokenWithdrawNativeIx(
          group,
          mangoAccount,
          bank.mint,
          U64_MAX_BN,
          false,
        );
      }),
    );

    return await this.sendAndConfirmTransactionForGroup(group, ixs.flat());
  }

  /**
   * Withdraw the entire deposit balance for a token, effectively freeing the token position
   *
   * @param group
   * @param mangoAccount
   * @param mintPk
   * @returns
   */
  public async tokenWithdrawAllDepositForMint(
    group: Group,
    mangoAccount: MangoAccount,
    mintPk: PublicKey,
  ): Promise<MangoSignatureStatus> {
    const bank = group.getFirstBankByMint(mintPk);
    const b = mangoAccount.getTokenBalance(bank).toNumber();
    if (b < 0) {
      throw new Error(`Only call this method for deposits and not borrows!`);
    }
    const ixs = await this.tokenWithdrawNativeIx(
      group,
      mangoAccount,
      mintPk,
      U64_MAX_BN,
      false,
    );

    return await this.sendAndConfirmTransactionForGroup(group, ixs);
  }

  public async tokenWithdraw(
    group: Group,
    mangoAccount: MangoAccount,
    mintPk: PublicKey,
    amount: number,
    allowBorrow: boolean,
  ): Promise<MangoSignatureStatus> {
    const nativeAmount = toNative(amount, group.getMintDecimals(mintPk));
    const ixs = await this.tokenWithdrawNativeIx(
      group,
      mangoAccount,
      mintPk,
      nativeAmount,
      allowBorrow,
    );

    return await this.sendAndConfirmTransactionForGroup(group, ixs);
  }

  public async tokenWithdrawNativeIx(
    group: Group,
    mangoAccount: MangoAccount,
    mintPk: PublicKey,
    nativeAmount: BN,
    allowBorrow: boolean,
  ): Promise<TransactionInstruction[]> {
    const bank = group.getFirstBankByMint(mintPk);

    const tokenAccountPk = await getAssociatedTokenAddress(
      bank.mint,
      mangoAccount.owner,
      true,
    );

    // ensure withdraws don't fail with missing ATAs
    const preInstructions: TransactionInstruction[] = [
      await createAssociatedTokenAccountIdempotentInstruction(
        mangoAccount.owner,
        mangoAccount.owner,
        bank.mint,
      ),
    ];

    const postInstructions: TransactionInstruction[] = [];
    if (mintPk.equals(NATIVE_MINT)) {
      postInstructions.push(
        createCloseAccountInstruction(
          tokenAccountPk,
          mangoAccount.owner,
          mangoAccount.owner,
        ),
      );
    }

    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(group, [mangoAccount], [bank], [], []);

    const ix = await this.program.methods
      .tokenWithdraw(new BN(nativeAmount), allowBorrow)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: mangoAccount.owner,
        bank: bank.publicKey,
        vault: bank.vault,
        oracle: bank.oracle,
        tokenAccount: tokenAccountPk,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({
              pubkey: pk,
              isWritable: false,
              isSigner: false,
            } as AccountMeta),
        ),
      )
      .instruction();

    return [...preInstructions, ix, ...postInstructions];
  }

  public async tokenWithdrawNative(
    group: Group,
    mangoAccount: MangoAccount,
    mintPk: PublicKey,
    nativeAmount: BN,
    allowBorrow: boolean,
  ): Promise<MangoSignatureStatus> {
    const ixs = await this.tokenWithdrawNativeIx(
      group,
      mangoAccount,
      mintPk,
      nativeAmount,
      allowBorrow,
    );
    return await this.sendAndConfirmTransactionForGroup(group, ixs);
  }

  // Serum

  public async serum3RegisterMarket(
    group: Group,
    serum3MarketExternalPk: PublicKey,
    baseBank: Bank,
    quoteBank: Bank,
    marketIndex: number,
    name: string,
    oraclePriceBand: number,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.program.methods
      .serum3RegisterMarket(marketIndex, name, oraclePriceBand)
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        serumProgram: OPENBOOK_PROGRAM_ID[this.cluster],
        serumMarketExternal: serum3MarketExternalPk,
        baseBank: baseBank.publicKey,
        quoteBank: quoteBank.publicKey,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async serum3ConsumeEvents(
    group: Group,
    serum3MarketExternalPk: PublicKey,
  ): Promise<MangoSignatureStatus> {
    const serum3MarketExternal = group.serum3ExternalMarketsMap.get(
      serum3MarketExternalPk.toBase58(),
    )!;
    const ai = await this.program.provider.connection.getAccountInfo(
      serum3MarketExternal.decoded.eventQueue,
    );
    const eq = decodeEventQueue(ai!.data);
    const orderedAccounts: PublicKey[] = eq
      .map((e) => e.openOrders)
      .sort((a, b) => a.toBuffer().swap64().compare(b.toBuffer().swap64()));
    if (orderedAccounts.length == 0) {
      throw new Error(`Event queue is empty!`);
    }
    return this.sendAndConfirmTransactionForGroup(group, [
      serum3MarketExternal.makeConsumeEventsInstruction(orderedAccounts, 65535),
    ]);
  }

  public async serum3EditMarketIx(
    group: Group,
    serum3MarketIndex: MarketIndex,
    admin: PublicKey,
    reduceOnly: boolean | null,
    forceClose: boolean | null,
    name: string | null,
    oraclePriceBand: number | null,
  ): Promise<TransactionInstruction> {
    const serum3Market =
      group.serum3MarketsMapByMarketIndex.get(serum3MarketIndex);
    const ix = await this.program.methods
      .serum3EditMarket(reduceOnly, forceClose, name, oraclePriceBand)
      .accounts({
        group: group.publicKey,
        admin: admin,
        market: serum3Market?.publicKey,
      })
      .instruction();
    return ix;
  }

  public async serum3EditMarket(
    group: Group,
    serum3MarketIndex: MarketIndex,
    reduceOnly: boolean | null,
    forceClose: boolean | null,
    name: string | null,
    oraclePriceBand: number | null,
  ): Promise<MangoSignatureStatus> {
    const admin = (this.program.provider as AnchorProvider).wallet.publicKey;
    const ix = await this.serum3EditMarketIx(
      group,
      serum3MarketIndex,
      admin,
      reduceOnly,
      forceClose,
      name,
      oraclePriceBand,
    );
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async serum3deregisterMarket(
    group: Group,
    externalMarketPk: PublicKey,
  ): Promise<MangoSignatureStatus> {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;

    const marketIndexBuf = Buffer.alloc(2);
    marketIndexBuf.writeUInt16LE(serum3Market.marketIndex);
    const [indexReservation] = await PublicKey.findProgramAddress(
      [Buffer.from('Serum3Index'), group.publicKey.toBuffer(), marketIndexBuf],
      this.program.programId,
    );

    const ix = await this.program.methods
      .serum3DeregisterMarket()
      .accounts({
        group: group.publicKey,
        serumMarket: serum3Market.publicKey,
        indexReservation,
        solDestination: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
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
    externalMarketPk: PublicKey,
  ): Promise<MangoSignatureStatus> {
    const serum3Market: Serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;

    const ix = await this.program.methods
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
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async serum3CreateOpenOrdersIx(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
  ): Promise<TransactionInstruction> {
    const serum3Market: Serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;

    const ix = await this.program.methods
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
      .instruction();

    return ix;
  }

  public async serum3CloseOpenOrdersIx(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
  ): Promise<TransactionInstruction> {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;

    return await this.program.methods
      .serum3CloseOpenOrders()
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        serumMarket: serum3Market.publicKey,
        serumProgram: serum3Market.serumProgram,
        serumMarketExternal: serum3Market.serumMarketExternal,
        openOrders: await serum3Market.findOoPda(
          this.programId,
          mangoAccount.publicKey,
        ),
        solDestination: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .instruction();
  }

  public async serum3CloseOpenOrders(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.serum3CloseOpenOrdersIx(
      group,
      mangoAccount,
      externalMarketPk,
    );

    return await sendTransaction(
      this.program.provider as AnchorProvider,
      [ix],
      group.addressLookupTablesList,
      {
        postSendTxCallback: this.postSendTxCallback,
        postTxConfirmationCallback: this.postTxConfirmationCallback,
      },
    );
  }

  public async serum3LiqForceCancelOrders(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    limit?: number,
  ): Promise<MangoSignatureStatus> {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;
    const serum3MarketExternal = group.serum3ExternalMarketsMap.get(
      externalMarketPk.toBase58(),
    )!;
    const openOrders = await serum3Market.findOoPda(
      this.programId,
      mangoAccount.publicKey,
    );

    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        group,
        [mangoAccount],
        [],
        [],
        [[serum3Market, openOrders]],
      );

    const ix = await this.program.methods
      .serum3LiqForceCancelOrders(limit ?? 10)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        openOrders,
        serumMarket: serum3Market.publicKey,
        serumProgram: OPENBOOK_PROGRAM_ID[this.cluster],
        serumMarketExternal: serum3Market.serumMarketExternal,
        marketBids: serum3MarketExternal.bidsAddress,
        marketAsks: serum3MarketExternal.asksAddress,
        marketEventQueue: serum3MarketExternal.decoded.eventQueue,
        marketBaseVault: serum3MarketExternal.decoded.baseVault,
        marketQuoteVault: serum3MarketExternal.decoded.quoteVault,
        marketVaultSigner: await generateSerum3MarketExternalVaultSignerAddress(
          this.cluster,
          serum3Market,
          serum3MarketExternal,
        ),
        quoteBank: group.getFirstBankByTokenIndex(serum3Market.quoteTokenIndex)
          .publicKey,
        quoteVault: group.getFirstBankByTokenIndex(serum3Market.quoteTokenIndex)
          .vault,
        baseBank: group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex)
          .publicKey,
        baseVault: group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex)
          .vault,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .instruction();

    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async serum3PlaceOrderIx(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    side: Serum3Side,
    price: number,
    size: number,
    selfTradeBehavior: Serum3SelfTradeBehavior,
    orderType: Serum3OrderType,
    clientOrderId: number,
    limit: number,
  ): Promise<TransactionInstruction[]> {
    return await this.serum3PlaceOrderV2Ix(
      group,
      mangoAccount,
      externalMarketPk,
      side,
      price,
      size,
      selfTradeBehavior,
      orderType,
      clientOrderId,
      limit,
    );
  }

  public async serum3PlaceOrderV1Ix(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    side: Serum3Side,
    price: number,
    size: number,
    selfTradeBehavior: Serum3SelfTradeBehavior,
    orderType: Serum3OrderType,
    clientOrderId: number,
    limit: number,
  ): Promise<TransactionInstruction[]> {
    const ixs: TransactionInstruction[] = [];
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;

    let openOrderPk: PublicKey | undefined = undefined;
    const banks: Bank[] = [];
    const openOrdersForMarket: [Serum3Market, PublicKey][] = [];
    if (!mangoAccount.getSerum3Account(serum3Market.marketIndex)) {
      const ix = await this.serum3CreateOpenOrdersIx(
        group,
        mangoAccount,
        serum3Market.serumMarketExternal,
      );
      ixs.push(ix);
      openOrderPk = await serum3Market.findOoPda(
        this.program.programId,
        mangoAccount.publicKey,
      );
      openOrdersForMarket.push([serum3Market, openOrderPk]);
      const baseTokenIndex = serum3Market.baseTokenIndex;
      const quoteTokenIndex = serum3Market.quoteTokenIndex;
      // only include banks if no deposit has been previously made for same token
      banks.push(group.getFirstBankByTokenIndex(quoteTokenIndex));
      banks.push(group.getFirstBankByTokenIndex(baseTokenIndex));
    }

    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        group,
        [mangoAccount],
        banks,
        [],
        openOrdersForMarket,
      );

    const serum3MarketExternal = group.serum3ExternalMarketsMap.get(
      externalMarketPk.toBase58(),
    )!;
    const serum3MarketExternalVaultSigner =
      await generateSerum3MarketExternalVaultSignerAddress(
        this.cluster,
        serum3Market,
        serum3MarketExternal,
      );

    const limitPrice = serum3MarketExternal.priceNumberToLots(price);
    const maxBaseQuantity = serum3MarketExternal.baseSizeNumberToLots(size);
    const isTaker = orderType !== Serum3OrderType.postOnly;
    const maxQuoteQuantity = new BN(
      Math.ceil(
        serum3MarketExternal.decoded.quoteLotSize.toNumber() *
          (1 + Math.max(serum3Market.getFeeRates(isTaker), 0)) *
          serum3MarketExternal.baseSizeNumberToLots(size).toNumber() *
          serum3MarketExternal.priceNumberToLots(price).toNumber(),
      ),
    );

    const payerTokenIndex = ((): TokenIndex => {
      if (side == Serum3Side.bid) {
        return serum3Market.quoteTokenIndex;
      } else {
        return serum3Market.baseTokenIndex;
      }
    })();

    const payerBank = group.getFirstBankByTokenIndex(payerTokenIndex);
    const ix = await this.program.methods
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
        openOrders:
          openOrderPk ||
          mangoAccount.getSerum3Account(serum3Market.marketIndex)?.openOrders,
        serumMarket: serum3Market.publicKey,
        serumProgram: OPENBOOK_PROGRAM_ID[this.cluster],
        serumMarketExternal: serum3Market.serumMarketExternal,
        marketBids: serum3MarketExternal.bidsAddress,
        marketAsks: serum3MarketExternal.asksAddress,
        marketEventQueue: serum3MarketExternal.decoded.eventQueue,
        marketRequestQueue: serum3MarketExternal.decoded.requestQueue,
        marketBaseVault: serum3MarketExternal.decoded.baseVault,
        marketQuoteVault: serum3MarketExternal.decoded.quoteVault,
        marketVaultSigner: serum3MarketExternalVaultSigner,
        payerBank: payerBank.publicKey,
        payerVault: payerBank.vault,
        payerOracle: payerBank.oracle,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .instruction();

    ixs.push(ix);

    return ixs;
  }

  public async serum3PlaceOrderV2Ix(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    side: Serum3Side,
    price: number,
    size: number,
    selfTradeBehavior: Serum3SelfTradeBehavior,
    orderType: Serum3OrderType,
    clientOrderId: number,
    limit: number,
  ): Promise<TransactionInstruction[]> {
    const ixs: TransactionInstruction[] = [];
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;

    let openOrderPk: PublicKey | undefined = undefined;
    const banks: Bank[] = [];
    const openOrdersForMarket: [Serum3Market, PublicKey][] = [];
    if (!mangoAccount.getSerum3Account(serum3Market.marketIndex)) {
      const ix = await this.serum3CreateOpenOrdersIx(
        group,
        mangoAccount,
        serum3Market.serumMarketExternal,
      );
      ixs.push(ix);
      openOrderPk = await serum3Market.findOoPda(
        this.program.programId,
        mangoAccount.publicKey,
      );
      openOrdersForMarket.push([serum3Market, openOrderPk]);
      const baseTokenIndex = serum3Market.baseTokenIndex;
      const quoteTokenIndex = serum3Market.quoteTokenIndex;
      // only include banks if no deposit has been previously made for same token
      banks.push(group.getFirstBankByTokenIndex(quoteTokenIndex));
      banks.push(group.getFirstBankByTokenIndex(baseTokenIndex));
    }

    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        group,
        [mangoAccount],
        banks,
        [],
        openOrdersForMarket,
      );

    const serum3MarketExternal = group.serum3ExternalMarketsMap.get(
      externalMarketPk.toBase58(),
    )!;
    const serum3MarketExternalVaultSigner =
      await generateSerum3MarketExternalVaultSignerAddress(
        this.cluster,
        serum3Market,
        serum3MarketExternal,
      );

    const limitPrice = serum3MarketExternal.priceNumberToLots(price);
    const maxBaseQuantity = serum3MarketExternal.baseSizeNumberToLots(size);
    const isTaker = orderType !== Serum3OrderType.postOnly;
    const maxQuoteQuantity = new BN(
      Math.ceil(
        serum3MarketExternal.decoded.quoteLotSize.toNumber() *
          (1 + Math.max(serum3Market.getFeeRates(isTaker), 0)) *
          serum3MarketExternal.baseSizeNumberToLots(size).toNumber() *
          serum3MarketExternal.priceNumberToLots(price).toNumber(),
      ),
    );

    const payerTokenIndex = ((): TokenIndex => {
      if (side == Serum3Side.bid) {
        return serum3Market.quoteTokenIndex;
      } else {
        return serum3Market.baseTokenIndex;
      }
    })();

    const receiverTokenIndex = ((): TokenIndex => {
      if (side == Serum3Side.bid) {
        return serum3Market.baseTokenIndex;
      } else {
        return serum3Market.quoteTokenIndex;
      }
    })();

    const payerBank = group.getFirstBankByTokenIndex(payerTokenIndex);
    const receiverBank = group.getFirstBankByTokenIndex(receiverTokenIndex);
    const ix = await this.program.methods
      .serum3PlaceOrderV2(
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
        openOrders:
          openOrderPk ||
          mangoAccount.getSerum3Account(serum3Market.marketIndex)?.openOrders,
        serumMarket: serum3Market.publicKey,
        serumProgram: OPENBOOK_PROGRAM_ID[this.cluster],
        serumMarketExternal: serum3Market.serumMarketExternal,
        marketBids: serum3MarketExternal.bidsAddress,
        marketAsks: serum3MarketExternal.asksAddress,
        marketEventQueue: serum3MarketExternal.decoded.eventQueue,
        marketRequestQueue: serum3MarketExternal.decoded.requestQueue,
        marketBaseVault: serum3MarketExternal.decoded.baseVault,
        marketQuoteVault: serum3MarketExternal.decoded.quoteVault,
        marketVaultSigner: serum3MarketExternalVaultSigner,
        payerBank: payerBank.publicKey,
        payerVault: payerBank.vault,
        payerOracle: payerBank.oracle,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({
              pubkey: pk,
              isWritable: receiverBank.publicKey.equals(pk) ? true : false,
              isSigner: false,
            } as AccountMeta),
        ),
      )
      .instruction();

    ixs.push(ix);

    return ixs;
  }

  public async serum3PlaceOrder(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    side: Serum3Side,
    price: number,
    size: number,
    selfTradeBehavior: Serum3SelfTradeBehavior,
    orderType: Serum3OrderType,
    clientOrderId: number,
    limit: number,
  ): Promise<MangoSignatureStatus> {
    const placeOrderIxs = await this.serum3PlaceOrderV2Ix(
      group,
      mangoAccount,
      externalMarketPk,
      side,
      price,
      size,
      selfTradeBehavior,
      orderType,
      clientOrderId,
      limit,
    );

    const settleIx = await this.serum3SettleFundsIx(
      group,
      mangoAccount,
      externalMarketPk,
    );

    const ixs = [...placeOrderIxs, settleIx];

    return await this.sendAndConfirmTransactionForGroup(group, ixs);
  }

  public async serum3CancelAllOrdersIx(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    limit?: number,
  ): Promise<TransactionInstruction> {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;

    const serum3MarketExternal = group.serum3ExternalMarketsMap.get(
      externalMarketPk.toBase58(),
    )!;

    return await this.program.methods
      .serum3CancelAllOrders(limit ? limit : 10)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        openOrders: await serum3Market.findOoPda(
          this.programId,
          mangoAccount.publicKey,
        ),
        serumMarket: serum3Market.publicKey,
        serumProgram: OPENBOOK_PROGRAM_ID[this.cluster],
        serumMarketExternal: serum3Market.serumMarketExternal,
        marketBids: serum3MarketExternal.bidsAddress,
        marketAsks: serum3MarketExternal.asksAddress,
        marketEventQueue: serum3MarketExternal.decoded.eventQueue,
      })
      .instruction();
  }

  public async serum3CancelAllOrders(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    limit?: number,
  ): Promise<MangoSignatureStatus> {
    return await this.sendAndConfirmTransactionForGroup(group, [
      await this.serum3CancelAllOrdersIx(
        group,
        mangoAccount,
        externalMarketPk,
        limit,
      ),
    ]);
  }

  public async serum3SettleFundsIx(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
  ): Promise<TransactionInstruction> {
    if (this.openbookFeesToDao == false) {
      throw new Error(
        `openbookFeesToDao is set to false, please use serum3SettleFundsV2Ix`,
      );
    }

    return await this.serum3SettleFundsV2Ix(
      group,
      mangoAccount,
      externalMarketPk,
    );
  }

  public async serum3SettleFundsV2Ix(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
  ): Promise<TransactionInstruction> {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;
    const serum3MarketExternal = group.serum3ExternalMarketsMap.get(
      externalMarketPk.toBase58(),
    )!;

    const [serum3MarketExternalVaultSigner, openOrderPublicKey] =
      await Promise.all([
        generateSerum3MarketExternalVaultSignerAddress(
          this.cluster,
          serum3Market,
          serum3MarketExternal,
        ),
        serum3Market.findOoPda(this.program.programId, mangoAccount.publicKey),
      ]);

    const ix = await this.program.methods
      .serum3SettleFundsV2(this.openbookFeesToDao)
      .accounts({
        v1: {
          group: group.publicKey,
          account: mangoAccount.publicKey,
          owner: (this.program.provider as AnchorProvider).wallet.publicKey,
          openOrders: openOrderPublicKey,
          serumMarket: serum3Market.publicKey,
          serumProgram: OPENBOOK_PROGRAM_ID[this.cluster],
          serumMarketExternal: serum3Market.serumMarketExternal,
          marketBaseVault: serum3MarketExternal.decoded.baseVault,
          marketQuoteVault: serum3MarketExternal.decoded.quoteVault,
          marketVaultSigner: serum3MarketExternalVaultSigner,
          quoteBank: group.getFirstBankByTokenIndex(
            serum3Market.quoteTokenIndex,
          ).publicKey,
          quoteVault: group.getFirstBankByTokenIndex(
            serum3Market.quoteTokenIndex,
          ).vault,
          baseBank: group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex)
            .publicKey,
          baseVault: group.getFirstBankByTokenIndex(serum3Market.baseTokenIndex)
            .vault,
        },
        v2: {
          quoteOracle: group.getFirstBankByTokenIndex(
            serum3Market.quoteTokenIndex,
          ).oracle,
          baseOracle: group.getFirstBankByTokenIndex(
            serum3Market.baseTokenIndex,
          ).oracle,
        },
      })
      .instruction();

    return ix;
  }

  public async serum3SettleFunds(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.serum3SettleFundsV2Ix(
      group,
      mangoAccount,
      externalMarketPk,
    );

    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async serum3CancelOrderIx(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    side: Serum3Side,
    orderId: BN,
  ): Promise<TransactionInstruction> {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;

    const serum3MarketExternal = group.serum3ExternalMarketsMap.get(
      externalMarketPk.toBase58(),
    )!;

    const ix = await this.program.methods
      .serum3CancelOrder(side, orderId)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        openOrders: mangoAccount.getSerum3Account(serum3Market.marketIndex)
          ?.openOrders,
        serumMarket: serum3Market.publicKey,
        serumProgram: OPENBOOK_PROGRAM_ID[this.cluster],
        serumMarketExternal: serum3Market.serumMarketExternal,
        marketBids: serum3MarketExternal.bidsAddress,
        marketAsks: serum3MarketExternal.asksAddress,
        marketEventQueue: serum3MarketExternal.decoded.eventQueue,
      })
      .instruction();

    return ix;
  }

  public async serum3CancelOrder(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    side: Serum3Side,
    orderId: BN,
  ): Promise<MangoSignatureStatus> {
    const ixs = await Promise.all([
      this.serum3CancelOrderIx(
        group,
        mangoAccount,
        externalMarketPk,
        side,
        orderId,
      ),
      this.serum3SettleFundsV2Ix(group, mangoAccount, externalMarketPk),
    ]);

    return await this.sendAndConfirmTransactionForGroup(group, ixs);
  }

  public async serum3CancelOrderByClientIdIx(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    clientOrderId: BN,
  ): Promise<TransactionInstruction> {
    const serum3Market = group.serum3MarketsMapByExternal.get(
      externalMarketPk.toBase58(),
    )!;

    const serum3MarketExternal = group.serum3ExternalMarketsMap.get(
      externalMarketPk.toBase58(),
    )!;

    const ix = await this.program.methods
      .serum3CancelOrderByClientOrderId(clientOrderId)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        openOrders: mangoAccount.getSerum3Account(serum3Market.marketIndex)
          ?.openOrders,
        serumMarket: serum3Market.publicKey,
        serumProgram: OPENBOOK_PROGRAM_ID[this.cluster],
        serumMarketExternal: serum3Market.serumMarketExternal,
        marketBids: serum3MarketExternal.bidsAddress,
        marketAsks: serum3MarketExternal.asksAddress,
        marketEventQueue: serum3MarketExternal.decoded.eventQueue,
      })
      .instruction();

    return ix;
  }

  public async serum3CancelOrderByClientId(
    group: Group,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    clientOrderId: BN,
  ): Promise<MangoSignatureStatus> {
    const ixs = await Promise.all([
      this.serum3CancelOrderByClientIdIx(
        group,
        mangoAccount,
        externalMarketPk,
        clientOrderId,
      ),
      this.serum3SettleFundsV2Ix(group, mangoAccount, externalMarketPk),
    ]);

    return await this.sendAndConfirmTransactionForGroup(group, ixs);
  }

  /// perps

  public async perpCreateMarket(
    group: Group,
    oraclePk: PublicKey,
    perpMarketIndex: number,
    name: string,
    oracleConfig: OracleConfigParams,
    baseDecimals: number,
    quoteLotSize: number,
    baseLotSize: number,
    maintBaseAssetWeight: number,
    initBaseAssetWeight: number,
    maintBaseLiabWeight: number,
    initBaseLiabWeight: number,
    maintOverallAssetWeight: number,
    initOverallAssetWeight: number,
    baseLiquidationFee: number,
    makerFee: number,
    takerFee: number,
    feePenalty: number,
    minFunding: number,
    maxFunding: number,
    impactQuantity: number,
    groupInsuranceFund: boolean,
    settleFeeFlat: number,
    settleFeeAmountThreshold: number,
    settleFeeFractionLowHealth: number,
    settleTokenIndex: number,
    settlePnlLimitFactor: number,
    settlePnlLimitWindowSize: number,
    positivePnlLiquidationFee: number,
    platformLiquidationFee: number,
  ): Promise<MangoSignatureStatus> {
    const bids = new Keypair();
    const asks = new Keypair();
    const eventQueue = new Keypair();

    const bookSideSize = (this.program as any)._coder.accounts.size(
      (this.program.account.bookSide as any)._idlAccount,
    );
    const eventQueueSize = (this.program as any)._coder.accounts.size(
      (this.program.account.eventQueue as any)._idlAccount,
    );

    const ix = await this.program.methods
      .perpCreateMarket(
        perpMarketIndex,
        name,
        oracleConfig,
        baseDecimals,
        new BN(quoteLotSize),
        new BN(baseLotSize),
        maintBaseAssetWeight,
        initBaseAssetWeight,
        maintBaseLiabWeight,
        initBaseLiabWeight,
        maintOverallAssetWeight,
        initOverallAssetWeight,
        baseLiquidationFee,
        makerFee,
        takerFee,
        minFunding,
        maxFunding,
        new BN(impactQuantity),
        groupInsuranceFund,
        feePenalty,
        settleFeeFlat,
        settleFeeAmountThreshold,
        settleFeeFractionLowHealth,
        settleTokenIndex,
        settlePnlLimitFactor,
        new BN(settlePnlLimitWindowSize),
        positivePnlLiquidationFee,
        platformLiquidationFee,
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
      .instruction();
    const preInstructions = [
      // book sides
      SystemProgram.createAccount({
        programId: this.program.programId,
        space: bookSideSize,
        lamports:
          await this.program.provider.connection.getMinimumBalanceForRentExemption(
            bookSideSize,
          ),
        fromPubkey: (this.program.provider as AnchorProvider).wallet.publicKey,
        newAccountPubkey: bids.publicKey,
      }),
      SystemProgram.createAccount({
        programId: this.program.programId,
        space: bookSideSize,
        lamports:
          await this.program.provider.connection.getMinimumBalanceForRentExemption(
            bookSideSize,
          ),
        fromPubkey: (this.program.provider as AnchorProvider).wallet.publicKey,
        newAccountPubkey: asks.publicKey,
      }),
      // event queue
      SystemProgram.createAccount({
        programId: this.program.programId,
        space: eventQueueSize,
        lamports:
          await this.program.provider.connection.getMinimumBalanceForRentExemption(
            eventQueueSize,
          ),
        fromPubkey: (this.program.provider as AnchorProvider).wallet.publicKey,
        newAccountPubkey: eventQueue.publicKey,
      }),
    ];
    return await this.sendAndConfirmTransactionForGroup(
      group,
      [...preInstructions, ix],
      {
        additionalSigners: [bids, asks, eventQueue],
      },
    );
  }

  public async perpEditMarket(
    group: Group,
    perpMarketIndex: PerpMarketIndex,
    params: PerpEditParams,
  ): Promise<MangoSignatureStatus> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);

    const ix = await this.program.methods
      .perpEditMarket(
        params.oracle,
        params.oracleConfig,
        params.baseDecimals,
        params.maintBaseAssetWeight,
        params.initBaseAssetWeight,
        params.maintBaseLiabWeight,
        params.initBaseLiabWeight,
        params.maintOverallAssetWeight,
        params.initOverallAssetWeight,
        params.baseLiquidationFee,
        params.makerFee,
        params.takerFee,
        params.minFunding,
        params.maxFunding,
        params.impactQuantity !== null ? new BN(params.impactQuantity) : null,
        params.groupInsuranceFund,
        params.feePenalty,
        params.settleFeeFlat,
        params.settleFeeAmountThreshold,
        params.settleFeeFractionLowHealth,
        params.stablePriceDelayIntervalSeconds,
        params.stablePriceDelayGrowthLimit,
        params.stablePriceGrowthLimit,
        params.settlePnlLimitFactor,
        params.settlePnlLimitWindowSize !== null
          ? new BN(params.settlePnlLimitWindowSize)
          : null,
        params.reduceOnly,
        params.resetStablePrice ?? false,
        params.positivePnlLiquidationFee,
        params.name,
        params.forceClose,
        params.platformLiquidationFee,
      )
      .accounts({
        group: group.publicKey,
        oracle: params.oracle ?? perpMarket.oracle,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        perpMarket: perpMarket.publicKey,
      })
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async perpForceClosePosition(
    group: Group,
    perpMarketIndex: PerpMarketIndex,
    accountA: MangoAccount,
    accountB: MangoAccount,
  ): Promise<MangoSignatureStatus> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);

    const ix = await this.program.methods
      .perpForceClosePosition()
      .accounts({
        group: group.publicKey,
        perpMarket: perpMarket.publicKey,
        accountA: accountA.publicKey,
        accountB: accountB.publicKey,
        oracle: perpMarket.oracle,
      })
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async perpCloseMarket(
    group: Group,
    perpMarketIndex: PerpMarketIndex,
  ): Promise<MangoSignatureStatus> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);

    const ix = await this.program.methods
      .perpCloseMarket()
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        perpMarket: perpMarket.publicKey,
        bids: perpMarket.bids,
        asks: perpMarket.asks,
        eventQueue: perpMarket.eventQueue,
        solDestination: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async perpGetMarkets(group: Group): Promise<PerpMarket[]> {
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

    return (await this.program.account.perpMarket.all(filters)).map((tuple) =>
      PerpMarket.from(tuple.publicKey, tuple.account),
    );
  }

  public async perpDeactivatePositionIx(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
  ): Promise<TransactionInstruction> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(group, [mangoAccount], [], []);
    return await this.program.methods
      .perpDeactivatePosition()
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        perpMarket: perpMarket.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .instruction();
  }

  public async perpDeactivatePosition(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.perpDeactivatePositionIx(
      group,
      mangoAccount,
      perpMarketIndex,
    );
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async perpCloseAll(
    group: Group,
    mangoAccount: MangoAccount,
    slippage = 0.01, // 1%, 100bps
  ): Promise<MangoSignatureStatus> {
    if (mangoAccount.perpActive().length == 0) {
      throw new Error(`No perp positions found.`);
    }

    if (mangoAccount.perpActive().length > 8) {
      // Technically we can fit in 16, 1.6M CU, 100k CU per ix, but lets be conservative
      throw new Error(
        `Can't close more than 8 positions in one tx, due to compute usage limit.`,
      );
    }

    const hrix1 = await this.healthRegionBeginIx(group, mangoAccount);
    const ixs = await Promise.all(
      mangoAccount.perpActive().map(async (pa) => {
        const pm = group.getPerpMarketByMarketIndex(pa.marketIndex);
        const isLong = pa.basePositionLots.gt(new BN(0));

        return await this.perpPlaceOrderV2Ix(
          group,
          mangoAccount,
          pa.marketIndex,
          isLong ? PerpOrderSide.ask : PerpOrderSide.bid,
          pm.uiPrice * (isLong ? 1 - slippage : 1 + slippage), // Try to cross the spread to guarantee matching
          Math.abs(pa.getBasePositionUi(pm) * 1.01), // Send a larger size to ensure full order is closed
          undefined,
          Date.now(),
          PerpOrderType.immediateOrCancel,
          PerpSelfTradeBehavior.decrementTake,
          true, // Reduce only
          undefined,
          undefined,
        );
      }),
    );
    const hrix2 = await this.healthRegionEndIx(group, mangoAccount);

    return await this.sendAndConfirmTransactionForGroup(
      group,
      [hrix1, ...ixs, hrix2],
      {
        estimateFee: true,
      },
    );
  }

  // perpPlaceOrder ix returns an optional, custom order id,
  // but, since we use a customer tx sender, this method
  // doesn't return it
  public async perpPlaceOrder(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    side: PerpOrderSide,
    price: number,
    quantity: number,
    maxQuoteQuantity?: number,
    clientOrderId?: number,
    orderType?: PerpOrderType,
    reduceOnly?: boolean,
    expiryTimestamp?: number,
    limit?: number,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.perpPlaceOrderV2Ix(
      group,
      mangoAccount,
      perpMarketIndex,
      side,
      price,
      quantity,
      maxQuoteQuantity,
      clientOrderId,
      orderType,
      PerpSelfTradeBehavior.decrementTake,
      reduceOnly,
      expiryTimestamp,
      limit,
    );

    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async perpPlaceOrderIx(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    side: PerpOrderSide,
    price: number,
    quantity: number,
    maxQuoteQuantity?: number,
    clientOrderId?: number,
    orderType?: PerpOrderType,
    reduceOnly?: boolean,
    expiryTimestamp?: number,
    limit?: number,
  ): Promise<TransactionInstruction> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        group,
        [mangoAccount],
        // Settlement token bank, because a position for it may be created
        [group.getFirstBankForPerpSettlement()],
        [perpMarket],
      );
    return await this.program.methods
      .perpPlaceOrder(
        side,
        perpMarket.uiPriceToLots(price),
        perpMarket.uiBaseToLots(quantity),
        maxQuoteQuantity
          ? perpMarket.uiQuoteToLots(maxQuoteQuantity)
          : I64_MAX_BN,
        new BN(clientOrderId ? clientOrderId : Date.now()),
        orderType ? orderType : PerpOrderType.limit,
        reduceOnly ? reduceOnly : false,
        new BN(expiryTimestamp ? expiryTimestamp : 0),
        limit ? limit : 10,
      )
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        perpMarket: perpMarket.publicKey,
        bids: perpMarket.bids,
        asks: perpMarket.asks,
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
      .instruction();
  }

  public async perpPlaceOrderV2Ix(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    side: PerpOrderSide,
    price: number,
    quantity: number,
    maxQuoteQuantity?: number,
    clientOrderId?: number,
    orderType?: PerpOrderType,
    selfTradeBehavior?: PerpSelfTradeBehavior,
    reduceOnly?: boolean,
    expiryTimestamp?: number,
    limit?: number,
  ): Promise<TransactionInstruction> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        group,
        [mangoAccount],
        // Settlement token bank, because a position for it may be created
        [group.getFirstBankForPerpSettlement()],
        [perpMarket],
      );
    return await this.program.methods
      .perpPlaceOrderV2(
        side,
        perpMarket.uiPriceToLots(price),
        perpMarket.uiBaseToLots(quantity),
        maxQuoteQuantity
          ? perpMarket.uiQuoteToLots(maxQuoteQuantity)
          : I64_MAX_BN,
        new BN(clientOrderId ? clientOrderId : Date.now()),
        orderType ?? PerpOrderType.limit,
        selfTradeBehavior ?? PerpSelfTradeBehavior.decrementTake,
        reduceOnly ?? false,
        new BN(expiryTimestamp ? expiryTimestamp : 0),
        limit ?? 10,
      )
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        perpMarket: perpMarket.publicKey,
        bids: perpMarket.bids,
        asks: perpMarket.asks,
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
      .instruction();
  }

  public async perpPlaceOrderPegged(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    side: PerpOrderSide,
    priceOffset: number,
    quantity: number,
    pegLimit?: number,
    maxQuoteQuantity?: number,
    clientOrderId?: number,
    orderType?: PerpOrderType,
    reduceOnly?: boolean,
    expiryTimestamp?: number,
    limit?: number,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.perpPlaceOrderPeggedV2Ix(
      group,
      mangoAccount,
      perpMarketIndex,
      side,
      priceOffset,
      quantity,
      pegLimit,
      maxQuoteQuantity,
      clientOrderId,
      orderType,
      PerpSelfTradeBehavior.decrementTake,
      reduceOnly,
      expiryTimestamp,
      limit,
    );

    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async perpPlaceOrderPeggedIx(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    side: PerpOrderSide,
    priceOffset: number,
    quantity: number,
    pegLimit?: number,
    maxQuoteQuantity?: number,
    clientOrderId?: number,
    orderType?: PerpOrderType,
    reduceOnly?: boolean,
    expiryTimestamp?: number,
    limit?: number,
  ): Promise<TransactionInstruction> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        group,
        [mangoAccount],
        // Settlement token bank, because a position for it may be created
        [group.getFirstBankForPerpSettlement()],
        [perpMarket],
      );
    return await this.program.methods
      .perpPlaceOrderPegged(
        side,
        perpMarket.uiPriceToLots(priceOffset),
        pegLimit ? perpMarket.uiPriceToLots(pegLimit) : new BN(-1),
        perpMarket.uiBaseToLots(quantity),
        maxQuoteQuantity
          ? perpMarket.uiQuoteToLots(maxQuoteQuantity)
          : I64_MAX_BN,
        new BN(clientOrderId ?? Date.now()),
        orderType ? orderType : PerpOrderType.limit,
        reduceOnly ? reduceOnly : false,
        new BN(expiryTimestamp ?? 0),
        limit ? limit : 10,
        -1,
      )
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        perpMarket: perpMarket.publicKey,
        bids: perpMarket.bids,
        asks: perpMarket.asks,
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
      .instruction();
  }

  public async perpPlaceOrderPeggedV2Ix(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    side: PerpOrderSide,
    priceOffset: number,
    quantity: number,
    pegLimit?: number,
    maxQuoteQuantity?: number,
    clientOrderId?: number,
    orderType?: PerpOrderType,
    selfTradeBehavior?: PerpSelfTradeBehavior,
    reduceOnly?: boolean,
    expiryTimestamp?: number,
    limit?: number,
  ): Promise<TransactionInstruction> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        group,
        [mangoAccount],
        // Settlement token bank, because a position for it may be created
        [group.getFirstBankForPerpSettlement()],
        [perpMarket],
      );
    return await this.program.methods
      .perpPlaceOrderPeggedV2(
        side,
        perpMarket.uiPriceToLots(priceOffset),
        pegLimit ? perpMarket.uiPriceToLots(pegLimit) : new BN(-1),
        perpMarket.uiBaseToLots(quantity),
        maxQuoteQuantity
          ? perpMarket.uiQuoteToLots(maxQuoteQuantity)
          : I64_MAX_BN,
        new BN(clientOrderId ?? Date.now()),
        orderType ?? PerpOrderType.limit,
        selfTradeBehavior ?? PerpSelfTradeBehavior.decrementTake,
        reduceOnly ?? false,
        new BN(expiryTimestamp ?? 0),
        limit ?? 10,
        -1,
      )
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        perpMarket: perpMarket.publicKey,
        bids: perpMarket.bids,
        asks: perpMarket.asks,
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
      .instruction();
  }

  public async perpCancelOrderByClientOrderIdIx(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    clientOrderId: BN,
  ): Promise<TransactionInstruction> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    return await this.program.methods
      .perpCancelOrderByClientOrderId(new BN(clientOrderId))
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        perpMarket: perpMarket.publicKey,
        bids: perpMarket.bids,
        asks: perpMarket.asks,
      })
      .instruction();
  }

  public async perpCancelOrderIx(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    orderId: BN,
  ): Promise<TransactionInstruction> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    return await this.program.methods
      .perpCancelOrder(new BN(orderId))
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        perpMarket: perpMarket.publicKey,
        bids: perpMarket.bids,
        asks: perpMarket.asks,
      })
      .instruction();
  }

  public async perpCancelOrder(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    orderId: BN,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.perpCancelOrderIx(
      group,
      mangoAccount,
      perpMarketIndex,
      orderId,
    );

    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async perpCancelAllOrders(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    limit: number,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.perpCancelAllOrdersIx(
      group,
      mangoAccount,
      perpMarketIndex,
      limit,
    );

    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async perpCancelAllOrdersIx(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    limit: number,
  ): Promise<TransactionInstruction> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    return await this.program.methods
      .perpCancelAllOrders(limit)
      .accounts({
        group: group.publicKey,
        account: mangoAccount.publicKey,
        perpMarket: perpMarket.publicKey,
        bids: perpMarket.bids,
        asks: perpMarket.asks,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
      })
      .instruction();
  }

  async settleAll(
    client: MangoClient,
    group: Group,
    mangoAccount: MangoAccount,
    allMangoAccounts?: MangoAccount[],
  ): Promise<MangoSignatureStatus> {
    if (!allMangoAccounts) {
      allMangoAccounts = await client.getAllMangoAccounts(group, true);
    }

    const ixs1 = new Array<TransactionInstruction>();
    // This is optimistic, since we might find the same opponent candidate for all markets,
    // and they have might not be able to settle at some point due to safety limits
    // Future: correct way to do is, to apply the settlement on a copy and then move to next position
    for (const pa of mangoAccount.perpActive()) {
      const pm = group.getPerpMarketByMarketIndex(pa.marketIndex);
      const candidates = await pm.getSettlePnlCandidates(
        client,
        group,
        allMangoAccounts,
        pa.getUnsettledPnlUi(pm) > 0 ? 'negative' : 'positive',
        2,
      );
      if (candidates.length == 0) {
        continue;
      }
      ixs1.push(
        // Takes ~250k CU
        await this.perpSettlePnlIx(
          group,
          pa.getUnsettledPnlUi(pm) > 0 ? mangoAccount : candidates[0].account,
          pa.getUnsettledPnlUi(pm) < 0 ? candidates[0].account : mangoAccount,
          mangoAccount,
          pm.perpMarketIndex,
        ),
      );
      ixs1.push(
        // Takes ~20k CU
        await this.perpSettleFeesIx(
          group,
          mangoAccount,
          pm.perpMarketIndex,
          undefined,
        ),
      );
    }

    const ixs2 = await Promise.all(
      mangoAccount.serum3Active().map((s) => {
        const serum3Market = group.getSerum3MarketByMarketIndex(s.marketIndex);
        // Takes ~65k CU
        return this.serum3SettleFundsV2Ix(
          group,
          mangoAccount,
          serum3Market.serumMarketExternal,
        );
      }),
    );

    if (
      mangoAccount.perpActive().length *
        (PERP_SETTLE_PNL_CU_LIMIT + PERP_SETTLE_FEES_CU_LIMIT) +
        mangoAccount.serum3Active().length * SERUM_SETTLE_FUNDS_CU_LIMIT >
      1600000
    ) {
      throw new Error(
        `Too many perp positions and serum open orders to settle in one tx! Please try settling individually!`,
      );
    }

    return await this.sendAndConfirmTransactionForGroup(
      group,
      [
        ComputeBudgetProgram.setComputeUnitLimit({
          units:
            mangoAccount.perpActive().length *
              (PERP_SETTLE_PNL_CU_LIMIT + PERP_SETTLE_FEES_CU_LIMIT) +
            mangoAccount.serum3Active().length * SERUM_SETTLE_FUNDS_CU_LIMIT,
        }),
        ...ixs1,
        ...ixs2,
      ],
      {
        estimateFee: true,
      },
    );
  }

  async perpSettlePnlAndFees(
    group: Group,
    profitableAccount: MangoAccount,
    unprofitableAccount: MangoAccount,
    accountToSettleFeesFor: MangoAccount,
    settler: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    maxSettleAmount?: number,
  ): Promise<MangoSignatureStatus> {
    return await this.sendAndConfirmTransactionForGroup(group, [
      ComputeBudgetProgram.setComputeUnitLimit({
        units: PERP_SETTLE_PNL_CU_LIMIT + PERP_SETTLE_FEES_CU_LIMIT,
      }),
      await this.perpSettlePnlIx(
        group,
        profitableAccount,
        unprofitableAccount,
        settler,
        perpMarketIndex,
      ),
      await this.perpSettleFeesIx(
        group,
        accountToSettleFeesFor,
        perpMarketIndex,
        maxSettleAmount,
      ),
    ]);
  }

  async perpSettlePnl(
    group: Group,
    profitableAccount: MangoAccount,
    unprofitableAccount: MangoAccount,
    settler: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
  ): Promise<MangoSignatureStatus> {
    return await this.sendAndConfirmTransactionForGroup(group, [
      ComputeBudgetProgram.setComputeUnitLimit({
        units: PERP_SETTLE_PNL_CU_LIMIT,
      }),
      await this.perpSettlePnlIx(
        group,
        profitableAccount,
        unprofitableAccount,
        settler,
        perpMarketIndex,
      ),
    ]);
  }

  async perpSettlePnlIx(
    group: Group,
    profitableAccount: MangoAccount,
    unprofitableAccount: MangoAccount,
    settler: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
  ): Promise<TransactionInstruction> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        group,
        [profitableAccount, unprofitableAccount],
        [group.getFirstBankForPerpSettlement()],
        [perpMarket],
      );
    const bank = group.banksMapByTokenIndex.get(0 as TokenIndex)![0];
    return await this.program.methods
      .perpSettlePnl()
      .accounts({
        group: group.publicKey,
        accountA: profitableAccount.publicKey,
        accountB: unprofitableAccount.publicKey,
        perpMarket: perpMarket.publicKey,
        oracle: perpMarket.oracle,
        settleOracle: bank.oracle,
        settleBank: bank.publicKey,
        settler: settler.publicKey,
        settlerOwner: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .instruction();
  }

  async perpSettleFees(
    group: Group,
    account: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    maxSettleAmount?: number,
  ): Promise<MangoSignatureStatus> {
    return await this.sendAndConfirmTransactionForGroup(group, [
      await this.perpSettleFeesIx(
        group,
        account,
        perpMarketIndex,
        maxSettleAmount,
      ),
    ]);
  }

  async perpSettleFeesIx(
    group: Group,
    account: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    maxSettleAmount?: number,
  ): Promise<TransactionInstruction> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        group,
        [account], // Account must be unprofitable
        [group.getFirstBankForPerpSettlement()],
        [perpMarket],
      );
    const bank = group.banksMapByTokenIndex.get(0 as TokenIndex)![0];
    return await this.program.methods
      .perpSettleFees(
        maxSettleAmount ? toNative(maxSettleAmount, 6) : RUST_U64_MAX(),
      )
      .accounts({
        group: group.publicKey,
        account: account.publicKey,
        perpMarket: perpMarket.publicKey,
        oracle: perpMarket.oracle,
        settleOracle: bank.oracle,
        settleBank: bank.publicKey,
      })
      .remainingAccounts(
        healthRemainingAccounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: false, isSigner: false } as AccountMeta),
        ),
      )
      .instruction();
  }

  public async perpConsumeEvents(
    group: Group,
    perpMarketIndex: PerpMarketIndex,
    accounts: PublicKey[],
    limit: number,
  ): Promise<MangoSignatureStatus> {
    return await this.sendAndConfirmTransactionForGroup(group, [
      await this.perpConsumeEventsIx(group, perpMarketIndex, accounts, limit),
    ]);
  }

  public async perpConsumeEventsIx(
    group: Group,
    perpMarketIndex: PerpMarketIndex,
    accounts: PublicKey[],
    limit: number,
  ): Promise<TransactionInstruction> {
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    return await this.program.methods
      .perpConsumeEvents(new BN(limit))
      .accounts({
        group: group.publicKey,
        perpMarket: perpMarket.publicKey,
        eventQueue: perpMarket.eventQueue,
      })
      .remainingAccounts(
        accounts.map(
          (pk) =>
            ({ pubkey: pk, isWritable: true, isSigner: false } as AccountMeta),
        ),
      )
      .instruction();
  }

  public async perpConsumeAllEvents(
    group: Group,
    perpMarketIndex: PerpMarketIndex,
  ): Promise<void> {
    const limit = 8;
    const perpMarket = group.getPerpMarketByMarketIndex(perpMarketIndex);
    const eventQueue = await perpMarket.loadEventQueue(this);
    const unconsumedEvents = eventQueue.getUnconsumedEvents();
    while (unconsumedEvents.length > 0) {
      const events = unconsumedEvents.splice(0, limit);
      const accounts = events
        .map((ev) => {
          switch (ev.eventType) {
            case PerpEventQueue.FILL_EVENT_TYPE: {
              const fill = <FillEvent>ev;
              return [fill.maker, fill.taker];
            }
            case PerpEventQueue.OUT_EVENT_TYPE: {
              const out = <OutEvent>ev;
              return [out.owner];
            }
            case PerpEventQueue.LIQUIDATE_EVENT_TYPE:
              return [];
            default:
              throw new Error(`Unknown event with eventType ${ev.eventType}!`);
          }
        })
        .flat();

      await this.perpConsumeEvents(group, perpMarketIndex, accounts, limit);
    }
  }

  public async perpUpdateFundingIx(
    group: Group,
    perpMarket: PerpMarket,
  ): Promise<TransactionInstruction> {
    return await this.program.methods
      .perpUpdateFunding()
      .accounts({
        group: group.publicKey,
        perpMarket: perpMarket.publicKey,
        bids: perpMarket.bids,
        asks: perpMarket.asks,
        oracle: perpMarket.oracle,
      })
      .instruction();
  }

  public async marginTrade({
    group,
    mangoAccount,
    inputMintPk,
    amountIn,
    outputMintPk,
    userDefinedInstructions,
    userDefinedAlts = [],
    // margin trade is a general function
    // set flash_loan_type to FlashLoanType.swap if you desire the transaction to be recorded as a swap
    flashLoanType,
  }: {
    group: Group;
    mangoAccount: MangoAccount;
    inputMintPk: PublicKey;
    amountIn: number;
    outputMintPk: PublicKey;
    userDefinedInstructions: TransactionInstruction[];
    userDefinedAlts: AddressLookupTableAccount[];
    flashLoanType: FlashLoanType;
  }): Promise<MangoSignatureStatus> {
    const isDelegate = (
      this.program.provider as AnchorProvider
    ).wallet.publicKey.equals(mangoAccount.delegate);
    const swapExecutingWallet = isDelegate
      ? mangoAccount.delegate
      : mangoAccount.owner;

    const inputBank: Bank = group.getFirstBankByMint(inputMintPk);
    const outputBank: Bank = group.getFirstBankByMint(outputMintPk);

    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        group,
        [mangoAccount],
        [inputBank, outputBank],
        [],
      );
    const parsedHealthAccounts = healthRemainingAccounts.map(
      (pk) =>
        ({
          pubkey: pk,
          isWritable: false,
          isSigner: false,
        } as AccountMeta),
    );

    /*
     * Find or create associated token accounts
     */
    const inputTokenAccountPk = await getAssociatedTokenAddress(
      inputBank.mint,
      swapExecutingWallet,
      true,
    );
    const inputTokenAccExists =
      await this.program.provider.connection.getAccountInfo(
        inputTokenAccountPk,
      );
    const preInstructions: TransactionInstruction[] = [];
    if (!inputTokenAccExists) {
      preInstructions.push(
        await createAssociatedTokenAccountIdempotentInstruction(
          swapExecutingWallet,
          swapExecutingWallet,
          inputBank.mint,
        ),
      );
    }

    const outputTokenAccountPk = await getAssociatedTokenAddress(
      outputBank.mint,
      swapExecutingWallet,
      true,
    );
    const outputTokenAccExists =
      await this.program.provider.connection.getAccountInfo(
        outputTokenAccountPk,
      );
    if (!outputTokenAccExists) {
      preInstructions.push(
        await createAssociatedTokenAccountIdempotentInstruction(
          swapExecutingWallet,
          swapExecutingWallet,
          outputBank.mint,
        ),
      );
    }

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
    const groupAM = {
      pubkey: group.publicKey,
      isWritable: false,
      isSigner: false,
    };

    const flashLoanEndIx = await this.program.methods
      .flashLoanEndV2(2, flashLoanType)
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
        groupAM,
      ])
      .instruction();

    const flashLoanBeginIx = await this.program.methods
      .flashLoanBegin([
        toNative(amountIn, inputBank.mintDecimals),
        new BN(
          0,
        ) /* we don't care about borrowing the target amount, this is just a dummy */,
      ])
      .accounts({
        account: mangoAccount.publicKey,
        owner: (this.program.provider as AnchorProvider).wallet.publicKey,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
      })
      .remainingAccounts([
        inputBankAccount,
        outputBankAccount,
        inputBankVault,
        outputBankVault,
        inputATA,
        outputATA,
        groupAM,
      ])
      .instruction();

    return await this.sendAndConfirmTransactionForGroup(
      group,
      [
        ...preInstructions,
        flashLoanBeginIx,
        ...userDefinedInstructions,
        flashLoanEndIx,
      ],
      { alts: [...group.addressLookupTablesList, ...userDefinedAlts] },
    );
  }

  public async tokenUpdateIndexAndRate(
    group: Group,
    mintPk: PublicKey,
  ): Promise<MangoSignatureStatus> {
    return await this.sendAndConfirmTransactionForGroup(group, [
      await this.tokenUpdateIndexAndRateIx(group, mintPk),
    ]);
  }

  public async tokenUpdateIndexAndRateIx(
    group: Group,
    mintPk: PublicKey,
  ): Promise<TransactionInstruction> {
    const bank = group.getFirstBankByMint(mintPk);
    const mintInfo = group.mintInfosMapByMint.get(mintPk.toString())!;

    return await this.program.methods
      .tokenUpdateIndexAndRate()
      .accounts({
        group: group.publicKey,
        mintInfo: mintInfo.publicKey,
        oracle: mintInfo.oracle,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
      })
      .remainingAccounts([
        {
          pubkey: bank.publicKey,
          isWritable: true,
          isSigner: false,
        } as AccountMeta,
      ])
      .instruction();
  }

  /// liquidations

  public async liqTokenWithToken(
    group: Group,
    liqor: MangoAccount,
    liqee: MangoAccount,
    assetMintPk: PublicKey,
    liabMintPk: PublicKey,
    maxLiabTransfer: number,
  ): Promise<MangoSignatureStatus> {
    const assetBank: Bank = group.getFirstBankByMint(assetMintPk);
    const liabBank: Bank = group.getFirstBankByMint(liabMintPk);

    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        group,
        [liqor, liqee],
        [assetBank, liabBank],
        [],
      );

    const parsedHealthAccounts = healthRemainingAccounts.map(
      (pk) =>
        ({
          pubkey: pk,
          isWritable:
            pk.equals(assetBank.publicKey) || pk.equals(liabBank.publicKey)
              ? true
              : false,
          isSigner: false,
        } as AccountMeta),
    );

    const ix = await this.program.methods
      .liqTokenWithToken(assetBank.tokenIndex, liabBank.tokenIndex, {
        val: I80F48.fromNumber(maxLiabTransfer).getData(),
      })
      .accounts({
        group: group.publicKey,
        liqor: liqor.publicKey,
        liqee: liqee.publicKey,
        liqorOwner: liqor.owner,
      })
      .remainingAccounts(parsedHealthAccounts)
      .instruction();

    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async tcsTakeProfitOnDeposit(
    group: Group,
    account: MangoAccount,
    sellBank: Bank,
    buyBank: Bank,
    thresholdPrice: number,
    thresholdPriceInSellPerBuyToken: boolean,
    maxSell: number | null,
    pricePremium: number | null,
    expiryTimestamp: number | null,
  ): Promise<MangoSignatureStatus> {
    const ixs = await this.tcsTakeProfitOnDepositIx(
      group,
      account,
      sellBank,
      buyBank,
      thresholdPrice,
      thresholdPriceInSellPerBuyToken,
      maxSell,
      pricePremium,
      expiryTimestamp,
    );
    return await this.sendAndConfirmTransactionForGroup(group, ixs);
  }

  public async tcsTakeProfitOnDepositIx(
    group: Group,
    account: MangoAccount,
    sellBank: Bank,
    buyBank: Bank,
    thresholdPrice: number,
    thresholdPriceInSellPerBuyToken: boolean,
    maxSell: number | null,
    pricePremium: number | null,
    expiryTimestamp: number | null,
  ): Promise<TransactionInstruction[]> {
    if (account.getTokenBalanceUi(sellBank) < 0) {
      throw new Error(
        `Only allowed to take profits on deposits! Current balance ${account.getTokenBalanceUi(
          sellBank,
        )}`,
      );
    }

    if (!thresholdPriceInSellPerBuyToken) {
      thresholdPrice = 1 / thresholdPrice;
    }
    const thresholdPriceNativeNative = toNativeSellPerBuyTokenPrice(
      thresholdPrice,
      sellBank,
      buyBank,
    );
    const lowerLimit = 0;
    const upperLimit = thresholdPriceNativeNative;

    return await this.tokenConditionalSwapCreateIx(
      group,
      account,
      sellBank,
      buyBank,
      lowerLimit,
      upperLimit,
      Number.MAX_SAFE_INTEGER,
      maxSell ?? account.getTokenBalanceUi(sellBank),
      'TakeProfitOnDeposit',
      pricePremium,
      true,
      false,
      expiryTimestamp,
      thresholdPriceInSellPerBuyToken,
    );
  }

  public async tcsStopLossOnDeposit(
    group: Group,
    account: MangoAccount,
    sellBank: Bank,
    buyBank: Bank,
    thresholdPrice: number,
    thresholdPriceInSellPerBuyToken: boolean,
    maxSell: number | null,
    pricePremium: number | null,
    expiryTimestamp: number | null,
  ): Promise<MangoSignatureStatus> {
    const ixs = await this.tcsStopLossOnDepositIx(
      group,
      account,
      sellBank,
      buyBank,
      thresholdPrice,
      thresholdPriceInSellPerBuyToken,
      maxSell,
      pricePremium,
      expiryTimestamp,
    );
    return await this.sendAndConfirmTransactionForGroup(group, ixs);
  }

  public async tcsStopLossOnDepositIx(
    group: Group,
    account: MangoAccount,
    sellBank: Bank,
    buyBank: Bank,
    thresholdPrice: number,
    thresholdPriceInSellPerBuyToken: boolean,
    maxSell: number | null,
    pricePremium: number | null,
    expiryTimestamp: number | null,
  ): Promise<TransactionInstruction[]> {
    if (account.getTokenBalanceUi(sellBank) < 0) {
      throw new Error(
        `Only allowed to set a stop loss on deposits! Current balance ${account.getTokenBalanceUi(
          sellBank,
        )}`,
      );
    }

    if (!thresholdPriceInSellPerBuyToken) {
      thresholdPrice = 1 / thresholdPrice;
    }
    const thresholdPriceNativeNative = toNativeSellPerBuyTokenPrice(
      thresholdPrice,
      sellBank,
      buyBank,
    );
    const lowerLimit = thresholdPriceNativeNative;
    const upperLimit = Number.MAX_SAFE_INTEGER;

    return await this.tokenConditionalSwapCreateIx(
      group,
      account,
      sellBank,
      buyBank,
      lowerLimit,
      upperLimit,
      Number.MAX_SAFE_INTEGER,
      maxSell ?? account.getTokenBalanceUi(sellBank),
      'StopLossOnDeposit',
      pricePremium,
      true,
      false,
      expiryTimestamp,
      thresholdPriceInSellPerBuyToken,
    );
  }

  public async tcsTakeProfitOnBorrow(
    group: Group,
    account: MangoAccount,
    sellBank: Bank,
    buyBank: Bank,
    thresholdPrice: number,
    thresholdPriceInSellPerBuyToken: boolean,
    maxBuyUi: number | null,
    pricePremium: number | null,
    allowMargin: boolean | null,
    expiryTimestamp: number | null,
  ): Promise<MangoSignatureStatus> {
    const ixs = await this.tcsTakeProfitOnBorrowIx(
      group,
      account,
      sellBank,
      buyBank,
      thresholdPrice,
      thresholdPriceInSellPerBuyToken,
      maxBuyUi,
      pricePremium,
      allowMargin,
      expiryTimestamp,
    );
    return await this.sendAndConfirmTransactionForGroup(group, ixs);
  }

  public async tcsTakeProfitOnBorrowIx(
    group: Group,
    account: MangoAccount,
    sellBank: Bank,
    buyBank: Bank,
    thresholdPrice: number,
    thresholdPriceInSellPerBuyToken: boolean,
    maxBuyUi: number | null,
    pricePremium: number | null,
    allowMargin: boolean | null,
    expiryTimestamp: number | null,
  ): Promise<TransactionInstruction[]> {
    if (account.getTokenBalanceUi(buyBank) > 0) {
      throw new Error(
        `Only allowed to take profits on borrows! Current balance ${account.getTokenBalanceUi(
          buyBank,
        )}`,
      );
    }

    if (!thresholdPriceInSellPerBuyToken) {
      thresholdPrice = 1 / thresholdPrice;
    }
    const thresholdPriceNativeNative = toNativeSellPerBuyTokenPrice(
      thresholdPrice,
      sellBank,
      buyBank,
    );
    const lowerLimit = 0;
    const upperLimit = thresholdPriceNativeNative;

    return await this.tokenConditionalSwapCreateIx(
      group,
      account,
      sellBank,
      buyBank,
      lowerLimit,
      upperLimit,
      maxBuyUi ?? -account.getTokenBalanceUi(buyBank),
      Number.MAX_SAFE_INTEGER,
      'TakeProfitOnBorrow',
      pricePremium,
      false,
      allowMargin ?? false,
      expiryTimestamp,
      thresholdPriceInSellPerBuyToken,
    );
  }

  public async tcsStopLossOnBorrow(
    group: Group,
    account: MangoAccount,
    sellBank: Bank,
    buyBank: Bank,
    thresholdPrice: number,
    thresholdPriceInSellPerBuyToken: boolean,
    maxBuyUi: number | null,
    pricePremium: number | null,
    allowMargin: boolean | null,
    expiryTimestamp: number | null,
  ): Promise<MangoSignatureStatus> {
    const ixs = await this.tcsStopLossOnBorrowIx(
      group,
      account,
      sellBank,
      buyBank,
      thresholdPrice,
      thresholdPriceInSellPerBuyToken,
      maxBuyUi,
      pricePremium,
      allowMargin,
      expiryTimestamp,
    );
    return await this.sendAndConfirmTransactionForGroup(group, ixs);
  }

  public async tcsStopLossOnBorrowIx(
    group: Group,
    account: MangoAccount,
    sellBank: Bank,
    buyBank: Bank,
    thresholdPrice: number,
    thresholdPriceInSellPerBuyToken: boolean,
    maxBuyUi: number | null,
    pricePremium: number | null,
    allowMargin: boolean | null,
    expiryTimestamp: number | null,
  ): Promise<TransactionInstruction[]> {
    if (account.getTokenBalanceUi(buyBank) > 0) {
      throw new Error(
        `Only allowed to set stop loss on borrows! Current balance ${account.getTokenBalanceUi(
          buyBank,
        )}`,
      );
    }

    if (!thresholdPriceInSellPerBuyToken) {
      thresholdPrice = 1 / thresholdPrice;
    }
    const thresholdPriceNativeNative = toNativeSellPerBuyTokenPrice(
      thresholdPrice,
      sellBank,
      buyBank,
    );
    const lowerLimit = thresholdPriceNativeNative;
    const upperLimit = Number.MAX_SAFE_INTEGER;

    return await this.tokenConditionalSwapCreateIx(
      group,
      account,
      sellBank,
      buyBank,
      lowerLimit,
      upperLimit,
      maxBuyUi ?? -account.getTokenBalanceUi(buyBank),
      Number.MAX_SAFE_INTEGER,
      'StopLossOnBorrow',
      pricePremium,
      false,
      allowMargin ?? false,
      expiryTimestamp,
      thresholdPriceInSellPerBuyToken,
    );
  }

  public async tokenConditionalSwapCreateIx(
    group: Group,
    account: MangoAccount,
    sellBank: Bank,
    buyBank: Bank,
    lowerLimitNativeNative: number,
    upperLimitNativeNative: number,
    maxBuy: number,
    maxSell: number,
    tcsIntention:
      | 'TakeProfitOnDeposit'
      | 'StopLossOnDeposit'
      | 'TakeProfitOnBorrow'
      | 'StopLossOnBorrow'
      | null,
    pricePremium: number | null,
    allowCreatingDeposits: boolean,
    allowCreatingBorrows: boolean,
    expiryTimestamp: number | null,
    displayPriceInSellTokenPerBuyToken: boolean,
  ): Promise<TransactionInstruction[]> {
    const maxBuyNative =
      maxBuy == Number.MAX_SAFE_INTEGER
        ? U64_MAX_BN
        : toNative(maxBuy, buyBank.mintDecimals);
    const maxSellNative =
      maxSell == Number.MAX_SAFE_INTEGER
        ? U64_MAX_BN
        : toNative(maxSell, sellBank.mintDecimals);
    pricePremium = TokenConditionalSwap.computePremium(
      group,
      buyBank,
      sellBank,
      maxBuyNative,
      maxSellNative,
      maxBuy,
      maxSell,
    );
    const pricePremiumRate = pricePremium > 0 ? pricePremium / 100 : 0.03;

    let intention: TokenConditionalSwapIntention;
    switch (tcsIntention) {
      case 'StopLossOnBorrow':
      case 'StopLossOnDeposit':
        intention = TokenConditionalSwapIntention.stopLoss;
        break;
      case 'TakeProfitOnBorrow':
      case 'TakeProfitOnDeposit':
        intention = TokenConditionalSwapIntention.takeProfit;
        break;
      default:
        intention = TokenConditionalSwapIntention.unknown;
        break;
    }

    return await this.tokenConditionalSwapCreateRawIx(
      group,
      account,
      buyBank.mint,
      sellBank.mint,
      maxBuyNative,
      maxSellNative,
      expiryTimestamp,
      lowerLimitNativeNative,
      upperLimitNativeNative,
      pricePremiumRate,
      allowCreatingDeposits,
      allowCreatingBorrows,
      displayPriceInSellTokenPerBuyToken
        ? TokenConditionalSwapDisplayPriceStyle.sellTokenPerBuyToken
        : TokenConditionalSwapDisplayPriceStyle.buyTokenPerSellToken,
      intention,
    );
  }

  public async tokenConditionalSwapCreate(
    group: Group,
    account: MangoAccount,
    sellBank: Bank,
    buyBank: Bank,
    lowerLimitNativeNative: number,
    upperLimitNativeNative: number,
    maxBuy: number,
    maxSell: number,
    tcsIntention:
      | 'TakeProfitOnDeposit'
      | 'StopLossOnDeposit'
      | 'TakeProfitOnBorrow'
      | 'StopLossOnBorrow'
      | null,
    pricePremium: number | null,
    allowCreatingDeposits: boolean,
    allowCreatingBorrows: boolean,
    expiryTimestamp: number | null,
    displayPriceInSellTokenPerBuyToken: boolean,
  ): Promise<MangoSignatureStatus> {
    const maxBuyNative =
      maxBuy == Number.MAX_SAFE_INTEGER
        ? U64_MAX_BN
        : toNative(maxBuy, buyBank.mintDecimals);
    const maxSellNative =
      maxSell == Number.MAX_SAFE_INTEGER
        ? U64_MAX_BN
        : toNative(maxSell, sellBank.mintDecimals);
    pricePremium = TokenConditionalSwap.computePremium(
      group,
      buyBank,
      sellBank,
      maxBuyNative,
      maxSellNative,
      maxBuy,
      maxSell,
    );
    const pricePremiumRate = pricePremium > 0 ? pricePremium / 100 : 0.03;

    let intention: TokenConditionalSwapIntention;
    switch (tcsIntention) {
      case 'StopLossOnBorrow':
      case 'StopLossOnDeposit':
        intention = TokenConditionalSwapIntention.stopLoss;
        break;
      case 'TakeProfitOnBorrow':
      case 'TakeProfitOnDeposit':
        intention = TokenConditionalSwapIntention.takeProfit;
        break;
      default:
        intention = TokenConditionalSwapIntention.unknown;
        break;
    }

    return await this.tokenConditionalSwapCreateRaw(
      group,
      account,
      buyBank.mint,
      sellBank.mint,
      maxBuyNative,
      maxSellNative,
      expiryTimestamp,
      lowerLimitNativeNative,
      upperLimitNativeNative,
      pricePremiumRate,
      allowCreatingDeposits,
      allowCreatingBorrows,
      displayPriceInSellTokenPerBuyToken
        ? TokenConditionalSwapDisplayPriceStyle.sellTokenPerBuyToken
        : TokenConditionalSwapDisplayPriceStyle.buyTokenPerSellToken,
      intention,
    );
  }

  public async tokenConditionalSwapCreateLinearAuctionIx(
    group: Group,
    account: MangoAccount,
    sellBank: Bank,
    buyBank: Bank,
    priceStart: number,
    priceEnd: number,
    maxBuy: number,
    maxSell: number,
    allowCreatingDeposits: boolean,
    allowCreatingBorrows: boolean,
    displayPriceInSellTokenPerBuyToken: boolean,
    startTimestamp: number,
    durationSeconds: number,
    expiryTimestamp: number | null,
  ): Promise<TransactionInstruction[]> {
    let maxBuyNative, maxSellNative;
    if (maxBuy == Number.MAX_SAFE_INTEGER) {
      maxBuyNative = U64_MAX_BN;
    } else {
      maxBuyNative = toNative(maxBuy, buyBank.mintDecimals);
    }
    if (maxSell == Number.MAX_SAFE_INTEGER) {
      maxSellNative = U64_MAX_BN;
    } else {
      maxSellNative = toNative(maxSell, sellBank.mintDecimals);
    }

    const priceStartNative = toNativeSellPerBuyTokenPrice(
      priceStart,
      sellBank,
      buyBank,
    );
    const priceEndNative = toNativeSellPerBuyTokenPrice(
      priceEnd,
      sellBank,
      buyBank,
    );

    const tcsIx = await this.program.methods
      .tokenConditionalSwapCreateLinearAuction(
        maxBuyNative,
        maxSellNative,
        expiryTimestamp !== null ? new BN(expiryTimestamp) : U64_MAX_BN,
        priceStartNative,
        priceEndNative,
        allowCreatingDeposits,
        allowCreatingBorrows,
        displayPriceInSellTokenPerBuyToken
          ? TokenConditionalSwapDisplayPriceStyle.sellTokenPerBuyToken
          : TokenConditionalSwapDisplayPriceStyle.buyTokenPerSellToken,
        new BN(startTimestamp),
        new BN(durationSeconds),
      )
      .accounts({
        group: group.publicKey,
        account: account.publicKey,
        authority: (this.program.provider as AnchorProvider).wallet.publicKey,
        buyBank: buyBank.publicKey,
        sellBank: sellBank.publicKey,
      })
      .instruction();

    const ixs: TransactionInstruction[] = [];
    if (account.tokenConditionalSwaps.length == 0) {
      ixs.push(
        await this.accountExpandV2Ix(
          group,
          account,
          account.tokens.length,
          account.serum3.length,
          account.perps.length,
          account.perpOpenOrders.length,
          DEFAULT_TOKEN_CONDITIONAL_SWAP_COUNT,
        ),
      );
    }
    ixs.push(tcsIx);

    return ixs;
  }

  public async tokenConditionalSwapCreateLinearAuction(
    group: Group,
    account: MangoAccount,
    sellBank: Bank,
    buyBank: Bank,
    priceStart: number,
    priceEnd: number,
    maxBuy: number,
    maxSell: number,
    allowCreatingDeposits: boolean,
    allowCreatingBorrows: boolean,
    displayPriceInSellTokenPerBuyToken: boolean,
    startTimestamp: number,
    durationSeconds: number,
    expiryTimestamp: number | null,
  ): Promise<MangoSignatureStatus> {
    const ixs = await this.tokenConditionalSwapCreateLinearAuctionIx(
      group,
      account,
      sellBank,
      buyBank,
      priceStart,
      priceEnd,
      maxBuy,
      maxSell,
      allowCreatingDeposits,
      allowCreatingBorrows,
      displayPriceInSellTokenPerBuyToken,
      startTimestamp,
      durationSeconds,
      expiryTimestamp,
    );
    return await this.sendAndConfirmTransactionForGroup(group, ixs);
  }

  public async tokenConditionalSwapCreatePremiumAuctionIx(
    group: Group,
    account: MangoAccount,
    sellBank: Bank,
    buyBank: Bank,
    lowerLimit: number,
    upperLimit: number,
    maxBuy: number,
    maxSell: number,
    tcsIntention:
      | 'TakeProfitOnDeposit'
      | 'StopLossOnDeposit'
      | 'TakeProfitOnBorrow'
      | 'StopLossOnBorrow'
      | null,
    maxPricePremiumPercent: number | null,
    allowCreatingDeposits: boolean,
    allowCreatingBorrows: boolean,
    expiryTimestamp: number | null,
    displayPriceInSellTokenPerBuyToken: boolean,
    durationSeconds: number,
  ): Promise<TransactionInstruction[]> {
    const lowerLimitNative = toNativeSellPerBuyTokenPrice(
      lowerLimit,
      sellBank,
      buyBank,
    );
    const upperLimitNative = toNativeSellPerBuyTokenPrice(
      upperLimit,
      sellBank,
      buyBank,
    );

    let maxBuyNative, maxSellNative, buyAmountInUsd, sellAmountInUsd;
    if (maxBuy == Number.MAX_SAFE_INTEGER) {
      maxBuyNative = U64_MAX_BN;
    } else {
      buyAmountInUsd = maxBuy * buyBank.uiPrice;
      maxBuyNative = toNative(maxBuy, buyBank.mintDecimals);
    }
    if (maxSell == Number.MAX_SAFE_INTEGER) {
      maxSellNative = U64_MAX_BN;
    } else {
      sellAmountInUsd = maxSell * sellBank.uiPrice;
      maxSellNative = toNative(maxSell, sellBank.mintDecimals);
    }

    // Used for computing optimal premium
    let liqorTcsChunkSizeInUsd = Math.min(buyAmountInUsd, sellAmountInUsd);
    if (liqorTcsChunkSizeInUsd > 5000) {
      liqorTcsChunkSizeInUsd = 5000;
    }
    // For small TCS swaps, reduce chunk size to 1000 USD
    else {
      liqorTcsChunkSizeInUsd = 1000;
    }

    // TODO: The max premium should likely be computed differently
    if (!maxPricePremiumPercent) {
      const buyTokenPriceImpact = group.getPriceImpactByTokenIndex(
        buyBank.tokenIndex,
        liqorTcsChunkSizeInUsd,
      );
      const sellTokenPriceImpact = group.getPriceImpactByTokenIndex(
        sellBank.tokenIndex,
        liqorTcsChunkSizeInUsd,
      );
      maxPricePremiumPercent =
        ((1 + buyTokenPriceImpact / 100) * (1 + sellTokenPriceImpact / 100) -
          1) *
        100;
    }
    const maxPricePremiumRate =
      maxPricePremiumPercent > 0 ? maxPricePremiumPercent / 100 : 0.03;

    let intention: TokenConditionalSwapIntention;
    switch (tcsIntention) {
      case 'StopLossOnBorrow':
      case 'StopLossOnDeposit':
        intention = TokenConditionalSwapIntention.stopLoss;
        break;
      case 'TakeProfitOnBorrow':
      case 'TakeProfitOnDeposit':
        intention = TokenConditionalSwapIntention.takeProfit;
        break;
      default:
        intention = TokenConditionalSwapIntention.unknown;
        break;
    }

    const tcsIx = await this.program.methods
      .tokenConditionalSwapCreatePremiumAuction(
        maxBuyNative,
        maxSellNative,
        expiryTimestamp !== null ? new BN(expiryTimestamp) : U64_MAX_BN,
        lowerLimitNative,
        upperLimitNative,
        maxPricePremiumRate,
        allowCreatingDeposits,
        allowCreatingBorrows,
        displayPriceInSellTokenPerBuyToken
          ? TokenConditionalSwapDisplayPriceStyle.sellTokenPerBuyToken
          : TokenConditionalSwapDisplayPriceStyle.buyTokenPerSellToken,
        intention,
        new BN(durationSeconds),
      )
      .accounts({
        group: group.publicKey,
        account: account.publicKey,
        authority: (this.program.provider as AnchorProvider).wallet.publicKey,
        buyBank: buyBank.publicKey,
        sellBank: sellBank.publicKey,
      })
      .instruction();

    const ixs: TransactionInstruction[] = [];
    if (account.tokenConditionalSwaps.length == 0) {
      ixs.push(
        await this.accountExpandV2Ix(
          group,
          account,
          account.tokens.length,
          account.serum3.length,
          account.perps.length,
          account.perpOpenOrders.length,
          DEFAULT_TOKEN_CONDITIONAL_SWAP_COUNT,
        ),
      );
    }
    ixs.push(tcsIx);

    return ixs;
  }
  public async tokenConditionalSwapCreatePremiumAuction(
    group: Group,
    account: MangoAccount,
    sellBank: Bank,
    buyBank: Bank,
    lowerLimit: number,
    upperLimit: number,
    maxBuy: number,
    maxSell: number,
    tcsIntention:
      | 'TakeProfitOnDeposit'
      | 'StopLossOnDeposit'
      | 'TakeProfitOnBorrow'
      | 'StopLossOnBorrow'
      | null,
    maxPricePremiumPercent: number | null,
    allowCreatingDeposits: boolean,
    allowCreatingBorrows: boolean,
    expiryTimestamp: number | null,
    displayPriceInSellTokenPerBuyToken: boolean,
    durationSeconds: number,
  ): Promise<MangoSignatureStatus> {
    const ixs = await this.tokenConditionalSwapCreatePremiumAuctionIx(
      group,
      account,
      sellBank,
      buyBank,
      lowerLimit,
      upperLimit,
      maxBuy,
      maxSell,
      tcsIntention,
      maxPricePremiumPercent,
      allowCreatingDeposits,
      allowCreatingBorrows,
      expiryTimestamp,
      displayPriceInSellTokenPerBuyToken,
      durationSeconds,
    );
    return await this.sendAndConfirmTransactionForGroup(group, ixs);
  }

  public async tokenConditionalSwapCreateRaw(
    group: Group,
    account: MangoAccount,
    buyMintPk: PublicKey,
    sellMintPk: PublicKey,
    maxBuy: BN,
    maxSell: BN,
    expiryTimestamp: number | null,
    priceLowerLimit: number,
    priceUpperLimit: number,
    pricePremiumRate: number,
    allowCreatingDeposits: boolean,
    allowCreatingBorrows: boolean,
    priceDisplayStyle: TokenConditionalSwapDisplayPriceStyle,
    intention: TokenConditionalSwapIntention,
  ): Promise<MangoSignatureStatus> {
    const ixs = await this.tokenConditionalSwapCreateRawIx(
      group,
      account,
      buyMintPk,
      sellMintPk,
      maxBuy,
      maxSell,
      expiryTimestamp,
      priceLowerLimit,
      priceUpperLimit,
      pricePremiumRate,
      allowCreatingDeposits,
      allowCreatingBorrows,
      priceDisplayStyle,
      intention,
    );
    return await this.sendAndConfirmTransactionForGroup(group, ixs);
  }

  public async tokenConditionalSwapCreateRawIx(
    group: Group,
    account: MangoAccount,
    buyMintPk: PublicKey,
    sellMintPk: PublicKey,
    maxBuy: BN,
    maxSell: BN,
    expiryTimestamp: number | null,
    priceLowerLimit: number,
    priceUpperLimit: number,
    pricePremiumRate: number,
    allowCreatingDeposits: boolean,
    allowCreatingBorrows: boolean,
    priceDisplayStyle: TokenConditionalSwapDisplayPriceStyle,
    intention: TokenConditionalSwapIntention,
  ): Promise<TransactionInstruction[]> {
    const buyBank: Bank = group.getFirstBankByMint(buyMintPk);
    const sellBank: Bank = group.getFirstBankByMint(sellMintPk);
    const tcsIx = await this.program.methods
      .tokenConditionalSwapCreateV2(
        maxBuy,
        maxSell,
        expiryTimestamp !== null ? new BN(expiryTimestamp) : U64_MAX_BN,
        priceLowerLimit,
        priceUpperLimit,
        pricePremiumRate,
        allowCreatingDeposits,
        allowCreatingBorrows,
        priceDisplayStyle,
        intention,
      )
      .accounts({
        group: group.publicKey,
        account: account.publicKey,
        authority: (this.program.provider as AnchorProvider).wallet.publicKey,
        buyBank: buyBank.publicKey,
        sellBank: sellBank.publicKey,
      })
      .instruction();

    const ixs: TransactionInstruction[] = [];
    if (account.tokenConditionalSwaps.length == 0) {
      ixs.push(
        await this.accountExpandV2Ix(
          group,
          account,
          account.tokens.length,
          account.serum3.length,
          account.perps.length,
          account.perpOpenOrders.length,
          DEFAULT_TOKEN_CONDITIONAL_SWAP_COUNT,
        ),
      );
    }
    ixs.push(tcsIx);

    return ixs;
  }

  public async tokenConditionalSwapCancelIx(
    group: Group,
    account: MangoAccount,
    tokenConditionalSwapId: BN,
  ): Promise<TransactionInstruction> {
    const tokenConditionalSwapIndex = account.tokenConditionalSwaps.findIndex(
      (tcs) => tcs.id.eq(tokenConditionalSwapId),
    );
    if (tokenConditionalSwapIndex == -1) {
      throw new Error('tcs with id not found');
    }
    const tcs = account.tokenConditionalSwaps[tokenConditionalSwapIndex];

    const buyBank = group.banksMapByTokenIndex.get(tcs.buyTokenIndex)![0];
    const sellBank = group.banksMapByTokenIndex.get(tcs.sellTokenIndex)![0];

    return this.program.methods
      .tokenConditionalSwapCancel(
        tokenConditionalSwapIndex,
        new BN(tokenConditionalSwapId),
      )
      .accounts({
        group: group.publicKey,
        account: account.publicKey,
        authority: (this.program.provider as AnchorProvider).wallet.publicKey,
        buyBank: buyBank.publicKey,
        sellBank: sellBank.publicKey,
      })
      .instruction();
  }

  public async tokenConditionalSwapCancel(
    group: Group,
    account: MangoAccount,
    tokenConditionalSwapId: BN,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.tokenConditionalSwapCancelIx(
      group,
      account,
      tokenConditionalSwapId,
    );
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async tokenConditionalSwapCancelAll(
    group: Group,
    account: MangoAccount,
  ): Promise<MangoSignatureStatus> {
    const ixs = await Promise.all(
      account.tokenConditionalSwaps
        .filter((tcs) => tcs.isConfigured)
        .map(async (tcs, i) => {
          const buyBank = group.banksMapByTokenIndex.get(tcs.buyTokenIndex)![0];
          const sellBank = group.banksMapByTokenIndex.get(
            tcs.sellTokenIndex,
          )![0];
          return await this.program.methods
            .tokenConditionalSwapCancel(i, new BN(tcs.id))
            .accounts({
              group: group.publicKey,
              account: account.publicKey,
              authority: (this.program.provider as AnchorProvider).wallet
                .publicKey,
              buyBank: buyBank.publicKey,
              sellBank: sellBank.publicKey,
            })
            .instruction();
        }),
    );

    return await this.sendAndConfirmTransactionForGroup(group, ixs);
  }

  public async tokenConditionalSwapTrigger(
    group: Group,
    liqee: MangoAccount,
    liqor: MangoAccount,
    tokenConditionalSwapId: BN,
    maxBuyTokenToLiqee: number,
    maxSellTokenToLiqor: number,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.tokenConditionalSwapTriggerIx(
      group,
      liqee,
      liqor,
      tokenConditionalSwapId,
      maxBuyTokenToLiqee,
      maxSellTokenToLiqor,
    );

    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async tokenConditionalSwapTriggerIx(
    group: Group,
    liqee: MangoAccount,
    liqor: MangoAccount,
    tokenConditionalSwapId: BN,
    maxBuyTokenToLiqee: number,
    maxSellTokenToLiqor: number,
  ): Promise<TransactionInstruction> {
    const tokenConditionalSwapIndex = liqee.tokenConditionalSwaps.findIndex(
      (tcs) => tcs.id.eq(tokenConditionalSwapId),
    );
    if (tokenConditionalSwapIndex == -1) {
      throw new Error('tcs with id not found');
    }
    const tcs = liqee.tokenConditionalSwaps[tokenConditionalSwapIndex];

    const buyBank = group.banksMapByTokenIndex.get(tcs.buyTokenIndex)![0];
    const sellBank = group.banksMapByTokenIndex.get(tcs.sellTokenIndex)![0];

    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        group,
        [liqor, liqee],
        [buyBank, sellBank],
        [],
      );

    const parsedHealthAccounts = healthRemainingAccounts.map(
      (pk) =>
        ({
          pubkey: pk,
          isWritable:
            pk.equals(buyBank.publicKey) || pk.equals(sellBank.publicKey)
              ? true
              : false,
          isSigner: false,
        } as AccountMeta),
    );

    return this.program.methods
      .tokenConditionalSwapTrigger(
        tokenConditionalSwapIndex,
        new BN(tokenConditionalSwapId),
        new BN(maxBuyTokenToLiqee),
        new BN(maxSellTokenToLiqor),
      )
      .accounts({
        group: group.publicKey,
        liqee: liqee.publicKey,
        liqor: liqor.publicKey,
        liqorAuthority: (this.program.provider as AnchorProvider).wallet
          .publicKey,
      })
      .remainingAccounts(parsedHealthAccounts)
      .instruction();
  }

  public async altSet(
    group: Group,
    addressLookupTable: PublicKey,
    index: number,
  ): Promise<MangoSignatureStatus> {
    const ix = await this.program.methods
      .altSet(index)
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        addressLookupTable,
      })
      .instruction();

    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async altExtend(
    group: Group,
    addressLookupTable: PublicKey,
    index: number,
    pks: PublicKey[],
  ): Promise<MangoSignatureStatus> {
    const ix = await this.program.methods
      .altExtend(index, pks)
      .accounts({
        group: group.publicKey,
        admin: (this.program.provider as AnchorProvider).wallet.publicKey,
        payer: (this.program.provider as AnchorProvider).wallet.publicKey,
        addressLookupTable,
      })
      .instruction();
    return await this.sendAndConfirmTransactionForGroup(group, [ix]);
  }

  public async healthRegionBeginIx(
    group: Group,
    account: MangoAccount,
    banks: Bank[] = [],
    perpMarkets: PerpMarket[] = [],
  ): Promise<TransactionInstruction> {
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        group,
        [account],
        [...banks],
        [...perpMarkets],
      );
    const parsedHealthAccounts = healthRemainingAccounts.map(
      (pk) =>
        ({
          pubkey: pk,
          isWritable: false,
          isSigner: false,
        } as AccountMeta),
    );

    return await this.program.methods
      .healthRegionBegin()
      .accounts({
        group: group.publicKey,
        account: account.publicKey,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
      })
      .remainingAccounts(parsedHealthAccounts)
      .instruction();
  }

  public async healthRegionEndIx(
    group: Group,
    account: MangoAccount,
    banks: Bank[] = [],
    perpMarkets: PerpMarket[] = [],
  ): Promise<TransactionInstruction> {
    const healthRemainingAccounts: PublicKey[] =
      this.buildHealthRemainingAccounts(
        group,
        [account],
        [...banks],
        [...perpMarkets],
      );
    const parsedHealthAccounts = healthRemainingAccounts.map(
      (pk) =>
        ({
          pubkey: pk,
          isWritable: false,
          isSigner: false,
        } as AccountMeta),
    );

    return await this.program.methods
      .healthRegionEnd()
      .accounts({ account: account.publicKey })
      .remainingAccounts(parsedHealthAccounts)
      .instruction();
  }

  /// static

  static connect(
    provider: Provider,
    cluster: Cluster,
    programId: PublicKey,
    opts?: MangoClientOptions,
  ): MangoClient {
    const idl = IDL;

    return new MangoClient(
      new Program<MangoV4>(idl as MangoV4, programId, provider),
      programId,
      cluster,
      opts,
    );
  }

  /**
   * Connect with defaults,
   *  - random ephemeral keypair,
   *  - fetch ids using gPa
   *  - connects to mainnet-beta
   *  - uses well known program Id
   * @param clusterUrl
   * @returns
   */
  static connectDefault(clusterUrl: string): MangoClient {
    const idl = IDL;

    const options = AnchorProvider.defaultOptions();
    const connection = new Connection(clusterUrl, options);

    return new MangoClient(
      new Program<MangoV4>(
        idl as MangoV4,
        MANGO_V4_ID['mainnet-beta'],
        new AnchorProvider(connection, new Wallet(new Keypair()), options),
      ),
      MANGO_V4_ID['mainnet-beta'],
      'mainnet-beta' as Cluster,
      {
        idsSource: 'get-program-accounts',
      },
    );
  }

  static connectForGroupName(
    provider: Provider,
    groupName: string,
  ): MangoClient {
    const idl = IDL;

    const id = Id.fromIdsByName(groupName);

    return new MangoClient(
      new Program<MangoV4>(
        idl as MangoV4,
        new PublicKey(id.mangoProgramId),
        provider,
      ),
      new PublicKey(id.mangoProgramId),
      id.cluster,
    );
  }

  /**
   * Builds health remaining accounts.
   *
   * For single mango account it builds a list of PublicKeys
   * which is compatbile with Fixed account retriever.
   *
   * For multiple mango accounts it uses same logic as for fixed
   * but packing all banks, then perp markets, and then serum oo accounts, which
   * should always be compatible with Scanning account retriever.
   *
   * @param group
   * @param mangoAccounts
   * @param banks - banks in which new positions might be opened
   * @param perpMarkets - markets in which new positions might be opened
   * @param openOrdersForMarket - markets in which new positions might be opened
   * @returns
   */
  buildHealthRemainingAccounts(
    group: Group,
    mangoAccounts: MangoAccount[],
    // Banks and markets for whom positions don't exist on mango account,
    // but user would potentially open new positions.
    banks: Bank[] = [],
    perpMarkets: PerpMarket[] = [],
    openOrdersForMarket: [Serum3Market, PublicKey][] = [],
  ): PublicKey[] {
    const healthRemainingAccounts: PublicKey[] = [];

    const tokenPositionIndices = mangoAccounts
      .map((mangoAccount) => mangoAccount.tokens.map((t) => t.tokenIndex))
      .flat();
    for (const bank of banks) {
      const tokenPositionExists =
        tokenPositionIndices.indexOf(bank.tokenIndex) > -1;
      if (!tokenPositionExists) {
        const inactiveTokenPosition = tokenPositionIndices.findIndex(
          (index) => index === TokenPosition.TokenIndexUnset,
        );
        if (inactiveTokenPosition != -1) {
          tokenPositionIndices[inactiveTokenPosition] = bank.tokenIndex;
        } else {
          throw new Error(
            `All token positions are occupied, either expand mango account, or close an existing token position, e.g. by withdrawing token deposits, or repaying all borrows!`,
          );
        }
      }
    }
    const mintInfos = uniq(
      tokenPositionIndices
        .filter((tokenIndex) => tokenIndex !== TokenPosition.TokenIndexUnset)
        .map((tokenIndex) => group.mintInfosMapByTokenIndex.get(tokenIndex)!),
      (mintInfo) => {
        mintInfo.tokenIndex;
      },
    );
    healthRemainingAccounts.push(
      ...mintInfos.map((mintInfo) => mintInfo.firstBank()),
    );
    healthRemainingAccounts.push(
      ...mintInfos.map((mintInfo) => mintInfo.oracle),
    );

    // Insert any extra perp markets in the free perp position slots
    const perpPositionsMarketIndices = mangoAccounts
      .map((mangoAccount) => mangoAccount.perps.map((p) => p.marketIndex))
      .flat();
    for (const perpMarket of perpMarkets) {
      const perpPositionExists =
        perpPositionsMarketIndices.indexOf(perpMarket.perpMarketIndex) > -1;
      if (!perpPositionExists) {
        const inactivePerpPosition = perpPositionsMarketIndices.findIndex(
          (perpIdx) => perpIdx === PerpPosition.PerpMarketIndexUnset,
        );
        if (inactivePerpPosition != -1) {
          perpPositionsMarketIndices[inactivePerpPosition] =
            perpMarket.perpMarketIndex;
        }
      }
    }
    const allPerpMarkets = uniq(
      perpPositionsMarketIndices
        .filter(
          (perpMarktIndex) =>
            perpMarktIndex !== PerpPosition.PerpMarketIndexUnset,
        )
        .map((perpIdx) => group.getPerpMarketByMarketIndex(perpIdx)!),
      (pm) => pm.perpMarketIndex,
    );
    healthRemainingAccounts.push(
      ...allPerpMarkets.map((perp) => perp.publicKey),
    );
    healthRemainingAccounts.push(...allPerpMarkets.map((perp) => perp.oracle));

    // Insert any extra open orders accounts in the cooresponding free serum market slot
    const serumPositionMarketIndices = mangoAccounts
      .map((mangoAccount) =>
        mangoAccount.serum3.map((s) => ({
          marketIndex: s.marketIndex,
          openOrders: s.openOrders,
        })),
      )
      .flat();
    for (const [serum3Market, openOrderPk] of openOrdersForMarket) {
      const ooPositionExists =
        serumPositionMarketIndices.findIndex(
          (i) => i.marketIndex === serum3Market.marketIndex,
        ) > -1;
      if (!ooPositionExists) {
        const inactiveSerumPosition = serumPositionMarketIndices.findIndex(
          (serumPos) =>
            serumPos.marketIndex === Serum3Orders.Serum3MarketIndexUnset,
        );
        if (inactiveSerumPosition != -1) {
          serumPositionMarketIndices[inactiveSerumPosition].marketIndex =
            serum3Market.marketIndex;
          serumPositionMarketIndices[inactiveSerumPosition].openOrders =
            openOrderPk;
        }
      }
    }

    healthRemainingAccounts.push(
      ...serumPositionMarketIndices
        .filter(
          (serumPosition) =>
            serumPosition.marketIndex !== Serum3Orders.Serum3MarketIndexUnset,
        )
        .map((serumPosition) => serumPosition.openOrders),
    );

    return healthRemainingAccounts;
  }

  public async modifyPerpOrder(
    group: Group,
    mangoAccount: MangoAccount,
    perpMarketIndex: PerpMarketIndex,
    orderId: BN,
    side: PerpOrderSide,
    price: number,
    quantity: number,
    maxQuoteQuantity?: number,
    clientOrderId?: number,
    orderType?: PerpOrderType,
    reduceOnly?: boolean,
    expiryTimestamp?: number,
    limit?: number,
  ): Promise<MangoSignatureStatus> {
    const transactionInstructions: TransactionInstruction[] = [];
    const [cancelOrderIx, placeOrderIx] = await Promise.all([
      this.perpCancelOrderIx(group, mangoAccount, perpMarketIndex, orderId),
      this.perpPlaceOrderIx(
        group,
        mangoAccount,
        perpMarketIndex,
        side,
        price,
        quantity,
        maxQuoteQuantity,
        clientOrderId,
        orderType,
        reduceOnly,
        expiryTimestamp,
        limit,
      ),
    ]);
    transactionInstructions.push(cancelOrderIx, placeOrderIx);

    return await this.sendAndConfirmTransactionForGroup(
      group,
      transactionInstructions,
    );
  }
  public async modifySerum3Order(
    group: Group,
    orderId: BN,
    mangoAccount: MangoAccount,
    externalMarketPk: PublicKey,
    side: Serum3Side,
    price: number,
    size: number,
    selfTradeBehavior: Serum3SelfTradeBehavior,
    orderType: Serum3OrderType,
    clientOrderId: number,
    limit: number,
  ): Promise<MangoSignatureStatus> {
    const transactionInstructions: TransactionInstruction[] = [];
    const [cancelOrderIx, settleIx, placeOrderIx] = await Promise.all([
      this.serum3CancelOrderIx(
        group,
        mangoAccount,
        externalMarketPk,
        side,
        orderId,
      ),
      this.serum3SettleFundsV2Ix(group, mangoAccount, externalMarketPk),
      this.serum3PlaceOrderV2Ix(
        group,
        mangoAccount,
        externalMarketPk,
        side,
        price,
        size,
        selfTradeBehavior,
        orderType,
        clientOrderId,
        limit,
      ),
    ]);
    transactionInstructions.push(cancelOrderIx, settleIx, ...placeOrderIx);

    return await this.sendAndConfirmTransactionForGroup(
      group,
      transactionInstructions,
    );
  }

  /**
   * Returns an estimate of a prioritization fee for a set of instructions.
   *
   * The estimate is based on the median fees of writable accounts that will be involved in the transaction.
   *
   * @param ixs - the instructions that make up the transaction
   * @returns prioritizationFeeEstimate -- in microLamports
   */
  public async estimatePrioritizationFee(
    ixs: TransactionInstruction[],
  ): Promise<number> {
    const writableAccounts = ixs
      .map((x) => x.keys.filter((a) => a.isWritable).map((k) => k.pubkey))
      .flat();
    const uniqueWritableAccounts = uniq(
      writableAccounts.map((x) => x.toBase58()),
    )
      .map((a) => new PublicKey(a))
      .slice(0, MAX_RECENT_PRIORITY_FEE_ACCOUNTS);

    const priorityFees = await this.connection.getRecentPrioritizationFees({
      lockedWritableAccounts: uniqueWritableAccounts,
    });

    if (priorityFees.length < 1) {
      return 1;
    }

    // get max priority fee per slot (and sort by slot from old to new)
    const maxFeeBySlot = mapValues(groupBy(priorityFees, 'slot'), (items) =>
      maxBy(items, 'prioritizationFee'),
    );
    const maximumFees = Object.values(maxFeeBySlot).sort(
      (a: RecentPrioritizationFees, b: RecentPrioritizationFees) =>
        a.slot - b.slot,
    ) as RecentPrioritizationFees[];

    // get median of last 20 fees
    const recentFees = maximumFees.slice(Math.max(maximumFees.length - 20, 0));
    const mid = Math.floor(recentFees.length / 2);
    const medianFee =
      recentFees.length % 2 !== 0
        ? recentFees[mid].prioritizationFee
        : (recentFees[mid - 1].prioritizationFee +
            recentFees[mid].prioritizationFee) /
          2;

    return Math.max(1, Math.ceil(medianFee));
  }
}
