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

export const SB_ORACLES = [
  ['RENDER', '94rcvEktGTwCr2uZ6UGq7GPwwkb5BXWox942pGqJPMW3'],
  ['MNGO', '9AhK6J5bNWBkBEqC9ix5K4bVSgwh9uMEoJvq8Ad2mKZZ'],
  ['BLZE', 'p8WhggEpj4bTQJpGqPANiqG2CWUxooxWBWzi5qhrdzy'],
  ['DAI', 'GXRCfroqu9k4ZoS5MyjUSiuoRb1bhps7nacEQLkLBVgr'],
  ['CHAI', 'GXRCfroqu9k4ZoS5MyjUSiuoRb1bhps7nacEQLkLBVgr'],
];

export const SB_LST_ORACLES = [
  ['JSOL', '91yrNSV8mofYcP6NCsHNi2YgNxwukBenv5MCRFD92Rgp'],
  ['HUBSOL', '318uRUE2RuYpvv1VwxC4eJwViDrRrxUTTqoUBV1cgUYi'],
  ['DUALSOL', '6zBkSKhAqLT2SNRbzTbrom2siKhVZ6SLQcFPnvyexdTE'],
  ['DIGITSOL', 'Am5rswhcxQhqviDXuaiZnLvkpmB4iJEdxmhqMMZDV3KJ'],
  ['MANGOSOL', 'FLroEBBA4Fa8ENqfBmqyypq8U6ai2mD7c5k6Vfb2PWzv'],
  ['COMPASSSOL', '9gFehBozPdWafFfPiZRbub2yUmwYJrGMvguKHii7cMTA'],
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

export const SB_FEEDS_TO_MIGRATE = [
  {
    name: 'STEP/USD',
    pk: '2MW4RK9a7omGDswjLvAmWc75r8zHNdneVwtgqpU1nK3v',
    newPk: '5anCm1isKCEyBaiLB4MXL4Q1XDAWgTyfT9i5knQsaZTJ',
    pythFeed: 'usd',
  },
  {
    name: 'POPCAT/USD',
    pk: '2stQe1XLGkuTZ22gQrgZKsb93iG9mWXSLfANMPRjs5Ky',
    newPk: 'G7yd9DdEDjb1ynTTmG2hZhPtenz5DVSzwvwtHf8T6JeW',
    pythFeed: 'usd',
  },
  {
    name: 'USDH/USD',
    pk: 'B2iwUqbK6ksAsD21SPUUjjx3EwdswpVWFGkeWPHaYd81',
    newPk: 'J2KP4GcXaC16fEB2vne1HsKxtNxsiAGS1geMUFuNpLuo',
    pythFeed: 'usd',
  },
  {
    name: 'NOS/USD',
    pk: 'ED844qf2K6M3JFD9RJCqEVaJ9zP2i9B5Rag5YzVw8Tav',
    newPk: 'DmAmYWwGQjHy6JY6EKW9fUNs2Bdaj1WNrjVgKEvuaNvL',
    pythFeed: 'usd',
  },
  {
    name: 'GUAC/USD',
    pk: '2kbaLTLTovQxkVzmwTXt5ddJKGgmpEAfx9ZNxZMspy8s',
    newPk: 'GBTqdMpJ3uJjzdhcCf9JYAwE2fXSSyPdJU1PL41PKZ1k',
    pythFeed: 'usd',
  },
  {
    name: 'CORN/USD',
    pk: 'BBZWtK26bnwnC6gtyEy2Z5XdrqGJTj4aEevkphuzA5Q8',
    newPk: 'CfXTvsF6E7ysLg6HnnmNpnaoYSa59rHuDcupqeKdy5aJ',
    pythFeed: 'usd',
  },
  {
    name: 'SLCL/USD',
    pk: '5aX5yToaDTkWz6mWKTfC5M9HxwWDSrTopU3UHEVRwp6Y',
    newPk: 'BcL5gWHvG5Kmw9okPcAq3ccFm1f3vBUeyvjXwzLLJcd4',
    pythFeed: 'usd',
  },
  {
    name: 'JLP/USD',
    pk: 'pmHEXBam7kbmCCg5ED5V7RNMN8e34sKu338KeuFAGof',
    newPk: 'ASAKdrSoMew3GerohdwFp3bT6HJPUVt3bZgN3JKFvinS',
    pythFeed: 'usd',
  },
  {
    name: 'SLERF/USD',
    pk: '8LxP1juSh9RPMECQiTocqk8bZcrhhtqgUEk76y4AmE2K',
    newPk: 'Cewh5ybWrXDxBJ2s7ZVmQsJRXR3DdKKik9P91ymT4MQe',
    pythFeed: 'usd',
  },
  {
    name: 'BOME/USD',
    pk: 'JDj6n1iBeJUB54rNsmKw9ty2psAnkcXySLRshBWrYfGD',
    newPk: 'DNChSQVXuefoZzeQURJ3JE7r8MsQ2aB8f1TSV75BEGmX',
    pythFeed: 'usd',
  },
  {
    name: 'WEN/USD',
    pk: 'DzGXTYWCAsQhZbP3KGPeA8G8SA7ynTNntrtDWZ2kPE8n',
    newPk: '4ctjNHu5xTrurB4wFCiZs8puC5UmQ4bFfAKVUuUG7E9Z',
    pythFeed: 'usd',
  },
  {
    name: 'MEW/USD',
    pk: 'BogEXWj8YcrXV31D7FzzUmTCS1hFHfRGZN7rnVN1Vcqe',
    newPk: '2GNGnpmku4Aw7ku3Xa3fZyPugcDg1GADSzu2C1pWXB7E',
    pythFeed: 'usd',
  },
  {
    name: 'MOTHER/USD',
    pk: '273kfU17iwVVgYCRrRR9rFmT2R8FysSPQ2jETuix2gpd',
    newPk: 'HcWVxt6fwp2i149GunKohiZCi9jz3tqXyD31drn9USoX',
    pythFeed: 'usd',
  },
  {
    name: 'USDY/USD',
    pk: '5RKJ9unGQQhHezsNg7wshfJD4c5jJ64iXYu1nk6PJ5fb',
    newPk: '234oAERsti3gMYH8DNXxawKm7jGLwqgSsGB5Cz72KeXU',
    pythFeed: 'usd',
  },
  {
    name: 'PUPS/USD',
    pk: 'ApF6hz2W7FSKMgmmpWxLm6ijA2J5vU2XDBaBLvjbyMbm',
    newPk: 'zH9ZpmU6xb6G2NzbujZthvUVdFxwAmbAgRrVX93gUX1',
    pythFeed: 'usd',
  },
  {
    name: 'GECKO/USD',
    pk: 'ERWF6PnFCVPWeDM9VGCQDC7pASvVrCUwv9Tk3Mh3oXGG',
    newPk: 'CseiaHZ8rT2MaD2RFb924huBpkQhd5Gvxd8egmbKBqeK',
    pythFeed: 'usd',
  },
  {
    name: 'KMNO/USD',
    pk: 'H8oLEoDyvABEDmGmXQuuzvSPWAkr2f2GKytbXiGX9YUm',
    newPk: 'ELMSj3w18giUcfU7XHDwxQn8A4At4Ao8aadopP2ZvWpn',
    pythFeed: 'usd',
  },
  {
    name: 'INF/USD',
    pk: '6AQHz9mpGNjyVafcWdqzzgsJq14Cs8gG6MiQKmdAgCuP',
    newPk: '6dM4Wppuz8GtpAqd5xgd1abtXCd1VBfqJAkkhTYW3JpZ',
    pythFeed: 'usd',
  },
  {
    name: 'GME/USD',
    pk: 'B9BzQ6hBBFn3C6fsGsVwcFd1v5cdbAwi8bUNmL58Bx8z',
    newPk: '3zz1k5dcKVSkiFh3DRaTMsZbAckEk1DNiJrWUJKJw2Nr',
    pythFeed: 'usd',
  },
  {
    name: 'BILLY/USD',
    pk: 'DKt5kYg2wcY3SpbMZrYcJUg23mwEEQ2PsCioyPfcX633',
    newPk: 'BvNyTAZp8P1KXXxb8U28Za8XAJGR4CGagexcPoYYr3BE',
    pythFeed: 'usd',
  },
  {
    name: 'LNGCAT/USD',
    pk: 'H5DimRdrm4xjMMEzg574QKkfaHZcraGLqC85JJ4PBm58',
    newPk: '4CgXzP6uCV829KtrvaXY6UuBJz6M4YjHy4YWzo4hanb9',
    pythFeed: 'usd',
  },
  {
    name: 'CROWN/USD',
    pk: 'RMy7j7BUNxhE4Njgq69KC6ZLzZEpKWoKSp4Y5JQPQLE',
    newPk: 'HJQfdAcZGgo9eJXkzPebcARe7Ptxv1G5xjcucZMvNSpt',
    pythFeed: 'usd',
  },
  {
    name: 'OPOS/USD',
    pk: '3nM4m9FX1ENp3vfbJKMK6mELH7PSPQX5apzonHB9VZeL',
    newPk: '59rJDd4xxZFsouZ73sTj3ysnNPCTmunTiThS21NHEazz',
    pythFeed: 'usd',
  },
  {
    name: 'KIN/USD',
    pk: 'FS4pE37HCGtwjrf4g3G4YfdfRN64nTm1z8iFNHyjZHB5',
    newPk: 'HHkJVKgbueG4eoeHf3WCXSuG3MVAqq2MwAaeiZBkTc1g',
    pythFeed: 'usd',
  },
];

export const SB_ON_DEMAND_LST_FALLBACK_ORACLES = [
  ['JSOL/USD', 'Dnn9fKeB3rA2bor6Fys7FBPqXneAK8brxNfsBfZ32939'],
  ['compassSOL/USD', 'GzBpasKMSTLkytXpyo6NesDGpe2mLjPSovECWsebQpu5'],
  ['dualSOL/USD', 'D6UqFgtVC1yADBxw2EZFmUCTNuoqFoUXD3NW4NqRn8v3'],
  ['hubSOL/USD', '7LRVXc8zdPpzXNdknU2kRTYt7BizYs7BaM6Ft2zv8E4h'],
  ['hubSOL/USD', '137fd2LnDEPVAALhPFjRyvh2MD9DxSHPFaod7a5tmMox'],
  ['digitSOL/USD', '7skmP8qLf8KKJ61cpPiw91GXYfoGvGWekzSDQ78T3z1f'],
  ['mangoSOL/USD', '7pD4Y1hCsU4M6rfoJvL8fAmmrB2LwrJYxvWz4S6Cc24T'],
];
