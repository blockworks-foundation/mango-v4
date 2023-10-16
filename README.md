_work in progress_

## License

See the LICENSE file.

The majority of this repo is MIT licensed, but some parts needed for compiling
the solana program are under GPL.

All GPL code is gated behind the `enable-gpl` feature. If you use the `mango-v4`
crate as a dependency with the `client` or `cpi` features, you use only MIT
parts of it.

The intention is for you to be able to depend on the `mango-v4` crate for
building closed-source tools and integrations, including other solana programs
that call into the mango program.

But deriving a solana program with similar functionality to the mango program
from this codebase would require the changes and improvements to stay publicly
available under GPL.

## Development

See DEVELOPING.md

### Dependencies

- rust version 1.69.0
- solana-cli 1.16.7
- anchor-cli 0.28.0
- npm 8.1.2
- node v16.13.1

### Deployments

- devnet: 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg
- mainnet-beta: 4MangoMjqJ2firMokCjjGgoK8d4MXcrgL7XJaL3w6fVg
- primary mango group on mainnet-beta: 78b8f4cGCwmZ9ysPFMWLaLTkkaYnUjwMJYStWe5RTSSX

### Release

For program deployment, see RELEASING.md.

Here are steps followed while performing a npm package release
note: the UI currently uses code directly from github, pointing to the ts-client branch

- use `yarn publish` to release a new package, ensure compatibility with program release to mainnet-beta
- fix the tag auto added by yarn to match our internal convention, see script `fix-npm-tag.sh`, tags should look like this e.g.`npm-v0.0.1`, note: the npm package version/tag should not necessarily match the latest program deployment


### FAQ (on-chain program)

#### Not enough CU
*(CU: Compute Unit)*

Although you want to keep an eye on the CU limit to make sure the program does not consume to many CUs, program integration test will fail unexpectedly if limit is too low when developing.

Use this method to change the CU limit:
```rust
test.set_compute_max_units(500_000);
```


#### Wrong Rust version


| Rust Version                        | Compatibility | Solana Version |
|-------------------------------------|---------------|---------------:|
| 1.69-x86_64-apple-darwin            | works         |         1.16.x |
| 1.66.1-x86_64-apple-darwin          | works         |         1.14.x |
| stable-x86_64-apple-darwin (1.69.0) | broken        |         1.14.x |
| 1.68.2-x86_64-apple-darwin          | broken        |         1.14.x |
| 1.67.1-x86_64-apple-darwin          | broken        |         1.14.x |

Check/set the version:
```
rustup override list
# WRONG:
# /Users/username/code/mango-v4               1.69-aarch64-apple-darwin

rustup override set 1.69-x86_64
# CORRECT:
# /Users/username/code/mango-v4               1.69-x86_64
```

The Rust version requirements for *Solana* and *Mango V4* can be found in the respective `rust-toolchain.toml` file.

On *MacOS* its also required to use the x86 toolchain.

#### Failed to create BPF VM: syscall #4389198304 was not registered before bind
This error results from wrong Rust version!

*Solution*: see [Section about Rust version](#wrong-rust-version)


#### attempt to compute 0_usize - 1_usize, which would overflow
Multiple errors of this style are shown:
```
666 | const_assert_eq!(size_of::<AnyNode>(), size_of::<LeafNode>());
| ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ attempt to compute `0_usize - 1_usize`, which would overflow
```

Note: the assertion error text is misleading, the problem is not the overflow but the inequality of the two constants which cannot be optimized to zero by compiler.

The problem appears on non-x86-targets (typically on MacOS).

*Solution*:
Inspect your rust version and make sure you use the x86 toolchain - see [Section about Rust version](#wrong-rust-version)


#### Failed custom program error: 0x65
(*anchor-related issue*)

*Solution*:
Make sure that instruction layout (defined using Anchor) used by client/cpi matches the program version.


#### Program failed to complete: Access violation in stack frame 5 at address 0x200005ff8 of size 8 by instruction #6340
No real solution yet! Please open an issue if you know how to fix this.

*Workaround*:
Experiment with `#[inline(never)]` on methods.


#### instruction tries to borrow reference for an account which is already borrowed
(*anchor-related issue*)

*Solution*:
Revisit the account references retrieved by explicit calls to `.load`/`.load_mut`. See doc on [`account_loader.rs`](https://github.com/coral-xyz/anchor/blob/fc9fd6d24b9be84abb2f40e47ed3faf7b11864ae/lang/src/accounts/account_loader.rs#L34).


#### Running on-chain program crash silently on VM when using rust logging

*Solution*:
For Solana programs you must use `msg!` for logging and ***not*** `info!`/`debug!`/`error!`/`warn!`/`trace!` from the `log` crate.


#### Exceeded max BPF to BPF call depth of 64 at instruction #4605 (stack size exceeded)

*Solution*: Fix the Rust version - see [Section about Rust version](#wrong-rust-version)


#### Program failed to complete: Invoked an instruction with data that is too large (12884928151 > 10240)

*Solution*:
Fix the Solana version (1.15.2 worked).


#### Error: Function _ZN86_$LT$switchboard_v2..aggregator..AggregatorAccountData$u20$as$u20$core..fmt..Debug$GT$3fmt17h22734c1ad9ed3ea8E Stack offset of 4128 exceeded max offset of 4096 by 32 bytes, please minimize large stack variables
This error results from very large Rust structures passed as parameters.

No solution yet! Please open an issue if you know how to fix this.

As of now the problem can be ignored as long as the method is not called.


#### Syscall lib binding fails for invalid solana version combinations

*Solution*:
Make sure the Solana version of the program is compatible with the validator version.

Check the Solana Feature Gate status and the [Solana Feature Gate Activation Schedule](https://github.com/solana-labs/solana/wiki/Feature-Gate-Activation-Schedule):
```
solana feature status

# Output (truncated):
Feature                                      | Status                  | Activation Slot | Description
7rcw5UtqgDTBBv2EcynNfYckgdAaH1MAsCjKgXMkN7Ri | active since epoch 516  | 217388260       | enable curve25519 syscalls
5x3825XS7M2A3Ekbn5VGGkvFoAg5qrRWkTrY4bARP1GL | inactive                | NA              | enable bpf upgradeable loader SetAuthorityChecked instruction #28424

Tool Feature Set: 4033350765
Software Version                    Feature Set   Stake     RPC
1.17.1, 1.17.0                       2241946265   2.20%   1.75%
1.16.17, 1.16.16, 1.16.15, 1.16.14   4033350765   9.12%   2.56%  <-- me
1.16.13                              3949673676   0.41%   0.27%
```




#### Large Cpi Contexts (e.g. Serum) will eventually overflow the stack
No real solution yet!

*Workaround*: Try to make method bodies smaller by extracting code into separate methods.


#### No log output or too noisy log output

*Solution*: Configure the log filter:
```
solana_rbpf=trace, solana_runtime::message_processor=debug, solana_runtime::system_instruction_processor=info, solana_program_test=debug, test_all=debug
```


#### Error: toolchain 'bpf' is not installed
Solana programs require the special Rust toolchain to be installed.


*Solution*:
Use the proper toolchain when invoking `cargo` build or test:
```
# 1.14.x
cargo +bpf build-bpf
# 1.15.x
cargo +sbf build-sbf
# 1.16.x
cargo +solana build-sbf
```

Make sure the toolchain is installed on your system:
```
rustup toolchain list -v
```

If toolchain is missing use the *Solana Install Tool* [here](https://docs.solana.com/cli/install-solana-cli-tools) to install it.


