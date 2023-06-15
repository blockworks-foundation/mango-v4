## Disclaimer

The following open source code contains an automated bot designed to perform liquidations. Please note that the use of this code is at your own risk and responsibility.

1. No Warranty: The code is provided "as is," without any warranty or guarantee of any kind, express or implied. The developers and contributors of this code do not make any representations or warranties regarding its accuracy, reliability, or functionality. The use of this code is solely at your own risk.

2. Limitation of Liability: In no event shall the developers and contributors of this code be liable for any direct, indirect, incidental, special, exemplary, or consequential damages (including, but not limited to, procurement of substitute goods or services, loss of use, data, or profits, or business interruption) arising in any way out of the use, inability to use, or the results of the use of this code, even if advised of the possibility of such damages.

3. Compliance with Laws: It is your responsibility to ensure that the use of this code complies with all applicable laws, regulations, and policies. The developers and contributors of this code shall not be held responsible for any illegal or unauthorized use of the code.

4. User Accountability: You are solely responsible for any actions performed using this code. The developers and contributors of this code shall not be held liable for any misuse, harm, or damages caused by the bot or its actions.

5. Security Considerations: While efforts have been made to ensure the security of this code, the developers and contributors do not guarantee its absolute security. It is recommended that you take appropriate measures to secure the code and any associated systems from potential vulnerabilities or threats.

6. Third-Party Dependencies: This code may rely on third-party libraries, frameworks, or APIs. The developers and contributors of this code are not responsible for the functionality, availability, or security of any third-party components.

By using this open source code, you acknowledge and agree to the above disclaimer. If you do not agree with any part of the disclaimer, refrain from using the code.

---


Two branches are relevant here:

- `devnet`: bleeding edge, may be unstable, could be incompatible with deployed program
- `main`: stable, currently running on the `mainnet-beta` cluster

## Setup Environment

### .env Config file:

A `.env` file can be used to configure the liquidator setup. See `.env.example` for a example.

The environment variables required are

- `LIQOR_MANGO_ACCOUNT` - public key of the mango account
- `LIQOR_OWNER` - private key of the owner of the mango account
- `RPC_URL` - RPC cluster url
- `SERUM_PROGRAM` - the Openbook program Id the mango group is configured with e.g. primary mango group `78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX` is configured to work with `srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX`

more advanced parameters

- `MIN_HEALTH_RATIO` - minimum health ratio the liquidator should retain (default 50%)
- `REBALANCE_SLIPPAGE_BPS` - slippage liquidator should tolerate when offloading tokens (default 100)

```shell
cargo run --bin liquidator
```

There is also a dockerfile `Dockerfile.liquidator` available in case one wants to run this in a containerized environment.
