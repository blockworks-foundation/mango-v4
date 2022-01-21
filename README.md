### Development
* rustc 1.57.0 (f1edd0429 2021-11-29)
* anchor-cli 0.20.1
* npm 8.1.2
* node v16.13.1

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

