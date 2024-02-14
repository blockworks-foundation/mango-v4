import {
  AccountInfo,
  AddressLookupTableAccount,
  Connection,
  PublicKey,
  TransactionInstruction,
  TransactionMessage,
  VersionedTransaction,
} from '@solana/web3.js';
import {
  buildFetch,
  createAssociatedTokenAccountIdempotentInstruction,
} from './utils';

export const MANGO_ROUTER_API_URL = 'https://api.mngo.cloud/router/v1';

export interface QuoteParams {
  sourceMint: string;
  destinationMint: string;
  amount: number;
  swapMode: SwapMode;
}
export declare type TokenMintAddress = string;
export interface Quote {
  notEnoughLiquidity: boolean;
  minInAmount?: number;
  minOutAmount?: number;
  inAmount: number;
  outAmount: number;
  feeAmount: number;
  feeMint: TokenMintAddress;
  feePct: number;
  priceImpactPct: number;
}
export declare type QuoteMintToReferrer = Map<TokenMintAddress, string>;
export interface SwapParams {
  sourceMint: string;
  destinationMint: string;
  userSourceTokenAccount: string;
  userDestinationTokenAccount: string;
  userTransferAuthority: string;
  /**
   * amount is used for instruction and can be null when it is an intermediate swap, only the first swap has an amount
   */
  amount: number;
  swapMode: SwapMode;
  openOrdersAddress?: string;
  quoteMintToReferrer?: QuoteMintToReferrer;
}
export declare type PlatformFee = {
  feeBps: number;
  feeAccount: string;
};
export interface ExactOutSwapParams extends SwapParams {
  inAmount: number;
  slippageBps: number;
  platformFee?: PlatformFee;
  overflowFeeAccount?: string;
}
export declare type AccountInfoMap = Map<string, AccountInfo<Buffer> | null>;

export declare type AmmLabel =
  | 'Aldrin'
  | 'Crema'
  | 'Cropper'
  | 'Cykura'
  | 'DeltaFi'
  | 'GooseFX'
  | 'Invariant'
  | 'Lifinity'
  | 'Lifinity V2'
  | 'Marinade'
  | 'Mercurial'
  | 'Meteora'
  | 'Raydium'
  | 'Raydium CLMM'
  | 'Saber'
  | 'Serum'
  | 'Orca'
  | 'Step'
  | 'Penguin'
  | 'Saros'
  | 'Stepn'
  | 'Orca (Whirlpools)'
  | 'Sencha'
  | 'Saber (Decimals)'
  | 'Dradex'
  | 'Balansol'
  | 'Openbook'
  | 'Unknown';

export interface TransactionFeeInfo {
  signatureFee: number;
  openOrdersDeposits: number[];
  ataDeposits: number[];
  totalFeeAndDeposits: number;
  minimumSOLForTransaction: number;
}

export declare enum SwapMode {
  ExactIn = 'ExactIn',
  ExactOut = 'ExactOut',
}

export interface Fee {
  amount: number;
  mint: string;
  pct: number;
}
export interface MarketInfo {
  id: string;
  inAmount: number;
  inputMint: string;
  label: string;
  lpFee: Fee;
  notEnoughLiquidity: boolean;
  outAmount: number;
  outputMint: string;
  platformFee: Fee;
  priceImpactPct: number;
}

export interface RouteInfo {
  amount: number;
  inAmount: number;
  marketInfos: MarketInfo[];
  otherAmountThreshold: number;
  outAmount: number;
  priceImpactPct: number;
  slippageBps: number;
  swapMode: SwapMode;
  instructions?: TransactionInstruction[];
  mints?: PublicKey[];
  routerName?: 'Mango';
}

export type Routes = {
  routes: RouteInfo[];
  bestRoute: RouteInfo | null;
};

export type Token = {
  address: string;
  chainId: number;
  decimals: number;
  name: string;
  symbol: string;
  logoURI: string;
  extensions: {
    coingeckoId?: string;
  };
  tags: string[];
};

const fetchJupiterRoutes = async (
  inputMint,
  outputMint,
  amount = '0',
  slippage = 50,
  swapMode = 'ExactIn',
  feeBps = '0',
): Promise<Routes> => {
  {
    const paramsString = new URLSearchParams({
      inputMint: inputMint.toString(),
      outputMint: outputMint.toString(),
      amount: amount.toString(),
      slippageBps: Math.ceil(slippage * 100).toString(),
      feeBps: feeBps.toString(),
      swapMode,
    }).toString();

    const response = await (
      await buildFetch()
    )(`https://quote-api.jup.ag/v4/quote?${paramsString}`);

    const res = await response.json();
    const data = res.data;

    return {
      routes: res.data as RouteInfo[],
      bestRoute: (data.length ? data[0] : null) as RouteInfo | null,
    };
  }
};

const fetchMangoRoutes = async (
  inputMint,
  outputMint,
  amount = '0',
  slippage = 50,
  swapMode = 'ExactIn',
  feeBps = '0',
  wallet = PublicKey.default,
): Promise<Routes> => {
  {
    const defaultOtherAmount =
      swapMode === 'ExactIn' ? 0 : Number.MAX_SAFE_INTEGER;

    const paramsString = new URLSearchParams({
      inputMint: inputMint.toString(),
      outputMint: outputMint.toString(),
      amount: amount.toString(),
      slippage: ((slippage * 1) / 100).toString(),
      feeBps: feeBps.toString(),
      mode: swapMode,
      wallet: wallet.toString(),
      otherAmountThreshold: defaultOtherAmount.toString(),
    }).toString();

    const response = await fetch(
      `${MANGO_ROUTER_API_URL}/swap?${paramsString}`,
    );

    const res = await response.json();
    const data: RouteInfo[] = res.map((route: any) => ({
      ...route,
      priceImpactPct: route.priceImpact,
      slippageBps: slippage,
      marketInfos: route.marketInfos.map((mInfo: any) => ({
        ...mInfo,
        lpFee: {
          ...mInfo.fee,
          pct: mInfo.fee.rate,
        },
      })),
      mints: route.mints.map((x: string) => new PublicKey(x)),
      instructions: route.instructions.map((ix: any) => ({
        ...ix,
        programId: new PublicKey(ix.programId),
        data: Buffer.from(ix.data, 'base64'),
        keys: ix.keys.map((key: any) => ({
          ...key,
          pubkey: new PublicKey(key.pubkey),
        })),
      })),
      routerName: 'Mango',
    }));
    return {
      routes: data,
      bestRoute: (data.length ? data[0] : null) as RouteInfo | null,
    };
  }
};

export const fetchRoutes = async (
  inputMint,
  outputMint,
  amount = '0',
  slippage = 50,
  swapMode = 'ExactIn',
  feeBps = '0',
  wallet = PublicKey.default,
): Promise<Routes> => {
  try {
    const responses = await Promise.allSettled([
      fetchMangoRoutes(
        inputMint,
        outputMint,
        amount,
        slippage,
        swapMode,
        feeBps,
        wallet,
      ),
      fetchJupiterRoutes(
        inputMint,
        outputMint,
        amount,
        slippage,
        swapMode,
        feeBps,
      ),
    ]);

    const routes: RouteInfo[] = responses
      .filter((x) => x.status === 'fulfilled' && x.value.bestRoute !== null)
      .map((x) => (x as any).value.routes)
      .flat();

    const sortedBestQuoteFirst = routes.sort(
      (a, b) =>
        swapMode == 'ExactIn'
          ? Number(b.outAmount) - Number(a.outAmount) // biggest out
          : Number(a.inAmount) - Number(b.inAmount), // smallest in
    );

    return {
      routes: sortedBestQuoteFirst,
      bestRoute: sortedBestQuoteFirst[0],
    };
  } catch (e) {
    return {
      routes: [],
      bestRoute: null,
    };
  }
};

export const prepareMangoRouterInstructions = async (
  selectedRoute: RouteInfo,
  inputMint: PublicKey,
  outputMint: PublicKey,
  userPublicKey: PublicKey,
): Promise<[TransactionInstruction[], AddressLookupTableAccount[]]> => {
  if (!selectedRoute || !selectedRoute.mints || !selectedRoute.instructions) {
    return [[], []];
  }
  const mintsToFilterOut = [inputMint, outputMint];
  const filteredOutMints = [
    ...selectedRoute.mints.filter(
      (routeMint) =>
        !mintsToFilterOut.find((filterOutMint) =>
          filterOutMint.equals(routeMint),
        ),
    ),
  ];
  const additionalInstructions: TransactionInstruction[] = [];
  for (const mint of filteredOutMints) {
    const ix = await createAssociatedTokenAccountIdempotentInstruction(
      userPublicKey,
      userPublicKey,
      mint,
    );
    additionalInstructions.push(ix);
  }
  const instructions = [
    ...additionalInstructions,
    ...selectedRoute.instructions,
  ];
  return [instructions, []];
};

const deserializeJupiterIxAndAlt = async (
  connection: Connection,
  swapTransaction: string,
): Promise<[TransactionInstruction[], AddressLookupTableAccount[]]> => {
  const parsedSwapTransaction = VersionedTransaction.deserialize(
    Buffer.from(swapTransaction, 'base64'),
  );
  const message = parsedSwapTransaction.message;
  // const lookups = message.addressTableLookups
  const addressLookupTablesResponses = await Promise.all(
    message.addressTableLookups.map((alt) =>
      connection.getAddressLookupTable(alt.accountKey),
    ),
  );
  const addressLookupTables: AddressLookupTableAccount[] =
    addressLookupTablesResponses
      .map((alt) => alt.value)
      .filter((x): x is AddressLookupTableAccount => x !== null);

  const decompiledMessage = TransactionMessage.decompile(message, {
    addressLookupTableAccounts: addressLookupTables,
  });

  return [decompiledMessage.instructions, addressLookupTables];
};

export const fetchJupiterTransaction = async (
  connection: Connection,
  selectedRoute: RouteInfo,
  userPublicKey: PublicKey,
  slippage: number,
  inputMint: PublicKey,
  outputMint: PublicKey,
): Promise<[TransactionInstruction[], AddressLookupTableAccount[]]> => {
  const transactions = await (
    await (
      await buildFetch()
    )('https://quote-api.jup.ag/v4/swap', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        // route from /quote api
        route: selectedRoute,
        // user public key to be used for the swap
        userPublicKey,
        // feeAccount is optional. Use if you want to charge a fee.  feeBps must have been passed in /quote API.
        // This is the ATA account for the output token where the fee will be sent to. If you are swapping from SOL->USDC then this would be the USDC ATA you want to collect the fee.
        // feeAccount: 'fee_account_public_key',
        slippageBps: Math.ceil(slippage * 100),
      }),
    })
  ).json();

  const { swapTransaction } = transactions;

  const [ixs, alts] = await deserializeJupiterIxAndAlt(
    connection,
    swapTransaction,
  );

  const isSetupIx = (pk: PublicKey): boolean =>
    pk.toString() === 'ComputeBudget111111111111111111111111111111' ||
    pk.toString() === 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA';

  const isDuplicateAta = (ix: TransactionInstruction): boolean => {
    return (
      ix.programId.toString() ===
        'ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL' &&
      (ix.keys[3].pubkey.toString() === inputMint.toString() ||
        ix.keys[3].pubkey.toString() === outputMint.toString())
    );
  };

  const filtered_jup_ixs = ixs
    .filter((ix) => !isSetupIx(ix.programId))
    .filter((ix) => !isDuplicateAta(ix));

  return [filtered_jup_ixs, alts];
};
