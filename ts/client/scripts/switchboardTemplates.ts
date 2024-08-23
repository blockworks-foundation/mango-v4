const pythSolOracle =
  'ef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d';
const pythUsdOracle =
  'eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a';
const switchboardSolOracle = 'AEcJSgRBkU9WnKCBELj66TPFfzhKWBWa4tL7JugnonUa';
const switchboardUsdOracle = 'FwYfsmj5x8YZXtQBNo2Cz8TE7WRCMFqA6UTffK4xQKMH';

export const LSTExactIn = (inMint: string, uiAmountIn: string): string => {
  const template = `tasks:
  - conditionalTask:
      attempt:
      - sanctumLstPriceTask:
          lstMint: ${inMint}
      - conditionalTask:
          attempt:
          - valueTask:
              big: ${uiAmountIn}
          - divideTask:
              job:
                tasks:
                - jupiterSwapTask:
                    inTokenAddress: So11111111111111111111111111111111111111112
                    outTokenAddress: ${inMint}
                    baseAmountString: ${uiAmountIn}
      - conditionalTask:
          attempt:
          - multiplyTask:
              job:
                tasks:
                - oracleTask:
                    pythAddress: ${pythSolOracle}
                    pythAllowedConfidenceInterval: 10
          onFailure:
          - multiplyTask:
              job:
                tasks:
                - oracleTask:
                    switchboardAddress: ${switchboardSolOracle}`;
  return template;
};

export const LSTExactOut = (inMint: string, uiOutSolAmount: string): string => {
  const template = `tasks:
  - conditionalTask:
      attempt:
      - sanctumLstPriceTask:
          lstMint: ${inMint}
      - conditionalTask:
          attempt:
          - cacheTask:
              cacheItems:
              - variableName: QTY
                job:
                  tasks:
                  - jupiterSwapTask:
                      inTokenAddress: So11111111111111111111111111111111111111112
                      outTokenAddress: ${inMint}
                      baseAmountString: ${uiOutSolAmount}
          - jupiterSwapTask:
              inTokenAddress: ${inMint}
              outTokenAddress: So11111111111111111111111111111111111111112
              baseAmountString: \${QTY}
          - divideTask:
              big: \${QTY}
      - conditionalTask:
          attempt:
          - multiplyTask:
              job:
                tasks:
                - oracleTask:
                    pythAddress: ${pythSolOracle}
                    pythAllowedConfidenceInterval: 10
          onFailure:
          - multiplyTask:
              job:
                tasks:
                - oracleTask:
                    switchboardAddress: ${switchboardSolOracle}`;
  return template;
};

export const usdcInTokenOutUsdcPool = (
  outMint: string,
  nativeAmountIn: string,
  poolAddress: string,
  poolName: string,
): string => {
  const template = `tasks:
- conditionalTask:
    attempt:
    - valueTask:
        big: ${nativeAmountIn}
    - divideTask:
        job:
          tasks:
          - jupiterSwapTask:
              inTokenAddress: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
              outTokenAddress: ${outMint}
              baseAmountString: ${nativeAmountIn}
    onFailure:
    - lpExchangeRateTask:
        ${poolName}: ${poolAddress}
- conditionalTask:
    attempt:
    - multiplyTask:
        job:
          tasks:
          - oracleTask:
              pythAddress: ${pythUsdOracle}
              pythAllowedConfidenceInterval: 10
    onFailure:
    - multiplyTask:
        job:
          tasks:
          - oracleTask:
              switchboardAddress: ${switchboardUsdOracle}`;
  return template;
};

export const tokenInUsdcOutUsdcPool = (
  inToken: string,
  nativeAmountIn: string,
  poolAddress: string,
  poolName: string,
): string => {
  const template = `tasks:
- conditionalTask:
    attempt:
    - cacheTask:
        cacheItems:
        - variableName: QTY
          job:
            tasks:
            - jupiterSwapTask:
                inTokenAddress: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
                outTokenAddress: ${inToken}
                baseAmountString: ${nativeAmountIn}
    - jupiterSwapTask:
        inTokenAddress: ${inToken}
        outTokenAddress: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
        baseAmountString: \${QTY}
    - divideTask:
        big: \${QTY}
    onFailure:
    - lpExchangeRateTask:
        ${poolName}: ${poolAddress}
- conditionalTask:
    attempt:
    - multiplyTask:
        job:
          tasks:
          - oracleTask:
              pythAddress: ${pythUsdOracle}
              pythAllowedConfidenceInterval: 10
    onFailure:
    - multiplyTask:
        job:
          tasks:
          - oracleTask:
              switchboardAddress: ${switchboardUsdOracle}`;
  return template;
};

export const usdcInTokenOutReversedSolPool = (
  outMint: string,
  nativeAmountIn: string,
  poolAddress: string,
  poolName: string,
): string => {
  const template = `tasks:
- conditionalTask:
    attempt:
    - valueTask:
        big: ${nativeAmountIn}
    - divideTask:
        job:
          tasks:
          - jupiterSwapTask:
              inTokenAddress: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
              outTokenAddress: ${outMint}
              baseAmountString: ${nativeAmountIn}
    onFailure:
    - valueTask:
        big: '1'
    - divideTask:
        job:
          tasks:
          - lpExchangeRateTask:
              ${poolName}: ${poolAddress}
    - conditionalTask:
          attempt:
          - multiplyTask:
              job:
                tasks:
                - oracleTask:
                    pythAddress: ${pythSolOracle}
                    pythAllowedConfidenceInterval: 10
          onFailure:
          - multiplyTask:
              job:
                tasks:
                - oracleTask:
                    switchboardAddress: ${switchboardSolOracle}
- conditionalTask:
    attempt:
    - multiplyTask:
        job:
          tasks:
          - oracleTask:
              pythAddress: ${pythUsdOracle}
              pythAllowedConfidenceInterval: 10
    onFailure:
    - multiplyTask:
        job:
          tasks:
          - oracleTask:
              switchboardAddress: ${switchboardUsdOracle}`;
  return template;
};

export const tokenInUsdcOutReversedSolPool = (
  inToken: string,
  nativeAmountIn: string,
  poolAddress: string,
  poolName: string,
): string => {
  const template = `tasks:
- conditionalTask:
    attempt:
    - cacheTask:
        cacheItems:
        - variableName: QTY
          job:
            tasks:
            - jupiterSwapTask:
                inTokenAddress: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
                outTokenAddress: ${inToken}
                baseAmountString: ${nativeAmountIn}
    - jupiterSwapTask:
        inTokenAddress: ${inToken}
        outTokenAddress: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
        baseAmountString: \${QTY}
    - divideTask:
        big: \${QTY}
    onFailure:
    - valueTask:
        big: '1'
    - divideTask:
        job:
          tasks:
          - lpExchangeRateTask:
              ${poolName}: ${poolAddress}
    - conditionalTask:
          attempt:
          - multiplyTask:
              job:
                tasks:
                - oracleTask:
                    pythAddress: ${pythSolOracle}
                    pythAllowedConfidenceInterval: 10
          onFailure:
          - multiplyTask:
              job:
                tasks:
                - oracleTask:
                    switchboardAddress: ${switchboardSolOracle}
- conditionalTask:
    attempt:
    - multiplyTask:
        job:
          tasks:
          - oracleTask:
              pythAddress: ${pythUsdOracle}
              pythAllowedConfidenceInterval: 10
    onFailure:
    - multiplyTask:
        job:
          tasks:
          - oracleTask:
              switchboardAddress: ${switchboardUsdOracle}`;
  return template;
};

export const usdcInTokenOutSolPool = (
  outMint: string,
  nativeAmountIn: string,
  poolAddress: string,
  poolName: string,
): string => {
  const template = `tasks:
- conditionalTask:
    attempt:
    - valueTask:
        big: ${nativeAmountIn}
    - divideTask:
        job:
          tasks:
          - jupiterSwapTask:
              inTokenAddress: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
              outTokenAddress: ${outMint}
              baseAmountString: ${nativeAmountIn}
    onFailure:
    - lpExchangeRateTask:
        ${poolName}: ${poolAddress}
    - conditionalTask:
          attempt:
          - multiplyTask:
              job:
                tasks:
                - oracleTask:
                    pythAddress: ${pythSolOracle}
                    pythAllowedConfidenceInterval: 10
          onFailure:
          - multiplyTask:
              job:
                tasks:
                - oracleTask:
                    switchboardAddress: ${switchboardSolOracle}
- conditionalTask:
    attempt:
    - multiplyTask:
        job:
          tasks:
          - oracleTask:
              pythAddress: ${pythUsdOracle}
              pythAllowedConfidenceInterval: 10
    onFailure:
    - multiplyTask:
        job:
          tasks:
          - oracleTask:
              switchboardAddress: ${switchboardUsdOracle}`;
  return template;
};

export const tokenInUsdcOutSolPool = (
  inToken: string,
  nativeAmountIn: string,
  poolAddress: string,
  poolName: string,
): string => {
  const template = `tasks:
- conditionalTask:
    attempt:
    - cacheTask:
        cacheItems:
        - variableName: QTY
          job:
            tasks:
            - jupiterSwapTask:
                inTokenAddress: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
                outTokenAddress: ${inToken}
                baseAmountString: ${nativeAmountIn}
    - jupiterSwapTask:
        inTokenAddress: ${inToken}
        outTokenAddress: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
        baseAmountString: \${QTY}
    - divideTask:
        big: \${QTY}
    onFailure:
    - lpExchangeRateTask:
        ${poolName}: ${poolAddress}
    - conditionalTask:
          attempt:
          - multiplyTask:
              job:
                tasks:
                - oracleTask:
                    pythAddress: ${pythSolOracle}
                    pythAllowedConfidenceInterval: 10
          onFailure:
          - multiplyTask:
              job:
                tasks:
                - oracleTask:
                    switchboardAddress: ${switchboardSolOracle}
- conditionalTask:
    attempt:
    - multiplyTask:
        job:
          tasks:
          - oracleTask:
              pythAddress: ${pythUsdOracle}
              pythAllowedConfidenceInterval: 10
    onFailure:
    - multiplyTask:
        job:
          tasks:
          - oracleTask:
              switchboardAddress: ${switchboardUsdOracle}`;
  return template;
};
