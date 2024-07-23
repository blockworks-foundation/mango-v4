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

export const PYTH_SPONSORED_ORACLES = [
  ['SOL/USD', '7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE'],
  ['JITOSOL/USD', 'AxaxyeDT8JnWERSaTKvFXvPKkEdxnamKSqpWbsSjYg1g'],
  ['MSOL/USD', '5CKzb9j4ChgLUt8Gfm5CNGLN6khXKiqMbnGAW4cgXgxK'],
  ['BSOL/USD', '5cN76Xm2Dtx9MnrQqBDeZZRsWruTTcw37UruznAdSvvE'],
  ['BONK/USD', 'DBE3N8uNjhKPRHfANdwGvCZghWXyLPdqdSbEW2XFwBiX'],
  ['W/USD', 'BEMsCSQEGi2kwPA4mKnGjxnreijhMki7L4eeb96ypzF9'],
  ['KMNO/USD', 'ArjngUHXrQPr1wH9Bqrji9hdDQirM6ijbzc1Jj1fXUk7'],
  ['MEW/USD', 'EF6U755BdHMXim8RBw6XSC6Yk6XaouTKpwcBZ7QkcanB'],
  ['TNSR/USD', '9TSGDwcPQX4JpAvZbu2Wp5b68wSYkQvHCvfeBjYcCyC'],
  ['USDC/USD', 'Dpw1EAVrSB1ibxiDQyTAW6Zip3J4Btk2x4SgApQCeFbX'],
  ['BTC/USD', '4cSM2e6rvbGQUFiJbqytoVMi5GgghSMr8LwVrT9VPSPo'],
  ['JTO/USD', '7ajR2zA4MGMMTqRAVjghTKqPPn4kbrj3pYkAVRVwTGzP'],
  ['USDT/USD', 'HT2PLQBcG5EiCcNSaMHAjSgd9F98ecpATbk4Sk5oYuM'],
  ['JUP/USD', '7dbob1psH1iZBS7qPsm3Kwbf5DzSXK8Jyg31CTgTnxH5'],
  ['ETH/USD', '42amVS4KgzR9rA28tkVYqVXjq9Qa8dcZQMbH5EYFX6XC'],
  ['PYTH/USD', '8vjchtMuJNY4oFQdTi8yCe6mhCaNBFaUbktT482TpLPS'],
  ['HNT/USD', '4DdmDswskDxXGpwHrXUfn2CNUm9rt21ac79GHNTN3J33'],
  ['RNDR/USD', 'GbgH1oen3Ne1RY4LwDgh8kEeA1KywHvs5x8zsx6uNV5M'],
  ['ORCA/USD', '4CBshVeNBEXz24GZpoj8SrqP5L7VGG3qjGd6tCST1pND'],
  ['SAMO/USD', '2eUVzcYccqXzsDU1iBuatUaDCbRKBjegEaPPeChzfocG'],
  ['WIF/USD', '6B23K3tkb51vLZA14jcEQVCA1pfHptzEHFA93V5dYwbT'],
  ['LST/USD', '7aT9A5knp62jVvnEW33xaWopaPHa3Y7ggULyYiUsDhu8'],
  ['INF/USD', 'Ceg5oePJv1a6RR541qKeQaTepvERA3i8SvyueX9tT8Sq'],
  ['PRCL/USD', '6a9HN13ZFf57WZd4msn85KWLe5iTayqS8Ee8gstQkxqm'],
  ['RAY/USD', 'Hhipna3EoWR7u8pDruUg8RxhP5F6XLh6SEHMVDmZhWi8'],
  ['FIDA/USD', '2cfmeuVBf7bvBJcjKBQgAwfvpUvdZV7K8NZxUEuccrub'],
  ['MNDE/USD', 'GHKcxocPyzSjy7tWApQjKRkDNuVXd4Kk624zhuaR7xhC'],
  ['MOBILE/USD', 'DQ4C1tzvu28cwo1roN1Wm6TW35sfJEjLh517k3ZeWevx'],
  ['IOT/USD', '8UYEn5Weq7toHwgcmctvcAxaNJo3SJxXEayM57rpoXr9'],
  ['GOFX/USD', '2WS7DByXgzmsGD1QfDyvY2pwAmxjsPDrF2DijwpRBxr7'],
  ['NEON/USD', 'F2VfCymdNQiCa8Vyg5E7BwEv9UPwfm8cVN6eqQLqXiGo'],
  ['AUD/USD', '6pPXqXcgFFoLEcXfedWJy3ypNZVJ1F3mgipaDFsvZ1co'],
  ['GBP/USD', 'G25Tm7UkVruTJ7mcbCxFm45XGWwsH72nJKNGcHEQw1tU'],
  ['EUR/USD', 'Fu76ChamBDjE8UuGLV6GP2AcPPSU6gjhkNhAyuoPm7ny'],
  ['XAG/USD', 'H9JxsWwtDZxjSL6m7cdCVsWibj3JBMD9sxqLjadoZnot'],
  ['XAU/USD', '2uPQGpm8X4ZkxMHxrAW1QuhXcse1AHEgPih6Xp9NuEWW'],
  ['INJ/USD', 'GwXYEfmPdgHcowF9GZwbb1WiTGTn1fuT3hbSLneoBKK6'],
  ['SLND/USD', '6vPfd6612huknxXaDapfj6cVmB8NvCwKm3BHKFxzo1EZ'],
  ['WEN/USD', 'CsG7wXoqZKNxx4UnFtvozfwXQ9RgpKe7zSJa4LWh5MT9'],
];
