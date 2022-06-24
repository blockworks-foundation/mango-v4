// import { Connection, PublicKey } from '@solana/web3.js';

// export const getSerumOutputAmount = async (
//   connection: Connection,
//   inputMint: string,
//   outputToken: string,
//   amountIn: number,
// ): Promise<number> => {
//   // TODO: select the correct pool params based on passed in banks
//   const poolParams = getOrcaPoolParams(inputToken, outputToken);

//   const { inputPoolToken, outputPoolToken } = getTokens(
//     poolParams,
//     inputMint.mint.toString(),
//   );

//   const { inputTokenCount, outputTokenCount } = await getTokenCount(
//     connection,
//     poolParams,
//     inputPoolToken,
//     outputPoolToken,
//   );

//   const [poolInputAmount, poolOutputAmount] = [
//     inputTokenCount,
//     outputTokenCount,
//   ];

//   const invariant = poolInputAmount.mul(poolOutputAmount);
//   const nativeAmountIn = toNativeDecimals(amountIn, 9);

//   const [newPoolOutputAmount] = ceilingDivision(
//     invariant,
//     poolInputAmount.add(nativeAmountIn),
//   );

//   const outputAmount = poolOutputAmount.sub(newPoolOutputAmount);

//   return toUiDecimals(outputAmount.toNumber(), 6);
// };
