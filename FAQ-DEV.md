# FAQ (on-chain program)


## Not enough CU in program tests

Although you want to keep an eye on the CU (compute unit) limit to make sure the program does not consume too many CUs, program integration test will fail unexpectedly if limit is too low when developing.

Use this method to change the CU limit:
```rust
test_builder.test().set_compute_max_units(500_000);
```


## Wrong Rust version

The Rust version requirements for *Solana* and *Mango V4* can be found in the respective `rust-toolchain.toml` file
and tools like *cargo* pick up on them automatically.

Check with this command:
```
rustup show
```

On *MacOS* its also required to use the x86 toolchain.

```
rustup override set 1.69-x86_64
# CORRECT:
# /Users/username/code/mango-v4               1.69-x86_64
```


## Failed to create BPF VM: syscall #4389198304 was not registered before bind

This error results from wrong Rust version!

*Solution*: see [Section about Rust version](#wrong-rust-version)


## attempt to compute 0_usize - 1_usize, which would overflow

Multiple errors of this style are shown:
```
666 | const_assert_eq!(size_of::<AnyNode>(), size_of::<LeafNode>());
| ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ attempt to compute `0_usize - 1_usize`, which would overflow
```

Note: the assertion error text is misleading, the problem is not the overflow but the static assert about struct sizes failing.

The problem appears on non-x86-targets (typically on MacOS) that produce different struct layouts than are expected on-chain. For performance reasons the mango program wants to do zero-copy deserialization of account state and for that it's essential to be able to define structs that match the on-chain data layout. The assert failing means that that didn't work on this target.

*Solution*:
For now, the only solution is to use an x86 toolchain - see [Section about Rust version](#wrong-rust-version)


## Failed custom program error: 0x65

(*anchor-related issue*)

*Solution*:
Make sure that instruction layout (defined using Anchor) used by client/cpi matches the program version.


## Program failed to complete: Access violation in stack frame 5 at address 0x200005ff8 of size 8 by instruction #6340

The solana runtime provides limited stack space to each callframe and likely you are exceeding it.

*Workaround*:
Avoid putting large data on the stack. Experiment with `#[inline(never)]` on methods.


## instruction tries to borrow reference for an account which is already borrowed

(*anchor-related issue*)

*Solution*:
Revisit the account references retrieved by explicit calls to `.load`/`.load_mut`. See doc on [`account_loader.rs`](https://github.com/coral-xyz/anchor/blob/fc9fd6d24b9be84abb2f40e47ed3faf7b11864ae/lang/src/accounts/account_loader.rs#L34).


## Running on-chain program crashes silently on VM when using rust logging

*Solution*:
For Solana programs you must use `msg!` for logging and ***not*** `info!`/`debug!`/`error!`/`warn!`/`trace!` from the `log` crate.


## Exceeded max BPF to BPF call depth of 64 at instruction #4605 (stack size exceeded)

*Solution*: Fix the Rust version - see [Section about Rust version](#wrong-rust-version)


## Program failed to complete: Invoked an instruction with data that is too large (12884928151 > 10240)

*Solution*:
Use the Solana version 1.16.x or later.


## Error: Function _ZN86_$LT$switchboard_v2..aggregator..AggregatorAccountData$u20$as$u20$core..fmt..Debug$GT$3fmt17h22734c1ad9ed3ea8E Stack offset of 4128 exceeded max offset of 4096 by 32 bytes, please minimize large stack variables

This error results from very large Rust structures passed as parameters.

No solution yet! Please open an issue if you know how to fix this.

As of now the problem can be ignored as long as the method is not called.


## Syscall lib binding fails for invalid solana version combinations

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


## No log output or too noisy log output

*Solution*: Configure the log filter.

Start with this filter using the `RUST_LOG` environment variable:
```
solana_rbpf=trace, solana_runtime::message_processor=debug, solana_runtime::system_instruction_processor=info, solana_program_test=debug, test_all=debug
```


## Error: toolchain 'bpf' is not installed

Solana programs require the special Rust toolchain to be installed.
Normally the solana tooling installs it automatically on first use and `cargo build-sbf` will just work.

*Workaround*:
Explicitly mention the toolchain when invoking `cargo` build or test:
```
# 1.16.x
cargo +solana build-sbf
```

Make sure the toolchain is installed on your system:
```
rustup toolchain list -v
```

If toolchain is missing use the *Solana Install Tool* [here](https://docs.solana.com/cli/install-solana-cli-tools) to install it.


