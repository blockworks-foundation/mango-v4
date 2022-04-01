### Development

- rust version 1.59.0 (9d1b2106e 2022-02-23)
- solana-cli 1.9.5
- anchor-cli 0.22.0
- npm 8.1.2
- node v16.13.1

Devnet deployment - m43thNJ58XCjL798ZSq6JGAG1BnWskhdq5or6kcnfsD

TS client - see ts dir, and ts/example.ts, run as `yarn ts-node ts/example.ts`

### Module structure

As and when we move to a more complete project, we should think of having multiple modules
e.g. core/shared, spot, perpetuals, etc., and then each would have its own instructions
and state sub module. Goal is that new contributors find relevant code easily and can navigate
easily.

```
programs
└── mango-v4
    ├── Cargo.toml
    ├── Xargo.toml
    └── src
    │    ├── error.rs
    │    ├── instructions # instructions go here, each instruction gets an individual file
    │    │   ├── initialiaze.rs
    │    │   └── mod.rs
    │    ├── lib.rs
    │    └── state # state goes here, each account state gets an individual file
    │       └── mod.rs
    └── tests # rust tests, TODO
```

### How to open and manage pull requests

- when in doubt dont squash commits, specially when merge request is very large, specially if your branch contains unrelated commits
- use the why along with what for commit messages, code comments, makes it easy to understand the context
- add descriptions to your merge requests if they are non trivial, helps code reviewer watch out for things, understand the motivation for the merge request
