import { PublicKey } from '@solana/web3.js';

export const MANGO_REALM_PK = new PublicKey(
  'DPiH3H3c7t47BMxqTxLsuPQpEC6Kne8GA9VXbxpnZxFE',
);
export const MANGO_GOVERNANCE_PROGRAM = new PublicKey(
  'GqTPL6qRf5aUuqscLh8Rg2HTxPUXfhhAXDptTLhp1t2J',
);

export const VOTER_INFO_EVENT_NAME = 'VoterInfo';
export const DEPOSIT_EVENT_NAME = 'DepositEntryInfo';
// The wallet can be any existing account for the simulation
// Note: when running a local validator ensure the account is copied from devnet: --clone ENmcpFCpxN1CqyUjuog9yyUVfdXBKF3LVCwLr7grJZpk -ud
export const SIMULATION_WALLET = 'ENmcpFCpxN1CqyUjuog9yyUVfdXBKF3LVCwLr7grJZpk';

export const MANGO_MINT = new PublicKey(
  'MangoCzJ36AjZyKwVj3VnYU4GTonjfVEnJmvvWaxLac',
);

export const MANGO_DAO_WALLET_GOVERNANCE = new PublicKey(
  '7zGXUAeUkY9pEGfApsY26amibvqsf2dmty1cbtxHdfaQ',
);
export const MANGO_DAO_WALLET = new PublicKey(
  '5tgfd6XgwiXB9otEnzFpXK11m7Q7yZUaAJzWK4oT5UGF',
);

export const MANGO_MINT_DECIMALS = 6;

export const MAINNET_PYTH_PROGRAM = new PublicKey(
  'FsJ3A3u2vn5cTVofAjvy6y5kwABJAqYWpe4975bi2epH',
);
export const DEVNET_PYTH_PROGRAM = new PublicKey(
  'gSbePebfvPy7tRqimPoVecS2UsBvYv46ynrzWocc92s',
);

export const SB_ON_DEMAND_TESTING_ORACLES = [
  ['DIGITSOL', '2A7aqNLy26ZBSMWP2Ekxv926hj16tCA47W1sHWVqaLii'],
  ['JLP', '65J9bVEMhNbtbsNgArNV1K4krzcsomjho4bgR51sZXoj'],
  ['INF', 'AZcoqpWhMJUaKEDUfKsfzCr3Y96gSQwv43KSQ6KpeyQ1'],
  ['GUAC', 'Ai2GsLRioGKwVgWX8dtbLF5rJJEZX17SteGEDqrpzBv3'],
  ['RAY', 'AJkAFiXdbMonys8rTXZBrRnuUiLcDFdkyoPuvrVKXhex'],
  ['JUP', '2F9M59yYc28WMrAymNWceaBEk8ZmDAjUAKULp8seAJF3'],
];
