# Important Notice
Please **DO NOT** create a GitHub issue to report a security problem. Instead, please send an email to hello@blockworks.foundation with a detailed description of the attack vector and security risk you have identified.

# Bug Bounty Overview
Mango Markets offers bug bounties for Mango Markets' on-chain program code; UI only bugs are omitted.

|Severity|Description|Bug Bounty|
|-----------|--------------|-------------|
|Critical|Bugs that freeze user funds or drain the contract's holdings or involve theft of funds without user signatures|10% of the value of the hack up to $1,000,000|
|High|Bugs that could temporarily freeze user funds or incorrectly assign value to user funds|$10,000 to $50,000 per bug, assessed on a case by case basis|
|Medium/Low|Bugs that don't threaten user funds|$1,000 to $5,000 per bug, assessed on a case by case basis|

The severity guidelines are based on [Immunefi's classification system](https://immunefi.com/severity-updated/). 
Note that these are simply guidelines for the severity of the bugs. Each bug bounty submission will be evaluated on a case-by-case basis.

## Submission
Please email hello@blockworks.foundation with a detailed description of the attack vector. For critical and moderate bugs, we require a proof of concept done on a privately deployed mainnet contract. We will reach out in 1 business day with additional questions or next steps on the bug bounty.

## Bug Bounty Payment
Bug bounties will be paid in USDC or locked MNGO, after a DAO vote. The Mango DAO has never refused a valid bug bounty so far.

## Invalid Bug Bounties
The following are out of scope for the bug bounty:
1. Attacks that the reporter has already exploited themselves, leading to damage.
2. Attacks requiring access to leaked keys/credentials.
3. Attacks requiring access to privileged addresses (governance, admin).
4. Incorrect data supplied by third party oracles (this does not exclude oracle manipulation/flash loan attacks).
5. Lack of liquidity.
6. Third party, off-chain bot errors (for instance bugs with an arbitrage bot running on the smart contracts).
7. Best practice critiques.
8. Sybil attacks.