export const getTokenDecimals = (symbol: string) => {
  const tokenMeta = tokens.find((t) => t.symbol === symbol);

  if (!tokenMeta) throw new Error('TokenDecimalError: Token not found');

  return tokenMeta.decimals;
};

const tokens = [
  { symbol: 'USDC', decimals: 6 },
  { symbol: 'SOL', decimals: 9 },
  { symbol: 'BTC', decimals: 6 },
];

export default tokens;
