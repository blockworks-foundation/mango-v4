import { Percentage, U64_MAX, ZERO } from '@orca-so/common-sdk';
import {
  ORCA_WHIRLPOOL_PROGRAM_ID,
  SwapQuote,
  SwapUtils,
  Whirlpool,
  WhirlpoolClient,
  WhirlpoolContext,
  WhirlpoolIx,
  buildWhirlpoolClient,
  swapQuoteByInputToken,
  swapQuoteByOutputToken,
} from '@orca-so/whirlpools-sdk';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import BN from 'bn.js';
import { getAssociatedTokenAddress, toUiDecimals } from '../../src/utils';
import { AnchorProvider, BorshAccountsCoder, Idl } from '@coral-xyz/anchor';

export interface DepthResult {
  label: string;
  maxAmtIn: BN;
  minAmtOut: BN;
  ok: boolean;
}

export enum SwapMode {
  ExactIn = 'ExactIn',
  ExactOut = 'ExactOut',
}

export interface SwapResult {
  instructions: (wallet: PublicKey) => Promise<TransactionInstruction[]>;
  label: string;
  marketInfos: {
    label: string;
    fee: { amount: BN; mint: PublicKey; rate: number };
  }[];
  maxAmtIn: BN;
  minAmtOut: BN;
  mints: PublicKey[];
  ok: boolean;
}

function mergeSwapResults(...hops: SwapResult[]) {
  const firstHop = hops[0];
  const lastHop = hops[hops.length - 1];
  return {
    instructions: async (wallet: PublicKey) =>
      await (await Promise.all(hops.map((h) => h.instructions(wallet)))).flat(),
    label: hops.map((h) => h.label).join('_'),
    marketInfos: [...firstHop.marketInfos, ...lastHop.marketInfos],
    maxAmtIn: firstHop.maxAmtIn,
    minAmtOut: lastHop.minAmtOut,
    mints: [...firstHop.mints, ...lastHop.mints],
    ok: hops.reduce((p, c) => p && c.ok, true),
  };
}

interface Edge {
  label: string;
  inputMint: PublicKey;
  outputMint: PublicKey;
  swap: (
    amount: BN,
    otherAmountThreshold: BN,
    mode: SwapMode,
    slippage: number,
  ) => Promise<SwapResult>;
}

class WhirlpoolEdge implements Edge {
  constructor(
    public label: string,
    public inputMint: PublicKey,
    public outputMint: PublicKey,
    public poolPk: PublicKey,
    public client: WhirlpoolClient,
  ) {}

  static pairFromPool(pool: Whirlpool, client: WhirlpoolClient): Edge[] {
    const label = pool.getAddress().toString();
    const fwd = new WhirlpoolEdge(
      label,
      pool.getTokenAInfo().mint,
      pool.getTokenBInfo().mint,
      pool.getAddress(),
      client,
    );
    const bwd = new WhirlpoolEdge(
      label,
      pool.getTokenBInfo().mint,
      pool.getTokenAInfo().mint,
      pool.getAddress(),
      client,
    );
    return [fwd, bwd];
  }

  async swap(
    amount: BN,
    otherAmountThreshold: BN,
    mode: SwapMode,
    slippage: number,
  ): Promise<SwapResult> {
    try {
      const fetcher = this.client.getFetcher();
      const pool = await this.client.getPool(this.poolPk);
      const programId = this.client.getContext().program.programId;
      const slippageLimit = Percentage.fromFraction(slippage * 1e8, 1e8);
      let quote: SwapQuote | undefined;
      let ok = false;

      if (mode === SwapMode.ExactIn) {
        quote = await swapQuoteByInputToken(
          pool,
          this.inputMint,
          amount,
          slippageLimit,
          programId,
          fetcher,
          false,
        );
        ok = otherAmountThreshold.lte(quote.estimatedAmountOut);
      } else {
        quote = await swapQuoteByOutputToken(
          pool,
          this.outputMint,
          amount,
          slippageLimit,
          programId,
          fetcher,
          false,
        );
        ok = otherAmountThreshold.gte(quote.estimatedAmountIn);
      }

      const instructions = async (wallet: PublicKey) => {
        if (!ok) {
          return [];
        }
        const tokenIn = await getAssociatedTokenAddress(this.inputMint, wallet);
        const tokenOut = await getAssociatedTokenAddress(
          this.outputMint,
          wallet,
        );
        const swapIx = WhirlpoolIx.swapIx(
          this.client.getContext().program,
          SwapUtils.getSwapParamsFromQuote(
            quote!,
            this.client.getContext(),
            pool,
            tokenIn,
            tokenOut,
            wallet,
          ),
        );
        return swapIx.instructions;
      };
      return {
        ok,
        instructions,
        label: this.poolPk.toString(),
        marketInfos: [
          {
            label: 'Whirlpool',
            fee: {
              amount: quote.estimatedFeeAmount,
              mint: this.inputMint,
              rate: pool.getData().feeRate * 1e-6,
            },
          },
        ],
        maxAmtIn: quote.estimatedAmountIn,
        minAmtOut: quote.estimatedAmountOut,
        mints: [this.inputMint, this.outputMint],
      };
    } catch (e) {
      // console.log(
      //   "could not swap",
      //   this.poolPk.toString().slice(0, 6),
      //   this.inputMint.toString().slice(0, 6),
      //   this.outputMint.toString().slice(0, 6),
      //   amount.toNumber(),
      //   otherAmountThreshold.toNumber()
      // );
      return {
        ok: false,
        label: '',
        marketInfos: [],
        maxAmtIn: amount,
        minAmtOut: otherAmountThreshold,
        mints: [this.inputMint, this.outputMint],
        instructions: async () => [],
      };
    }
  }
}

export class Router {
  minTvl: number;
  routes: Map<string, Map<string, Edge[]>>;

  whirlpoolClient: WhirlpoolClient;
  whirlpoolSub?: number;

  constructor(anchorProvider: AnchorProvider, minTvl: number) {
    this.minTvl = minTvl;
    this.routes = new Map();
    this.whirlpoolClient = buildWhirlpoolClient(
      WhirlpoolContext.withProvider(anchorProvider, ORCA_WHIRLPOOL_PROGRAM_ID),
    );
  }

  public async start(): Promise<void> {
    await this.indexWhirpools();

    // setup a websocket connection to refresh all whirpool program accounts
    const idl = this.whirlpoolClient.getContext().program.idl;
    const whirlpoolCoder = new BorshAccountsCoder(idl as Idl);
    this.whirlpoolSub = this.whirlpoolClient
      .getContext()
      .connection.onProgramAccountChange(
        ORCA_WHIRLPOOL_PROGRAM_ID,
        (p) => {
          const key = p.accountId.toBase58();
          const accountData = p.accountInfo.data;
          const value = whirlpoolCoder.decodeAny(accountData);
          this.whirlpoolClient.getFetcher()['_cache'][key] = {
            entity: undefined,
            value,
          };
        },
        'processed',
      );
  }

  public async stop(): Promise<void> {
    if (this.whirlpoolSub) {
      await this.whirlpoolClient
        .getContext()
        .connection.removeProgramAccountChangeListener(this.whirlpoolSub);
    }
  }

  addEdge(edge: Edge) {
    const mintA = edge.inputMint.toString();
    const mintB = edge.outputMint.toString();
    if (!this.routes.has(mintA)) {
      this.routes.set(mintA, new Map());
    }

    const routesFromA = this.routes.get(mintA)!;
    if (!routesFromA.has(mintB)) {
      routesFromA.set(mintB, []);
    }

    const routesFromAToB = routesFromA.get(mintB)!;
    routesFromAToB.push(edge);
  }

  addEdges(edges: Edge[]) {
    for (const edge of edges) {
      this.addEdge(edge);
    }
  }

  async indexWhirpools(): Promise<void> {
    console.log('fetch poolPks');
    const poolsPks = (
      await this.whirlpoolClient.getContext().program.account.whirlpool.all()
    ).map((p) => p.publicKey);
    console.log('fetch pools', poolsPks.length);

    // sucks to double fetch but I couldn't find another way to do this
    const pools = (
      await this.whirlpoolClient.getPools(
        poolsPks.slice(0, poolsPks.length / 2),
        true,
      )
    ).concat(
      await this.whirlpoolClient.getPools(
        poolsPks.slice(poolsPks.length / 2),
        true,
      ),
    );
    const mints = Array.from(
      new Set(
        pools.flatMap((p) => [
          p.getTokenAInfo().mint.toString(),
          p.getTokenBInfo().mint.toString(),
        ]),
      ),
    );
    console.log('fetch prices', mints.length);
    const prices: Record<string, number> = {};
    const batchSize = 64;
    for (let i = 0; i < mints.length; i += batchSize) {
      const mintBatch = mints.slice(i, i + batchSize);
      const quoteResponse = await fetch(
        `https://quote-api.jup.ag/v4/price?ids=${mintBatch.join(',')}`,
      );
      const quotes: any = await quoteResponse.json();

      for (const pk in quotes.data) {
        prices[pk] = quotes.data[pk].price;
      }
    }

    const filtered = pools.filter((p) => {
      const mintA = p.getTokenAInfo().mint.toString();
      const mintB = p.getTokenBInfo().mint.toString();
      const priceA = prices[mintA];
      const priceB = prices[mintB];

      if (!priceA || !priceB) {
        // console.log(
        //   "filter pool",
        //   p.getAddress().toString(),
        //   "unknown price for mint",
        //   priceA ? mintB : mintA
        // );
        return false;
      }

      const vaultBalanceA = toUiDecimals(
        p.getTokenVaultAInfo().amount,
        p.getTokenAInfo().decimals,
      );
      const vaultBalanceB = toUiDecimals(
        p.getTokenVaultBInfo().amount,
        p.getTokenBInfo().decimals,
      );

      const tvl = vaultBalanceA * priceA + vaultBalanceB * priceB;
      if (tvl <= this.minTvl) {
        // console.log(
        //   "filter pool",
        //   p.getAddress().toString(),
        //   "tvl",
        //   tvl,
        //   mintA,
        //   mintB
        // );
        return false;
      }

      return true;
    });

    console.log(
      'found',
      poolsPks.length,
      'pools.',
      filtered.length,
      'of those with TVL >',
      this.minTvl,
      'USD',
    );

    this.routes = new Map();
    for (const pool of filtered) {
      this.addEdges(WhirlpoolEdge.pairFromPool(pool, this.whirlpoolClient));
    }
  }

  public async queryDepth(
    inputMint: PublicKey,
    outputMint: PublicKey,
    startAmount: BN,
    referencePrice: number,
    priceImpactLimit: number,
  ): Promise<DepthResult[]> {
    let results: DepthResult[] = [];

    const A = inputMint.toString();
    const fromA = this.routes.get(A);
    if (!fromA) return results;

    const Z = outputMint.toString();
    const AtoZ = fromA?.get(Z);

    // direct swaps A->Z
    if (AtoZ) {
      results = await Promise.all(
        AtoZ.map(async (eAZ) => {
          let bestResult = {
            label: eAZ.label,
            maxAmtIn: ZERO,
            minAmtOut: ZERO,
            ok: false,
          };
          let inAmount = startAmount;
          while (inAmount.lt(U64_MAX)) {
            const outAmountThreshold = inAmount
              .divn(referencePrice)
              .muln(1 - priceImpactLimit);
            const swapResult = await eAZ.swap(
              inAmount,
              outAmountThreshold,
              SwapMode.ExactIn,
              0,
            );
            const actualPrice =
              Number(swapResult.maxAmtIn.toString()) /
              Number(swapResult.minAmtOut.toString());
            const priceImpact = actualPrice / referencePrice - 1;

            if (!swapResult.ok || priceImpact >= priceImpactLimit) break;

            bestResult = { ...swapResult, ok: true };
            inAmount = inAmount.muln(1.1);
          }
          return bestResult;
        }),
      );
    }

    // swap A->B->Z
    for (const [B, AtoB] of fromA.entries()) {
      const fromB = this.routes.get(B);
      const BtoZ = fromB?.get(Z);

      if (!BtoZ) continue;

      // swap A->B->Z amt=IN oth=OUT
      for (const eAB of AtoB) {
        for (const eBZ of BtoZ) {
          let bestResult = {
            label: `${eAB.label}_${eBZ.label}`,
            maxAmtIn: ZERO,
            minAmtOut: ZERO,
            ok: false,
          };
          let inAmount = startAmount;

          while (inAmount.lt(U64_MAX)) {
            const outAmountThreshold = inAmount
              .divn(referencePrice)
              .muln(1 - priceImpactLimit);
            const firstHop = await eAB.swap(
              inAmount,
              ZERO,
              SwapMode.ExactIn,
              0,
            );
            const secondHop = await eBZ.swap(
              firstHop.minAmtOut,
              outAmountThreshold,
              SwapMode.ExactIn,
              0,
            );
            const actualPrice =
              Number(firstHop.maxAmtIn.toString()) /
              Number(secondHop.minAmtOut.toString());
            const priceImpact = actualPrice / referencePrice - 1;

            if (
              !firstHop.ok ||
              !secondHop.ok ||
              priceImpact >= priceImpactLimit
            )
              break;

            bestResult = {
              label: `${firstHop.label}_${secondHop.label}`,
              maxAmtIn: firstHop.maxAmtIn,
              minAmtOut: secondHop.minAmtOut,
              ok: true,
            };
            inAmount = inAmount.muln(2 ** 0.5);
          }

          results.push(bestResult);
        }
      }
    }

    // swap A->B->C->Z
    for (const [B, AtoB] of fromA.entries()) {
      const fromB = this.routes.get(B)!;
      for (const [C, BtoC] of fromB.entries()) {
        const fromC = this.routes.get(C)!;
        const CtoZ = fromC?.get(Z);

        if (!CtoZ) continue;

        // swap A->B->Z amt=IN oth=OUT
        for (const eAB of AtoB) {
          for (const eBC of BtoC) {
            for (const eCZ of CtoZ) {
              let bestResult = {
                label: `${eAB.label}_${eBC.label}_${eCZ.label}`,
                maxAmtIn: ZERO,
                minAmtOut: ZERO,
                ok: false,
              };
              let inAmount = startAmount;

              while (inAmount.lt(U64_MAX)) {
                const outAmountThreshold = inAmount
                  .divn(referencePrice)
                  .muln(1 - priceImpactLimit);
                const firstHop = await eAB.swap(
                  inAmount,
                  ZERO,
                  SwapMode.ExactIn,
                  0,
                );
                const secondHop = await eBC.swap(
                  firstHop.minAmtOut,
                  ZERO,
                  SwapMode.ExactIn,
                  0,
                );
                const thirdHop = await eCZ.swap(
                  secondHop.minAmtOut,
                  outAmountThreshold,
                  SwapMode.ExactIn,
                  0,
                );

                const actualPrice =
                  Number(firstHop.maxAmtIn.toString()) /
                  Number(thirdHop.minAmtOut.toString());
                const priceImpact = actualPrice / referencePrice - 1;

                if (
                  !firstHop.ok ||
                  !secondHop.ok ||
                  !thirdHop.ok ||
                  priceImpact >= priceImpactLimit
                )
                  break;

                bestResult = {
                  label: `${firstHop.label}_${secondHop.label}_${thirdHop.label}`,
                  maxAmtIn: firstHop.maxAmtIn,
                  minAmtOut: thirdHop.minAmtOut,
                  ok: true,
                };
                inAmount = inAmount.muln(2 ** 0.5);
              }

              results.push(bestResult);
            }
          }
        }
      }
    }

    return results;
  }

  public async swap(
    inputMint: PublicKey,
    outputMint: PublicKey,
    amount: BN,
    otherAmountThreshold: BN,
    mode: SwapMode,
    slippage: number,
  ): Promise<SwapResult[]> {
    let results: SwapResult[] = [];

    const A = inputMint.toString();
    const fromA = this.routes.get(A);
    if (!fromA) return results;

    const Z = outputMint.toString();
    const AtoZ = fromA?.get(Z);

    // direct swaps A->Z
    if (AtoZ) {
      results = await Promise.all(
        AtoZ.map((eAZ) =>
          eAZ.swap(amount, otherAmountThreshold, mode, slippage),
        ),
      );
    }

    for (const [B, AtoB] of fromA.entries()) {
      const fromB = this.routes.get(B);
      const BtoZ = fromB?.get(Z);

      if (!BtoZ) continue;

      if (mode === SwapMode.ExactIn) {
        // swap A->B->Z amt=IN oth=OUT
        for (const eAB of AtoB) {
          // TODO: slippage limit should apply for whole route not single hop
          const firstHop = await eAB.swap(amount, ZERO, mode, slippage);
          for (const eBZ of BtoZ) {
            const secondHop = await eBZ.swap(
              firstHop.minAmtOut,
              otherAmountThreshold,
              mode,
              slippage,
            );
            results.push(mergeSwapResults(firstHop, secondHop));
          }
        }
      } else if (mode === SwapMode.ExactOut) {
        // swap A->B->Z amt=OUT oth=IN
        for (const eBZ of BtoZ) {
          const secondHop = await eBZ.swap(amount, U64_MAX, mode, slippage);
          for (const eAB of AtoB) {
            const firstHop = await eAB.swap(
              secondHop.maxAmtIn,
              otherAmountThreshold,
              mode,
              slippage,
            );
            const merged = mergeSwapResults(firstHop, secondHop);
            results.push(merged);
          }
        }
      }

      // TODO: A->B->C->Z
    }
    return results;
  }
}
