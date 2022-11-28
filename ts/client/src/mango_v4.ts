export type MangoV4 = {
  "version": "0.1.0",
  "name": "mango_v4",
  "instructions": [
    {
      "name": "groupCreate",
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Group"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "creator"
              },
              {
                "kind": "arg",
                "type": "u32",
                "path": "group_num"
              }
            ]
          }
        },
        {
          "name": "creator",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "insuranceMint",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "insuranceVault",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "InsuranceVault"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              }
            ]
          }
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "groupNum",
          "type": "u32"
        },
        {
          "name": "testing",
          "type": "u8"
        },
        {
          "name": "version",
          "type": "u8"
        }
      ]
    },
    {
      "name": "groupEdit",
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "adminOpt",
          "type": {
            "option": "publicKey"
          }
        },
        {
          "name": "fastListingAdminOpt",
          "type": {
            "option": "publicKey"
          }
        },
        {
          "name": "testingOpt",
          "type": {
            "option": "u8"
          }
        },
        {
          "name": "versionOpt",
          "type": {
            "option": "u8"
          }
        }
      ]
    },
    {
      "name": "groupClose",
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "insuranceVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "solDestination",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "tokenRegister",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "mint",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Bank"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "const",
                "type": "u32",
                "value": 0
              }
            ]
          }
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Vault"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "const",
                "type": "u32",
                "value": 0
              }
            ]
          }
        },
        {
          "name": "mintInfo",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "MintInfo"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "account": "Mint",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "tokenIndex",
          "type": "u16"
        },
        {
          "name": "name",
          "type": "string"
        },
        {
          "name": "oracleConfig",
          "type": {
            "defined": "OracleConfigParams"
          }
        },
        {
          "name": "interestRateParams",
          "type": {
            "defined": "InterestRateParams"
          }
        },
        {
          "name": "loanFeeRate",
          "type": "f32"
        },
        {
          "name": "loanOriginationFeeRate",
          "type": "f32"
        },
        {
          "name": "maintAssetWeight",
          "type": "f32"
        },
        {
          "name": "initAssetWeight",
          "type": "f32"
        },
        {
          "name": "maintLiabWeight",
          "type": "f32"
        },
        {
          "name": "initLiabWeight",
          "type": "f32"
        },
        {
          "name": "liquidationFee",
          "type": "f32"
        },
        {
          "name": "minVaultToDepositsRatio",
          "type": "f64"
        },
        {
          "name": "netBorrowsWindowSizeTs",
          "type": "u64"
        },
        {
          "name": "netBorrowsLimitNative",
          "type": "i64"
        }
      ]
    },
    {
      "name": "tokenRegisterTrustless",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "fastListingAdmin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "mint",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Bank"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "const",
                "type": "u32",
                "value": 0
              }
            ]
          }
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Vault"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "const",
                "type": "u32",
                "value": 0
              }
            ]
          }
        },
        {
          "name": "mintInfo",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "MintInfo"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "account": "Mint",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "tokenIndex",
          "type": "u16"
        },
        {
          "name": "name",
          "type": "string"
        }
      ]
    },
    {
      "name": "tokenEdit",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "mintInfo",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "oracleOpt",
          "type": {
            "option": "publicKey"
          }
        },
        {
          "name": "oracleConfigOpt",
          "type": {
            "option": {
              "defined": "OracleConfigParams"
            }
          }
        },
        {
          "name": "groupInsuranceFundOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "interestRateParamsOpt",
          "type": {
            "option": {
              "defined": "InterestRateParams"
            }
          }
        },
        {
          "name": "loanFeeRateOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "loanOriginationFeeRateOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "maintAssetWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "initAssetWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "maintLiabWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "initLiabWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "liquidationFeeOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "stablePriceDelayIntervalSecondsOpt",
          "type": {
            "option": "u32"
          }
        },
        {
          "name": "stablePriceDelayGrowthLimitOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "stablePriceGrowthLimitOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "minVaultToDepositsRatioOpt",
          "type": {
            "option": "f64"
          }
        },
        {
          "name": "netBorrowsLimitNativeOpt",
          "type": {
            "option": "i64"
          }
        },
        {
          "name": "netBorrowsWindowSizeTsOpt",
          "type": {
            "option": "u64"
          }
        }
      ]
    },
    {
      "name": "tokenAddBank",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "mint",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "existingBank",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Bank"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "arg",
                "type": "u32",
                "path": "bank_num"
              }
            ]
          }
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Vault"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "arg",
                "type": "u32",
                "path": "bank_num"
              }
            ]
          }
        },
        {
          "name": "mintInfo",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "tokenIndex",
          "type": "u16"
        },
        {
          "name": "bankNum",
          "type": "u32"
        }
      ]
    },
    {
      "name": "tokenDeregister",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "mintInfo",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "dustVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "solDestination",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "tokenUpdateIndexAndRate",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "mintInfo",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "instructions",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "accountCreate",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "MangoAccount"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "owner"
              },
              {
                "kind": "arg",
                "type": "u32",
                "path": "account_num"
              }
            ]
          }
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "accountNum",
          "type": "u32"
        },
        {
          "name": "tokenCount",
          "type": "u8"
        },
        {
          "name": "serum3Count",
          "type": "u8"
        },
        {
          "name": "perpCount",
          "type": "u8"
        },
        {
          "name": "perpOoCount",
          "type": "u8"
        },
        {
          "name": "name",
          "type": "string"
        }
      ]
    },
    {
      "name": "accountExpand",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "tokenCount",
          "type": "u8"
        },
        {
          "name": "serum3Count",
          "type": "u8"
        },
        {
          "name": "perpCount",
          "type": "u8"
        },
        {
          "name": "perpOoCount",
          "type": "u8"
        }
      ]
    },
    {
      "name": "accountEdit",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "nameOpt",
          "type": {
            "option": "string"
          }
        },
        {
          "name": "delegateOpt",
          "type": {
            "option": "publicKey"
          }
        }
      ]
    },
    {
      "name": "accountClose",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "solDestination",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "stubOracleCreate",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "StubOracle"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "account": "Mint",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "mint",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "price",
          "type": {
            "defined": "I80F48"
          }
        }
      ]
    },
    {
      "name": "stubOracleClose",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "oracle",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "solDestination",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "stubOracleSet",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "oracle",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "price",
          "type": {
            "defined": "I80F48"
          }
        }
      ]
    },
    {
      "name": "tokenDeposit",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "tokenAccount",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "tokenDepositIntoExisting",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "tokenAccount",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "tokenWithdraw",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "tokenAccount",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        },
        {
          "name": "allowBorrow",
          "type": "bool"
        }
      ]
    },
    {
      "name": "flashLoanBegin",
      "accounts": [
        {
          "name": "account",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "instructions",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Instructions Sysvar for instruction introspection"
          ]
        }
      ],
      "args": [
        {
          "name": "loanAmounts",
          "type": {
            "vec": "u64"
          }
        }
      ]
    },
    {
      "name": "flashLoanEnd",
      "accounts": [
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "flashLoanType",
          "type": {
            "defined": "FlashLoanType"
          }
        }
      ]
    },
    {
      "name": "healthRegionBegin",
      "accounts": [
        {
          "name": "instructions",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Instructions Sysvar for instruction introspection"
          ]
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "healthRegionEnd",
      "accounts": [
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "serum3RegisterMarket",
      "docs": [
        "",
        "Serum",
        ""
      ],
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "serumProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumMarketExternal",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumMarket",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Serum3Market"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "serum_market_external"
              }
            ]
          }
        },
        {
          "name": "indexReservation",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Serum3Index"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "market_index"
              }
            ]
          }
        },
        {
          "name": "quoteBank",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "baseBank",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "marketIndex",
          "type": "u16"
        },
        {
          "name": "name",
          "type": "string"
        }
      ]
    },
    {
      "name": "serum3DeregisterMarket",
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "serumMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "indexReservation",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "solDestination",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "serum3CreateOpenOrders",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumMarketExternal",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Serum3OO"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "account"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "serum_market"
              }
            ]
          }
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "serum3CloseOpenOrders",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumMarketExternal",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "solDestination",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "serum3PlaceOrder",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumMarketExternal",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketBids",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketAsks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketEventQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketRequestQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketBaseVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketQuoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketVaultSigner",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "needed for the automatic settle_funds call"
          ]
        },
        {
          "name": "payerBank",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The bank that pays for the order, if necessary"
          ]
        },
        {
          "name": "payerVault",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The bank vault that pays for the order, if necessary"
          ]
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "side",
          "type": {
            "defined": "Serum3Side"
          }
        },
        {
          "name": "limitPrice",
          "type": "u64"
        },
        {
          "name": "maxBaseQty",
          "type": "u64"
        },
        {
          "name": "maxNativeQuoteQtyIncludingFees",
          "type": "u64"
        },
        {
          "name": "selfTradeBehavior",
          "type": {
            "defined": "Serum3SelfTradeBehavior"
          }
        },
        {
          "name": "orderType",
          "type": {
            "defined": "Serum3OrderType"
          }
        },
        {
          "name": "clientOrderId",
          "type": "u64"
        },
        {
          "name": "limit",
          "type": "u16"
        }
      ]
    },
    {
      "name": "serum3CancelOrder",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumMarketExternal",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketBids",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketAsks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketEventQueue",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "side",
          "type": {
            "defined": "Serum3Side"
          }
        },
        {
          "name": "orderId",
          "type": "u128"
        }
      ]
    },
    {
      "name": "serum3CancelAllOrders",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumMarketExternal",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketBids",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketAsks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketEventQueue",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "limit",
          "type": "u8"
        }
      ]
    },
    {
      "name": "serum3SettleFunds",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumMarketExternal",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketBaseVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketQuoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketVaultSigner",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "needed for the automatic settle_funds call"
          ]
        },
        {
          "name": "quoteBank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "quoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "baseBank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "baseVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "serum3LiqForceCancelOrders",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumMarketExternal",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketBids",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketAsks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketEventQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketBaseVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketQuoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketVaultSigner",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "quoteBank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "quoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "baseBank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "baseVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "limit",
          "type": "u8"
        }
      ]
    },
    {
      "name": "liqTokenWithToken",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "assetTokenIndex",
          "type": "u16"
        },
        {
          "name": "liabTokenIndex",
          "type": "u16"
        },
        {
          "name": "maxLiabTransfer",
          "type": {
            "defined": "I80F48"
          }
        }
      ]
    },
    {
      "name": "liqTokenBankruptcy",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "liabMintInfo",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "quoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "insuranceVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "maxLiabTransfer",
          "type": {
            "defined": "I80F48"
          }
        }
      ]
    },
    {
      "name": "tokenLiqWithToken",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "assetTokenIndex",
          "type": "u16"
        },
        {
          "name": "liabTokenIndex",
          "type": "u16"
        },
        {
          "name": "maxLiabTransfer",
          "type": {
            "defined": "I80F48"
          }
        }
      ]
    },
    {
      "name": "tokenLiqBankruptcy",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "liabMintInfo",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "quoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "insuranceVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "maxLiabTransfer",
          "type": {
            "defined": "I80F48"
          }
        }
      ]
    },
    {
      "name": "perpCreateMarket",
      "docs": [
        "",
        "Perps",
        ""
      ],
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "PerpMarket"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "perp_market_index"
              }
            ]
          }
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Accounts are initialised by client,",
            "anchor discriminator is set first when ix exits,"
          ]
        },
        {
          "name": "eventQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "perpMarketIndex",
          "type": "u16"
        },
        {
          "name": "name",
          "type": "string"
        },
        {
          "name": "oracleConfig",
          "type": {
            "defined": "OracleConfigParams"
          }
        },
        {
          "name": "baseDecimals",
          "type": "u8"
        },
        {
          "name": "quoteLotSize",
          "type": "i64"
        },
        {
          "name": "baseLotSize",
          "type": "i64"
        },
        {
          "name": "maintAssetWeight",
          "type": "f32"
        },
        {
          "name": "initAssetWeight",
          "type": "f32"
        },
        {
          "name": "maintLiabWeight",
          "type": "f32"
        },
        {
          "name": "initLiabWeight",
          "type": "f32"
        },
        {
          "name": "liquidationFee",
          "type": "f32"
        },
        {
          "name": "makerFee",
          "type": "f32"
        },
        {
          "name": "takerFee",
          "type": "f32"
        },
        {
          "name": "minFunding",
          "type": "f32"
        },
        {
          "name": "maxFunding",
          "type": "f32"
        },
        {
          "name": "impactQuantity",
          "type": "i64"
        },
        {
          "name": "groupInsuranceFund",
          "type": "bool"
        },
        {
          "name": "trustedMarket",
          "type": "bool"
        },
        {
          "name": "feePenalty",
          "type": "f32"
        },
        {
          "name": "settleFeeFlat",
          "type": "f32"
        },
        {
          "name": "settleFeeAmountThreshold",
          "type": "f32"
        },
        {
          "name": "settleFeeFractionLowHealth",
          "type": "f32"
        },
        {
          "name": "settleTokenIndex",
          "type": "u16"
        },
        {
          "name": "settlePnlLimitFactor",
          "type": "f32"
        },
        {
          "name": "settlePnlLimitFactorWindowSizeTs",
          "type": "u64"
        }
      ]
    },
    {
      "name": "perpEditMarket",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "oracleOpt",
          "type": {
            "option": "publicKey"
          }
        },
        {
          "name": "oracleConfigOpt",
          "type": {
            "option": {
              "defined": "OracleConfigParams"
            }
          }
        },
        {
          "name": "baseDecimalsOpt",
          "type": {
            "option": "u8"
          }
        },
        {
          "name": "maintAssetWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "initAssetWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "maintLiabWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "initLiabWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "liquidationFeeOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "makerFeeOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "takerFeeOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "minFundingOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "maxFundingOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "impactQuantityOpt",
          "type": {
            "option": "i64"
          }
        },
        {
          "name": "groupInsuranceFundOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "trustedMarketOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "feePenaltyOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "settleFeeFlatOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "settleFeeAmountThresholdOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "settleFeeFractionLowHealthOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "stablePriceDelayIntervalSecondsOpt",
          "type": {
            "option": "u32"
          }
        },
        {
          "name": "stablePriceDelayGrowthLimitOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "stablePriceGrowthLimitOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "settlePnlLimitFactorOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "settlePnlLimitFactorWindowSizeTs",
          "type": {
            "option": "u64"
          }
        }
      ]
    },
    {
      "name": "perpCloseMarket",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "eventQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "solDestination",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "perpDeactivatePosition",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "perpPlaceOrder",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "eventQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "side",
          "type": {
            "defined": "Side"
          }
        },
        {
          "name": "priceLots",
          "type": "i64"
        },
        {
          "name": "maxBaseLots",
          "type": "i64"
        },
        {
          "name": "maxQuoteLots",
          "type": "i64"
        },
        {
          "name": "clientOrderId",
          "type": "u64"
        },
        {
          "name": "orderType",
          "type": {
            "defined": "PlaceOrderType"
          }
        },
        {
          "name": "reduceOnly",
          "type": "bool"
        },
        {
          "name": "expiryTimestamp",
          "type": "u64"
        },
        {
          "name": "limit",
          "type": "u8"
        }
      ]
    },
    {
      "name": "perpPlaceOrderPegged",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "eventQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "side",
          "type": {
            "defined": "Side"
          }
        },
        {
          "name": "priceOffsetLots",
          "type": "i64"
        },
        {
          "name": "pegLimit",
          "type": "i64"
        },
        {
          "name": "maxBaseLots",
          "type": "i64"
        },
        {
          "name": "maxQuoteLots",
          "type": "i64"
        },
        {
          "name": "clientOrderId",
          "type": "u64"
        },
        {
          "name": "orderType",
          "type": {
            "defined": "PlaceOrderType"
          }
        },
        {
          "name": "reduceOnly",
          "type": "bool"
        },
        {
          "name": "expiryTimestamp",
          "type": "u64"
        },
        {
          "name": "limit",
          "type": "u8"
        }
      ]
    },
    {
      "name": "perpCancelOrder",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "orderId",
          "type": "u128"
        }
      ]
    },
    {
      "name": "perpCancelOrderByClientOrderId",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "clientOrderId",
          "type": "u64"
        }
      ]
    },
    {
      "name": "perpCancelAllOrders",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "limit",
          "type": "u8"
        }
      ]
    },
    {
      "name": "perpCancelAllOrdersBySide",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "sideOption",
          "type": {
            "option": {
              "defined": "Side"
            }
          }
        },
        {
          "name": "limit",
          "type": "u8"
        }
      ]
    },
    {
      "name": "perpConsumeEvents",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "eventQueue",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "limit",
          "type": "u64"
        }
      ]
    },
    {
      "name": "perpUpdateFunding",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "perpSettlePnl",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "settler",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "settlerOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "accountA",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "accountB",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "settleBank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "settleOracle",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "perpSettleFees",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "settleBank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "settleOracle",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "maxSettleAmount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "perpLiqBasePosition",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "maxBaseTransfer",
          "type": "i64"
        }
      ]
    },
    {
      "name": "perpLiqForceCancelOrders",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "limit",
          "type": "u8"
        }
      ]
    },
    {
      "name": "perpLiqBankruptcy",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "settleBank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "settleVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "settleOracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "insuranceVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "maxLiabTransfer",
          "type": "u64"
        }
      ]
    },
    {
      "name": "altSet",
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "addressLookupTable",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "index",
          "type": "u8"
        }
      ]
    },
    {
      "name": "altExtend",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "payer",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "addressLookupTable",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "index",
          "type": "u8"
        },
        {
          "name": "newAddresses",
          "type": {
            "vec": "publicKey"
          }
        }
      ]
    },
    {
      "name": "computeAccountData",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "benchmark",
      "docs": [
        "",
        "benchmark",
        ""
      ],
      "accounts": [],
      "args": []
    }
  ],
  "accounts": [
    {
      "name": "bank",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "group",
            "type": "publicKey"
          },
          {
            "name": "name",
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          },
          {
            "name": "mint",
            "type": "publicKey"
          },
          {
            "name": "vault",
            "type": "publicKey"
          },
          {
            "name": "oracle",
            "type": "publicKey"
          },
          {
            "name": "oracleConfFilter",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "depositIndex",
            "docs": [
              "the index used to scale the value of an IndexedPosition",
              "TODO: should always be >= 0, add checks?"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "borrowIndex",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "cachedIndexedTotalDeposits",
            "docs": [
              "total deposits/borrows, only updated during UpdateIndexAndRate",
              "TODO: These values could be dropped from the bank, they're written in UpdateIndexAndRate",
              "and never read."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "cachedIndexedTotalBorrows",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "indexedDeposits",
            "docs": [
              "deposits/borrows for this bank",
              "",
              "Note that these may become negative. It's perfectly fine for users to borrow one one bank",
              "(increasing indexed_borrows there) and paying back on another (possibly decreasing indexed_borrows",
              "below zero).",
              "",
              "The vault amount is not deducable from these values.",
              "",
              "These become meaningful when summed over all banks (like in update_index_and_rate)."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "indexedBorrows",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "indexLastUpdated",
            "type": "u64"
          },
          {
            "name": "bankRateLastUpdated",
            "type": "u64"
          },
          {
            "name": "avgUtilization",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "adjustmentFactor",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "util0",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "rate0",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "util1",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "rate1",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maxRate",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "collectedFeesNative",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "loanOriginationFeeRate",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "loanFeeRate",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maintAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maintLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "liquidationFee",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "dust",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "flashLoanTokenAccountInitial",
            "type": "u64"
          },
          {
            "name": "flashLoanApprovedAmount",
            "type": "u64"
          },
          {
            "name": "tokenIndex",
            "type": "u16"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "mintDecimals",
            "type": "u8"
          },
          {
            "name": "bankNum",
            "type": "u32"
          },
          {
            "name": "oracleConfig",
            "type": {
              "defined": "OracleConfig"
            }
          },
          {
            "name": "stablePriceModel",
            "type": {
              "defined": "StablePriceModel"
            }
          },
          {
            "name": "minVaultToDepositsRatio",
            "type": "f64"
          },
          {
            "name": "netBorrowsWindowSizeTs",
            "type": "u64"
          },
          {
            "name": "lastNetBorrowsWindowStartTs",
            "type": "u64"
          },
          {
            "name": "netBorrowsLimitNative",
            "type": "i64"
          },
          {
            "name": "netBorrowsWindowNative",
            "type": "i64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                2136
              ]
            }
          }
        ]
      }
    },
    {
      "name": "group",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "creator",
            "type": "publicKey"
          },
          {
            "name": "groupNum",
            "type": "u32"
          },
          {
            "name": "admin",
            "type": "publicKey"
          },
          {
            "name": "fastListingAdmin",
            "type": "publicKey"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "insuranceVault",
            "type": "publicKey"
          },
          {
            "name": "insuranceMint",
            "type": "publicKey"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "testing",
            "type": "u8"
          },
          {
            "name": "version",
            "type": "u8"
          },
          {
            "name": "padding2",
            "type": {
              "array": [
                "u8",
                5
              ]
            }
          },
          {
            "name": "addressLookupTables",
            "type": {
              "array": [
                "publicKey",
                20
              ]
            }
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                1920
              ]
            }
          }
        ]
      }
    },
    {
      "name": "mangoAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "group",
            "type": "publicKey"
          },
          {
            "name": "owner",
            "type": "publicKey"
          },
          {
            "name": "name",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "delegate",
            "type": "publicKey"
          },
          {
            "name": "accountNum",
            "type": "u32"
          },
          {
            "name": "beingLiquidated",
            "docs": [
              "Tracks that this account should be liquidated until init_health >= 0.",
              "",
              "Normally accounts can not be liquidated while maint_health >= 0. But when an account",
              "reaches maint_health < 0, liquidators will call a liquidation instruction and thereby",
              "set this flag. Now the account may be liquidated until init_health >= 0.",
              "",
              "Many actions should be disabled while the account is being liquidated, even if",
              "its maint health has recovered to positive. Creating new open orders would, for example,",
              "confuse liquidators."
            ],
            "type": "u8"
          },
          {
            "name": "inHealthRegion",
            "docs": [
              "The account is currently inside a health region marked by HealthRegionBegin...HealthRegionEnd.",
              "",
              "Must never be set after a transaction ends."
            ],
            "type": "u8"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                1
              ]
            }
          },
          {
            "name": "netDeposits",
            "type": "i64"
          },
          {
            "name": "perpSpotTransfers",
            "type": "i64"
          },
          {
            "name": "healthRegionBeginInitHealth",
            "docs": [
              "Init health as calculated during HealthReginBegin, rounded up."
            ],
            "type": "i64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                240
              ]
            }
          },
          {
            "name": "headerVersion",
            "type": "u8"
          },
          {
            "name": "padding3",
            "type": {
              "array": [
                "u8",
                7
              ]
            }
          },
          {
            "name": "padding4",
            "type": "u32"
          },
          {
            "name": "tokens",
            "type": {
              "vec": {
                "defined": "TokenPosition"
              }
            }
          },
          {
            "name": "padding5",
            "type": "u32"
          },
          {
            "name": "serum3",
            "type": {
              "vec": {
                "defined": "Serum3Orders"
              }
            }
          },
          {
            "name": "padding6",
            "type": "u32"
          },
          {
            "name": "perps",
            "type": {
              "vec": {
                "defined": "PerpPosition"
              }
            }
          },
          {
            "name": "padding7",
            "type": "u32"
          },
          {
            "name": "perpOpenOrders",
            "type": {
              "vec": {
                "defined": "PerpOpenOrder"
              }
            }
          }
        ]
      }
    },
    {
      "name": "mintInfo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "group",
            "type": "publicKey"
          },
          {
            "name": "tokenIndex",
            "type": "u16"
          },
          {
            "name": "groupInsuranceFund",
            "type": "u8"
          },
          {
            "name": "padding1",
            "type": {
              "array": [
                "u8",
                5
              ]
            }
          },
          {
            "name": "mint",
            "type": "publicKey"
          },
          {
            "name": "banks",
            "type": {
              "array": [
                "publicKey",
                6
              ]
            }
          },
          {
            "name": "vaults",
            "type": {
              "array": [
                "publicKey",
                6
              ]
            }
          },
          {
            "name": "oracle",
            "type": "publicKey"
          },
          {
            "name": "registrationTime",
            "type": "u64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                2560
              ]
            }
          }
        ]
      }
    },
    {
      "name": "stubOracle",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "group",
            "type": "publicKey"
          },
          {
            "name": "mint",
            "type": "publicKey"
          },
          {
            "name": "price",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "lastUpdated",
            "type": "i64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                128
              ]
            }
          }
        ]
      }
    },
    {
      "name": "orderbook",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "bids",
            "type": {
              "defined": "BookSide"
            }
          },
          {
            "name": "asks",
            "type": {
              "defined": "BookSide"
            }
          }
        ]
      }
    },
    {
      "name": "eventQueue",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "header",
            "type": {
              "defined": "EventQueueHeader"
            }
          },
          {
            "name": "buf",
            "type": {
              "array": [
                {
                  "defined": "AnyEvent"
                },
                488
              ]
            }
          }
        ]
      }
    },
    {
      "name": "perpMarket",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "group",
            "type": "publicKey"
          },
          {
            "name": "settleTokenIndex",
            "type": "u16"
          },
          {
            "name": "perpMarketIndex",
            "docs": [
              "Lookup indices"
            ],
            "type": "u16"
          },
          {
            "name": "trustedMarket",
            "docs": [
              "May this market contribute positive values to health?"
            ],
            "type": "u8"
          },
          {
            "name": "groupInsuranceFund",
            "docs": [
              "Is this market covered by the group insurance fund?"
            ],
            "type": "u8"
          },
          {
            "name": "padding1",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "name",
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          },
          {
            "name": "oracle",
            "type": "publicKey"
          },
          {
            "name": "oracleConfig",
            "type": {
              "defined": "OracleConfig"
            }
          },
          {
            "name": "orderbook",
            "type": "publicKey"
          },
          {
            "name": "eventQueue",
            "type": "publicKey"
          },
          {
            "name": "quoteLotSize",
            "docs": [
              "Number of quote native that reresents min tick"
            ],
            "type": "i64"
          },
          {
            "name": "baseLotSize",
            "docs": [
              "Represents number of base native quantity",
              "e.g. if base decimals for underlying asset are 6, base lot size is 100, and base position is 10000, then",
              "UI position is 1"
            ],
            "type": "i64"
          },
          {
            "name": "maintAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maintLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "liquidationFee",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "makerFee",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "takerFee",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "minFunding",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maxFunding",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "impactQuantity",
            "type": "i64"
          },
          {
            "name": "longFunding",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "shortFunding",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "fundingLastUpdated",
            "docs": [
              "timestamp that funding was last updated in"
            ],
            "type": "u64"
          },
          {
            "name": "openInterest",
            "docs": [
              ""
            ],
            "type": "i64"
          },
          {
            "name": "seqNum",
            "docs": [
              "Total number of orders seen"
            ],
            "type": "u64"
          },
          {
            "name": "feesAccrued",
            "docs": [
              "Fees accrued in native quote currency"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "bump",
            "docs": [
              "Liquidity mining metadata",
              "pub liquidity_mining_info: LiquidityMiningInfo,",
              "Token vault which holds mango tokens to be disbursed as liquidity incentives for this perp market",
              "pub mngo_vault: Pubkey,",
              "PDA bump"
            ],
            "type": "u8"
          },
          {
            "name": "baseDecimals",
            "type": "u8"
          },
          {
            "name": "padding2",
            "type": {
              "array": [
                "u8",
                6
              ]
            }
          },
          {
            "name": "registrationTime",
            "type": "u64"
          },
          {
            "name": "feesSettled",
            "docs": [
              "Fees settled in native quote currency"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "feePenalty",
            "type": "f32"
          },
          {
            "name": "settleFeeFlat",
            "docs": [
              "In native units of settlement token, given to each settle call above the",
              "settle_fee_amount_threshold."
            ],
            "type": "f32"
          },
          {
            "name": "settleFeeAmountThreshold",
            "docs": [
              "Pnl settlement amount needed to be eligible for fees."
            ],
            "type": "f32"
          },
          {
            "name": "settleFeeFractionLowHealth",
            "docs": [
              "Fraction of pnl to pay out as fee if +pnl account has low health."
            ],
            "type": "f32"
          },
          {
            "name": "stablePriceModel",
            "type": {
              "defined": "StablePriceModel"
            }
          },
          {
            "name": "settlePnlLimitFactor",
            "type": "f32"
          },
          {
            "name": "settlePnlLimitFactorWindowSizeTs",
            "type": "u64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                1944
              ]
            }
          }
        ]
      }
    },
    {
      "name": "serum3Market",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "group",
            "type": "publicKey"
          },
          {
            "name": "baseTokenIndex",
            "type": "u16"
          },
          {
            "name": "quoteTokenIndex",
            "type": "u16"
          },
          {
            "name": "padding1",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "name",
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          },
          {
            "name": "serumProgram",
            "type": "publicKey"
          },
          {
            "name": "serumMarketExternal",
            "type": "publicKey"
          },
          {
            "name": "marketIndex",
            "type": "u16"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "padding2",
            "type": {
              "array": [
                "u8",
                5
              ]
            }
          },
          {
            "name": "registrationTime",
            "type": "u64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                128
              ]
            }
          }
        ]
      }
    },
    {
      "name": "serum3MarketIndexReservation",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "group",
            "type": "publicKey"
          },
          {
            "name": "marketIndex",
            "type": "u16"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                38
              ]
            }
          }
        ]
      }
    }
  ],
  "types": [
    {
      "name": "Equity",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tokens",
            "type": {
              "vec": {
                "defined": "TokenEquity"
              }
            }
          },
          {
            "name": "perps",
            "type": {
              "vec": {
                "defined": "PerpEquity"
              }
            }
          }
        ]
      }
    },
    {
      "name": "TokenEquity",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tokenIndex",
            "type": "u16"
          },
          {
            "name": "value",
            "type": {
              "defined": "I80F48"
            }
          }
        ]
      }
    },
    {
      "name": "PerpEquity",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "perpMarketIndex",
            "type": "u16"
          },
          {
            "name": "value",
            "type": {
              "defined": "I80F48"
            }
          }
        ]
      }
    },
    {
      "name": "InterestRateParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "util0",
            "type": "f32"
          },
          {
            "name": "rate0",
            "type": "f32"
          },
          {
            "name": "util1",
            "type": "f32"
          },
          {
            "name": "rate1",
            "type": "f32"
          },
          {
            "name": "maxRate",
            "type": "f32"
          },
          {
            "name": "adjustmentFactor",
            "type": "f32"
          }
        ]
      }
    },
    {
      "name": "FlashLoanTokenDetail",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tokenIndex",
            "type": "u16"
          },
          {
            "name": "changeAmount",
            "type": "i128"
          },
          {
            "name": "loan",
            "type": "i128"
          },
          {
            "name": "loanOriginationFee",
            "type": "i128"
          },
          {
            "name": "depositIndex",
            "type": "i128"
          },
          {
            "name": "borrowIndex",
            "type": "i128"
          },
          {
            "name": "price",
            "type": "i128"
          }
        ]
      }
    },
    {
      "name": "Prices",
      "docs": [
        "Information about prices for a bank or perp market."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "oracle",
            "docs": [
              "The current oracle price"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "stable",
            "docs": [
              "A \"stable\" price, provided by StablePriceModel"
            ],
            "type": {
              "defined": "I80F48"
            }
          }
        ]
      }
    },
    {
      "name": "TokenInfo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tokenIndex",
            "type": "u16"
          },
          {
            "name": "maintAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maintLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "prices",
            "type": {
              "defined": "Prices"
            }
          },
          {
            "name": "balanceNative",
            "type": {
              "defined": "I80F48"
            }
          }
        ]
      }
    },
    {
      "name": "Serum3Info",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "reservedBase",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "reservedQuote",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "baseIndex",
            "type": "u64"
          },
          {
            "name": "quoteIndex",
            "type": "u64"
          },
          {
            "name": "marketIndex",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "PerpInfo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "perpMarketIndex",
            "type": "u16"
          },
          {
            "name": "maintAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maintLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "baseLotSize",
            "type": "i64"
          },
          {
            "name": "baseLots",
            "type": "i64"
          },
          {
            "name": "bidsBaseLots",
            "type": "i64"
          },
          {
            "name": "asksBaseLots",
            "type": "i64"
          },
          {
            "name": "quote",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "prices",
            "type": {
              "defined": "Prices"
            }
          },
          {
            "name": "hasOpenOrders",
            "type": "bool"
          },
          {
            "name": "trustedMarket",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "HealthCache",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tokenInfos",
            "type": {
              "vec": {
                "defined": "TokenInfo"
              }
            }
          },
          {
            "name": "serum3Infos",
            "type": {
              "vec": {
                "defined": "Serum3Info"
              }
            }
          },
          {
            "name": "perpInfos",
            "type": {
              "vec": {
                "defined": "PerpInfo"
              }
            }
          },
          {
            "name": "beingLiquidated",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "TokenPosition",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "indexedPosition",
            "docs": [
              "The deposit_index (if positive) or borrow_index (if negative) scaled position"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "tokenIndex",
            "docs": [
              "index into Group.tokens"
            ],
            "type": "u16"
          },
          {
            "name": "inUseCount",
            "docs": [
              "incremented when a market requires this position to stay alive"
            ],
            "type": "u8"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                5
              ]
            }
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                8
              ]
            }
          },
          {
            "name": "previousIndex",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "cumulativeDepositInterest",
            "type": "f64"
          },
          {
            "name": "cumulativeBorrowInterest",
            "type": "f64"
          }
        ]
      }
    },
    {
      "name": "Serum3Orders",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "openOrders",
            "type": "publicKey"
          },
          {
            "name": "baseBorrowsWithoutFee",
            "docs": [
              "Tracks the amount of borrows that have flowed into the serum open orders account.",
              "These borrows did not have the loan origination fee applied, and that may happen",
              "later (in serum3_settle_funds) if we can guarantee that the funds were used.",
              "In particular a place-on-book, cancel, settle should not cost fees."
            ],
            "type": "u64"
          },
          {
            "name": "quoteBorrowsWithoutFee",
            "type": "u64"
          },
          {
            "name": "marketIndex",
            "type": "u16"
          },
          {
            "name": "baseTokenIndex",
            "docs": [
              "Store the base/quote token index, so health computations don't need",
              "to get passed the static SerumMarket to find which tokens a market",
              "uses and look up the correct oracles."
            ],
            "type": "u16"
          },
          {
            "name": "quoteTokenIndex",
            "type": "u16"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          }
        ]
      }
    },
    {
      "name": "PerpPosition",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "marketIndex",
            "type": "u16"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "settlePnlLimitWindow",
            "type": "u32"
          },
          {
            "name": "basePositionLots",
            "docs": [
              "Active position size, measured in base lots"
            ],
            "type": "i64"
          },
          {
            "name": "quotePositionNative",
            "docs": [
              "Active position in quote (conversation rate is that of the time the order was settled)",
              "measured in native quote"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "padding2",
            "docs": [
              "Tracks what the position is to calculate average entry & break even price"
            ],
            "type": {
              "array": [
                "u8",
                8
              ]
            }
          },
          {
            "name": "quoteRunningNative",
            "type": "i64"
          },
          {
            "name": "longSettledFunding",
            "docs": [
              "Already settled funding"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "shortSettledFunding",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "bidsBaseLots",
            "docs": [
              "Base lots in bids"
            ],
            "type": "i64"
          },
          {
            "name": "asksBaseLots",
            "docs": [
              "Base lots in asks"
            ],
            "type": "i64"
          },
          {
            "name": "takerBaseLots",
            "docs": [
              "Liquidity mining rewards",
              "Amount that's on EventQueue waiting to be processed"
            ],
            "type": "i64"
          },
          {
            "name": "takerQuoteLots",
            "type": "i64"
          },
          {
            "name": "cumulativeLongFunding",
            "type": "f64"
          },
          {
            "name": "cumulativeShortFunding",
            "type": "f64"
          },
          {
            "name": "makerVolume",
            "type": "u64"
          },
          {
            "name": "takerVolume",
            "type": "u64"
          },
          {
            "name": "perpSpotTransfers",
            "type": "i64"
          },
          {
            "name": "avgEntryPricePerBaseLot",
            "type": "f64"
          },
          {
            "name": "realizedPnlNative",
            "type": "i64"
          },
          {
            "name": "settlePnlLimitSettledInCurrentWindowNative",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "PerpOpenOrder",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sideAndTree",
            "type": {
              "defined": "SideAndOrderTree"
            }
          },
          {
            "name": "padding1",
            "type": {
              "array": [
                "u8",
                1
              ]
            }
          },
          {
            "name": "market",
            "type": "u16"
          },
          {
            "name": "padding2",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "clientId",
            "type": "u64"
          },
          {
            "name": "id",
            "type": "u128"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          }
        ]
      }
    },
    {
      "name": "OracleConfig",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "confFilter",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maxStalenessSlots",
            "type": "i64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                72
              ]
            }
          }
        ]
      }
    },
    {
      "name": "OracleConfigParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "confFilter",
            "type": "f32"
          },
          {
            "name": "maxStalenessSlots",
            "type": {
              "option": "u32"
            }
          }
        ]
      }
    },
    {
      "name": "BookSide",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "fixed",
            "type": {
              "defined": "OrderTree"
            }
          },
          {
            "name": "oraclePegged",
            "type": {
              "defined": "OrderTree"
            }
          }
        ]
      }
    },
    {
      "name": "InnerNode",
      "docs": [
        "InnerNodes and LeafNodes compose the binary tree of orders.",
        "",
        "Each InnerNode has exactly two children, which are either InnerNodes themselves,",
        "or LeafNodes. The children share the top `prefix_len` bits of `key`. The left",
        "child has a 0 in the next bit, and the right a 1."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tag",
            "type": "u32"
          },
          {
            "name": "prefixLen",
            "docs": [
              "number of highest `key` bits that all children share",
              "e.g. if it's 2, the two highest bits of `key` will be the same on all children"
            ],
            "type": "u32"
          },
          {
            "name": "key",
            "docs": [
              "only the top `prefix_len` bits of `key` are relevant"
            ],
            "type": "u128"
          },
          {
            "name": "children",
            "docs": [
              "indexes into `BookSide::nodes`"
            ],
            "type": {
              "array": [
                "u32",
                2
              ]
            }
          },
          {
            "name": "childEarliestExpiry",
            "docs": [
              "The earliest expiry timestamp for the left and right subtrees.",
              "",
              "Needed to be able to find and remove expired orders without having to",
              "iterate through the whole bookside."
            ],
            "type": {
              "array": [
                "u64",
                2
              ]
            }
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                48
              ]
            }
          }
        ]
      }
    },
    {
      "name": "LeafNode",
      "docs": [
        "LeafNodes represent an order in the binary tree"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tag",
            "type": "u32"
          },
          {
            "name": "ownerSlot",
            "type": "u8"
          },
          {
            "name": "orderType",
            "type": {
              "defined": "PostOrderType"
            }
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                1
              ]
            }
          },
          {
            "name": "timeInForce",
            "docs": [
              "Time in seconds after `timestamp` at which the order expires.",
              "A value of 0 means no expiry."
            ],
            "type": "u8"
          },
          {
            "name": "key",
            "docs": [
              "The binary tree key"
            ],
            "type": "u128"
          },
          {
            "name": "owner",
            "type": "publicKey"
          },
          {
            "name": "quantity",
            "type": "i64"
          },
          {
            "name": "clientOrderId",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "u64"
          },
          {
            "name": "pegLimit",
            "type": "i64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                8
              ]
            }
          }
        ]
      }
    },
    {
      "name": "AnyNode",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tag",
            "type": "u32"
          },
          {
            "name": "data",
            "type": {
              "array": [
                "u8",
                92
              ]
            }
          }
        ]
      }
    },
    {
      "name": "OrderTree",
      "docs": [
        "A binary tree on AnyNode::key()",
        "",
        "The key encodes the price in the top 64 bits."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "orderTreeType",
            "type": {
              "defined": "OrderTreeType"
            }
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                3
              ]
            }
          },
          {
            "name": "bumpIndex",
            "type": "u32"
          },
          {
            "name": "freeListLen",
            "type": "u32"
          },
          {
            "name": "freeListHead",
            "type": "u32"
          },
          {
            "name": "rootNode",
            "type": "u32"
          },
          {
            "name": "leafCount",
            "type": "u32"
          },
          {
            "name": "nodes",
            "type": {
              "array": [
                {
                  "defined": "AnyNode"
                },
                1024
              ]
            }
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                256
              ]
            }
          }
        ]
      }
    },
    {
      "name": "EventQueueHeader",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "head",
            "type": "u32"
          },
          {
            "name": "count",
            "type": "u32"
          },
          {
            "name": "seqNum",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "AnyEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "eventType",
            "type": "u8"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                207
              ]
            }
          }
        ]
      }
    },
    {
      "name": "FillEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "eventType",
            "type": "u8"
          },
          {
            "name": "takerSide",
            "type": {
              "defined": "Side"
            }
          },
          {
            "name": "makerOut",
            "type": "bool"
          },
          {
            "name": "makerSlot",
            "type": "u8"
          },
          {
            "name": "marketFeesApplied",
            "type": "bool"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                3
              ]
            }
          },
          {
            "name": "timestamp",
            "type": "u64"
          },
          {
            "name": "seqNum",
            "type": "u64"
          },
          {
            "name": "maker",
            "type": "publicKey"
          },
          {
            "name": "makerOrderId",
            "type": "u128"
          },
          {
            "name": "makerClientOrderId",
            "type": "u64"
          },
          {
            "name": "makerFee",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "makerTimestamp",
            "type": "u64"
          },
          {
            "name": "taker",
            "type": "publicKey"
          },
          {
            "name": "takerOrderId",
            "type": "u128"
          },
          {
            "name": "takerClientOrderId",
            "type": "u64"
          },
          {
            "name": "takerFee",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "price",
            "type": "i64"
          },
          {
            "name": "quantity",
            "type": "i64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          }
        ]
      }
    },
    {
      "name": "OutEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "eventType",
            "type": "u8"
          },
          {
            "name": "side",
            "type": {
              "defined": "Side"
            }
          },
          {
            "name": "ownerSlot",
            "type": "u8"
          },
          {
            "name": "padding0",
            "type": {
              "array": [
                "u8",
                5
              ]
            }
          },
          {
            "name": "timestamp",
            "type": "u64"
          },
          {
            "name": "seqNum",
            "type": "u64"
          },
          {
            "name": "owner",
            "type": "publicKey"
          },
          {
            "name": "quantity",
            "type": "i64"
          },
          {
            "name": "padding1",
            "type": {
              "array": [
                "u8",
                144
              ]
            }
          }
        ]
      }
    },
    {
      "name": "StablePriceModel",
      "docs": [
        "Maintains a \"stable_price\" based on the oracle price.",
        "",
        "The stable price follows the oracle price, but its relative rate of",
        "change is limited (to `stable_growth_limit`) and futher reduced if",
        "the oracle price is far from the `delay_price`.",
        "",
        "Conceptually the `delay_price` is itself a time delayed",
        "(`24 * delay_interval_seconds`, assume 24h) and relative rate of change limited",
        "function of the oracle price. It is implemented as averaging the oracle",
        "price over every `delay_interval_seconds` (assume 1h) and then applying the",
        "`delay_growth_limit` between intervals."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "stablePrice",
            "docs": [
              "Current stable price to use in health"
            ],
            "type": "f64"
          },
          {
            "name": "lastUpdateTimestamp",
            "type": "u64"
          },
          {
            "name": "delayPrices",
            "docs": [
              "Stored delay_price for each delay_interval.",
              "If we want the delay_price to be 24h delayed, we would store one for each hour.",
              "This is used in a cyclical way: We use the maximally-delayed value at delay_interval_index",
              "and once enough time passes to move to the next delay interval, that gets overwritten and",
              "we use the next one."
            ],
            "type": {
              "array": [
                "f64",
                24
              ]
            }
          },
          {
            "name": "delayAccumulatorPrice",
            "docs": [
              "The delay price is based on an average over each delay_interval. The contributions",
              "to the average are summed up here."
            ],
            "type": "f64"
          },
          {
            "name": "delayAccumulatorTime",
            "docs": [
              "Accumulating the total time for the above average."
            ],
            "type": "u32"
          },
          {
            "name": "delayIntervalSeconds",
            "docs": [
              "Length of a delay_interval"
            ],
            "type": "u32"
          },
          {
            "name": "delayGrowthLimit",
            "docs": [
              "Maximal relative difference between two delay_price in consecutive intervals."
            ],
            "type": "f32"
          },
          {
            "name": "stableGrowthLimit",
            "docs": [
              "Maximal per-second relative difference of the stable price.",
              "It gets further reduced if stable and delay price disagree."
            ],
            "type": "f32"
          },
          {
            "name": "lastDelayIntervalIndex",
            "docs": [
              "The delay_interval_index that update() was last called on."
            ],
            "type": "u8"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                7
              ]
            }
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                48
              ]
            }
          }
        ]
      }
    },
    {
      "name": "TokenIndex",
      "docs": [
        "Nothing in Rust shall use these types. They only exist so that the Anchor IDL",
        "knows about them and typescript can deserialize it."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "val",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "Serum3MarketIndex",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "val",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "PerpMarketIndex",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "val",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "I80F48",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "val",
            "type": "i128"
          }
        ]
      }
    },
    {
      "name": "FlashLoanType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Unknown"
          },
          {
            "name": "Swap"
          }
        ]
      }
    },
    {
      "name": "Serum3SelfTradeBehavior",
      "docs": [
        "Copy paste a bunch of enums so that we could AnchorSerialize & AnchorDeserialize them"
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "DecrementTake"
          },
          {
            "name": "CancelProvide"
          },
          {
            "name": "AbortTransaction"
          }
        ]
      }
    },
    {
      "name": "Serum3OrderType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Limit"
          },
          {
            "name": "ImmediateOrCancel"
          },
          {
            "name": "PostOnly"
          }
        ]
      }
    },
    {
      "name": "Serum3Side",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Bid"
          },
          {
            "name": "Ask"
          }
        ]
      }
    },
    {
      "name": "LoanOriginationFeeInstruction",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Unknown"
          },
          {
            "name": "LiqTokenBankruptcy"
          },
          {
            "name": "LiqTokenWithToken"
          },
          {
            "name": "Serum3LiqForceCancelOrders"
          },
          {
            "name": "Serum3PlaceOrder"
          },
          {
            "name": "Serum3SettleFunds"
          },
          {
            "name": "TokenWithdraw"
          }
        ]
      }
    },
    {
      "name": "HealthType",
      "docs": [
        "There are two types of health, initial health used for opening new positions and maintenance",
        "health used for liquidations. They are both calculated as a weighted sum of the assets",
        "minus the liabilities but the maint. health uses slightly larger weights for assets and",
        "slightly smaller weights for the liabilities. Zero is used as the bright line for both",
        "i.e. if your init health falls below zero, you cannot open new positions and if your maint. health",
        "falls below zero you will be liquidated."
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Init"
          },
          {
            "name": "Maint"
          }
        ]
      }
    },
    {
      "name": "OracleType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Pyth"
          },
          {
            "name": "Stub"
          },
          {
            "name": "SwitchboardV1"
          },
          {
            "name": "SwitchboardV2"
          }
        ]
      }
    },
    {
      "name": "OrderState",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Valid"
          },
          {
            "name": "Invalid"
          },
          {
            "name": "Skipped"
          }
        ]
      }
    },
    {
      "name": "BookSideOrderTree",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Fixed"
          },
          {
            "name": "OraclePegged"
          }
        ]
      }
    },
    {
      "name": "NodeTag",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Uninitialized"
          },
          {
            "name": "InnerNode"
          },
          {
            "name": "LeafNode"
          },
          {
            "name": "FreeNode"
          },
          {
            "name": "LastFreeNode"
          }
        ]
      }
    },
    {
      "name": "PlaceOrderType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Limit"
          },
          {
            "name": "ImmediateOrCancel"
          },
          {
            "name": "PostOnly"
          },
          {
            "name": "Market"
          },
          {
            "name": "PostOnlySlide"
          }
        ]
      }
    },
    {
      "name": "PostOrderType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Limit"
          },
          {
            "name": "PostOnly"
          },
          {
            "name": "PostOnlySlide"
          }
        ]
      }
    },
    {
      "name": "Side",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Bid"
          },
          {
            "name": "Ask"
          }
        ]
      }
    },
    {
      "name": "SideAndOrderTree",
      "docs": [
        "SideAndOrderTree is a storage optimization, so we don't need two bytes for the data"
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "BidFixed"
          },
          {
            "name": "AskFixed"
          },
          {
            "name": "BidOraclePegged"
          },
          {
            "name": "AskOraclePegged"
          }
        ]
      }
    },
    {
      "name": "OrderParams",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Market"
          },
          {
            "name": "ImmediateOrCancel",
            "fields": [
              {
                "name": "price_lots",
                "type": "i64"
              }
            ]
          },
          {
            "name": "Fixed",
            "fields": [
              {
                "name": "price_lots",
                "type": "i64"
              },
              {
                "name": "order_type",
                "type": {
                  "defined": "PostOrderType"
                }
              }
            ]
          },
          {
            "name": "OraclePegged",
            "fields": [
              {
                "name": "price_offset_lots",
                "type": "i64"
              },
              {
                "name": "order_type",
                "type": {
                  "defined": "PostOrderType"
                }
              },
              {
                "name": "peg_limit",
                "type": "i64"
              }
            ]
          }
        ]
      }
    },
    {
      "name": "OrderTreeType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Bids"
          },
          {
            "name": "Asks"
          }
        ]
      }
    },
    {
      "name": "EventType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Fill"
          },
          {
            "name": "Out"
          },
          {
            "name": "Liquidate"
          }
        ]
      }
    }
  ],
  "events": [
    {
      "name": "MangoAccountData",
      "fields": [
        {
          "name": "healthCache",
          "type": {
            "defined": "HealthCache"
          },
          "index": false
        },
        {
          "name": "initHealth",
          "type": {
            "defined": "I80F48"
          },
          "index": false
        },
        {
          "name": "maintHealth",
          "type": {
            "defined": "I80F48"
          },
          "index": false
        },
        {
          "name": "equity",
          "type": {
            "defined": "Equity"
          },
          "index": false
        }
      ]
    },
    {
      "name": "PerpBalanceLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "marketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "basePosition",
          "type": "i64",
          "index": false
        },
        {
          "name": "quotePosition",
          "type": "i128",
          "index": false
        },
        {
          "name": "longSettledFunding",
          "type": "i128",
          "index": false
        },
        {
          "name": "shortSettledFunding",
          "type": "i128",
          "index": false
        },
        {
          "name": "longFunding",
          "type": "i128",
          "index": false
        },
        {
          "name": "shortFunding",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "TokenBalanceLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "indexedPosition",
          "type": "i128",
          "index": false
        },
        {
          "name": "depositIndex",
          "type": "i128",
          "index": false
        },
        {
          "name": "borrowIndex",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "FlashLoanLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenLoanDetails",
          "type": {
            "vec": {
              "defined": "FlashLoanTokenDetail"
            }
          },
          "index": false
        },
        {
          "name": "flashLoanType",
          "type": {
            "defined": "FlashLoanType"
          },
          "index": false
        }
      ]
    },
    {
      "name": "WithdrawLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "signer",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "quantity",
          "type": "u64",
          "index": false
        },
        {
          "name": "price",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "DepositLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "signer",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "quantity",
          "type": "u64",
          "index": false
        },
        {
          "name": "price",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "FillLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "marketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "takerSide",
          "type": "u8",
          "index": false
        },
        {
          "name": "makerSlot",
          "type": "u8",
          "index": false
        },
        {
          "name": "marketFeesApplied",
          "type": "bool",
          "index": false
        },
        {
          "name": "makerOut",
          "type": "bool",
          "index": false
        },
        {
          "name": "timestamp",
          "type": "u64",
          "index": false
        },
        {
          "name": "seqNum",
          "type": "u64",
          "index": false
        },
        {
          "name": "maker",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "makerOrderId",
          "type": "u128",
          "index": false
        },
        {
          "name": "makerClientOrderId",
          "type": "u64",
          "index": false
        },
        {
          "name": "makerFee",
          "type": "i128",
          "index": false
        },
        {
          "name": "makerTimestamp",
          "type": "u64",
          "index": false
        },
        {
          "name": "taker",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "takerOrderId",
          "type": "u128",
          "index": false
        },
        {
          "name": "takerClientOrderId",
          "type": "u64",
          "index": false
        },
        {
          "name": "takerFee",
          "type": "i128",
          "index": false
        },
        {
          "name": "price",
          "type": "i64",
          "index": false
        },
        {
          "name": "quantity",
          "type": "i64",
          "index": false
        }
      ]
    },
    {
      "name": "PerpUpdateFundingLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "marketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "longFunding",
          "type": "i128",
          "index": false
        },
        {
          "name": "shortFunding",
          "type": "i128",
          "index": false
        },
        {
          "name": "price",
          "type": "i128",
          "index": false
        },
        {
          "name": "stablePrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "feesAccrued",
          "type": "i128",
          "index": false
        },
        {
          "name": "openInterest",
          "type": "i64",
          "index": false
        }
      ]
    },
    {
      "name": "UpdateIndexLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "depositIndex",
          "type": "i128",
          "index": false
        },
        {
          "name": "borrowIndex",
          "type": "i128",
          "index": false
        },
        {
          "name": "avgUtilization",
          "type": "i128",
          "index": false
        },
        {
          "name": "price",
          "type": "i128",
          "index": false
        },
        {
          "name": "stablePrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "collectedFees",
          "type": "i128",
          "index": false
        },
        {
          "name": "loanFeeRate",
          "type": "i128",
          "index": false
        },
        {
          "name": "totalBorrows",
          "type": "i128",
          "index": false
        },
        {
          "name": "totalDeposits",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "UpdateRateLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "rate0",
          "type": "i128",
          "index": false
        },
        {
          "name": "rate1",
          "type": "i128",
          "index": false
        },
        {
          "name": "maxRate",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "TokenLiqWithTokenLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "liqee",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "liqor",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "assetTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "liabTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "assetTransfer",
          "type": "i128",
          "index": false
        },
        {
          "name": "liabTransfer",
          "type": "i128",
          "index": false
        },
        {
          "name": "assetPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "liabPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "bankruptcy",
          "type": "bool",
          "index": false
        }
      ]
    },
    {
      "name": "Serum3OpenOrdersBalanceLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "baseTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "quoteTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "baseTotal",
          "type": "u64",
          "index": false
        },
        {
          "name": "baseFree",
          "type": "u64",
          "index": false
        },
        {
          "name": "quoteTotal",
          "type": "u64",
          "index": false
        },
        {
          "name": "quoteFree",
          "type": "u64",
          "index": false
        },
        {
          "name": "referrerRebatesAccrued",
          "type": "u64",
          "index": false
        }
      ]
    },
    {
      "name": "WithdrawLoanOriginationFeeLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "loanOriginationFee",
          "type": "i128",
          "index": false
        },
        {
          "name": "instruction",
          "type": {
            "defined": "LoanOriginationFeeInstruction"
          },
          "index": false
        }
      ]
    },
    {
      "name": "TokenLiqBankruptcyLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "liqee",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "liqor",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "liabTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "initialLiabNative",
          "type": "i128",
          "index": false
        },
        {
          "name": "liabPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "insuranceTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "insuranceTransfer",
          "type": "i128",
          "index": false
        },
        {
          "name": "socializedLoss",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "DeactivateTokenPositionLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "cumulativeDepositInterest",
          "type": "f64",
          "index": false
        },
        {
          "name": "cumulativeBorrowInterest",
          "type": "f64",
          "index": false
        }
      ]
    },
    {
      "name": "DeactivatePerpPositionLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "marketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "cumulativeLongFunding",
          "type": "f64",
          "index": false
        },
        {
          "name": "cumulativeShortFunding",
          "type": "f64",
          "index": false
        },
        {
          "name": "makerVolume",
          "type": "u64",
          "index": false
        },
        {
          "name": "takerVolume",
          "type": "u64",
          "index": false
        },
        {
          "name": "perpSpotTransfers",
          "type": "i64",
          "index": false
        }
      ]
    },
    {
      "name": "TokenMetaDataLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mint",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "mintDecimals",
          "type": "u8",
          "index": false
        },
        {
          "name": "oracle",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mintInfo",
          "type": "publicKey",
          "index": false
        }
      ]
    },
    {
      "name": "PerpMarketMetaDataLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "perpMarket",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "perpMarketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "baseDecimals",
          "type": "u8",
          "index": false
        },
        {
          "name": "baseLotSize",
          "type": "i64",
          "index": false
        },
        {
          "name": "quoteLotSize",
          "type": "i64",
          "index": false
        },
        {
          "name": "oracle",
          "type": "publicKey",
          "index": false
        }
      ]
    },
    {
      "name": "Serum3RegisterMarketLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "serumMarket",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "marketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "baseTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "quoteTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "serumProgram",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "serumProgramExternal",
          "type": "publicKey",
          "index": false
        }
      ]
    },
    {
      "name": "PerpLiqBasePositionLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "perpMarketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "liqor",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "liqee",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "baseTransfer",
          "type": "i64",
          "index": false
        },
        {
          "name": "quoteTransfer",
          "type": "i128",
          "index": false
        },
        {
          "name": "price",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "PerpLiqBankruptcyLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "liqee",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "liqor",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "perpMarketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "insuranceTransfer",
          "type": "i128",
          "index": false
        },
        {
          "name": "socializedLoss",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "PerpSettlePnlLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccountA",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccountB",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "perpMarketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "settlement",
          "type": "i128",
          "index": false
        },
        {
          "name": "settler",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "fee",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "PerpSettleFeesLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "perpMarketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "settlement",
          "type": "i128",
          "index": false
        }
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "SomeError",
      "msg": ""
    },
    {
      "code": 6001,
      "name": "NotImplementedError",
      "msg": ""
    },
    {
      "code": 6002,
      "name": "MathError",
      "msg": "checked math error"
    },
    {
      "code": 6003,
      "name": "UnexpectedOracle",
      "msg": ""
    },
    {
      "code": 6004,
      "name": "UnknownOracleType",
      "msg": "oracle type cannot be determined"
    },
    {
      "code": 6005,
      "name": "InvalidFlashLoanTargetCpiProgram",
      "msg": ""
    },
    {
      "code": 6006,
      "name": "HealthMustBePositive",
      "msg": "health must be positive"
    },
    {
      "code": 6007,
      "name": "HealthMustBePositiveOrIncrease",
      "msg": "health must be positive or increase"
    },
    {
      "code": 6008,
      "name": "HealthMustBeNegative",
      "msg": "health must be negative"
    },
    {
      "code": 6009,
      "name": "IsBankrupt",
      "msg": "the account is bankrupt"
    },
    {
      "code": 6010,
      "name": "IsNotBankrupt",
      "msg": "the account is not bankrupt"
    },
    {
      "code": 6011,
      "name": "NoFreeTokenPositionIndex",
      "msg": "no free token position index"
    },
    {
      "code": 6012,
      "name": "NoFreeSerum3OpenOrdersIndex",
      "msg": "no free serum3 open orders index"
    },
    {
      "code": 6013,
      "name": "NoFreePerpPositionIndex",
      "msg": "no free perp position index"
    },
    {
      "code": 6014,
      "name": "Serum3OpenOrdersExistAlready",
      "msg": "serum3 open orders exist already"
    },
    {
      "code": 6015,
      "name": "InsufficentBankVaultFunds",
      "msg": "bank vault has insufficent funds"
    },
    {
      "code": 6016,
      "name": "BeingLiquidated",
      "msg": "account is currently being liquidated"
    },
    {
      "code": 6017,
      "name": "InvalidBank",
      "msg": "invalid bank"
    },
    {
      "code": 6018,
      "name": "ProfitabilityMismatch",
      "msg": "account profitability is mismatched"
    },
    {
      "code": 6019,
      "name": "CannotSettleWithSelf",
      "msg": "cannot settle with self"
    },
    {
      "code": 6020,
      "name": "PerpPositionDoesNotExist",
      "msg": "perp position does not exist"
    },
    {
      "code": 6021,
      "name": "MaxSettleAmountMustBeGreaterThanZero",
      "msg": "max settle amount must be greater than zero"
    },
    {
      "code": 6022,
      "name": "HasOpenPerpOrders",
      "msg": "the perp position has open orders or unprocessed fill events"
    },
    {
      "code": 6023,
      "name": "OracleConfidence",
      "msg": "an oracle does not reach the confidence threshold"
    },
    {
      "code": 6024,
      "name": "OracleStale",
      "msg": "an oracle is stale"
    },
    {
      "code": 6025,
      "name": "SettlementAmountMustBePositive",
      "msg": "settlement amount must always be positive"
    },
    {
      "code": 6026,
      "name": "BankBorrowLimitReached",
      "msg": "bank utilization has reached limit"
    },
    {
      "code": 6027,
      "name": "BankNetBorrowsLimitReached",
      "msg": "bank net borrows has reached limit - this is an intermittent error - the limit will reset regularly"
    }
  ]
};

export const IDL: MangoV4 = {
  "version": "0.1.0",
  "name": "mango_v4",
  "instructions": [
    {
      "name": "groupCreate",
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Group"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "creator"
              },
              {
                "kind": "arg",
                "type": "u32",
                "path": "group_num"
              }
            ]
          }
        },
        {
          "name": "creator",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "insuranceMint",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "insuranceVault",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "InsuranceVault"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              }
            ]
          }
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "groupNum",
          "type": "u32"
        },
        {
          "name": "testing",
          "type": "u8"
        },
        {
          "name": "version",
          "type": "u8"
        }
      ]
    },
    {
      "name": "groupEdit",
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "adminOpt",
          "type": {
            "option": "publicKey"
          }
        },
        {
          "name": "fastListingAdminOpt",
          "type": {
            "option": "publicKey"
          }
        },
        {
          "name": "testingOpt",
          "type": {
            "option": "u8"
          }
        },
        {
          "name": "versionOpt",
          "type": {
            "option": "u8"
          }
        }
      ]
    },
    {
      "name": "groupClose",
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "insuranceVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "solDestination",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "tokenRegister",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "mint",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Bank"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "const",
                "type": "u32",
                "value": 0
              }
            ]
          }
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Vault"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "const",
                "type": "u32",
                "value": 0
              }
            ]
          }
        },
        {
          "name": "mintInfo",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "MintInfo"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "account": "Mint",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "tokenIndex",
          "type": "u16"
        },
        {
          "name": "name",
          "type": "string"
        },
        {
          "name": "oracleConfig",
          "type": {
            "defined": "OracleConfigParams"
          }
        },
        {
          "name": "interestRateParams",
          "type": {
            "defined": "InterestRateParams"
          }
        },
        {
          "name": "loanFeeRate",
          "type": "f32"
        },
        {
          "name": "loanOriginationFeeRate",
          "type": "f32"
        },
        {
          "name": "maintAssetWeight",
          "type": "f32"
        },
        {
          "name": "initAssetWeight",
          "type": "f32"
        },
        {
          "name": "maintLiabWeight",
          "type": "f32"
        },
        {
          "name": "initLiabWeight",
          "type": "f32"
        },
        {
          "name": "liquidationFee",
          "type": "f32"
        },
        {
          "name": "minVaultToDepositsRatio",
          "type": "f64"
        },
        {
          "name": "netBorrowsWindowSizeTs",
          "type": "u64"
        },
        {
          "name": "netBorrowsLimitNative",
          "type": "i64"
        }
      ]
    },
    {
      "name": "tokenRegisterTrustless",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "fastListingAdmin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "mint",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Bank"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "const",
                "type": "u32",
                "value": 0
              }
            ]
          }
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Vault"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "const",
                "type": "u32",
                "value": 0
              }
            ]
          }
        },
        {
          "name": "mintInfo",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "MintInfo"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "account": "Mint",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "tokenIndex",
          "type": "u16"
        },
        {
          "name": "name",
          "type": "string"
        }
      ]
    },
    {
      "name": "tokenEdit",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "mintInfo",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "oracleOpt",
          "type": {
            "option": "publicKey"
          }
        },
        {
          "name": "oracleConfigOpt",
          "type": {
            "option": {
              "defined": "OracleConfigParams"
            }
          }
        },
        {
          "name": "groupInsuranceFundOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "interestRateParamsOpt",
          "type": {
            "option": {
              "defined": "InterestRateParams"
            }
          }
        },
        {
          "name": "loanFeeRateOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "loanOriginationFeeRateOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "maintAssetWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "initAssetWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "maintLiabWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "initLiabWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "liquidationFeeOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "stablePriceDelayIntervalSecondsOpt",
          "type": {
            "option": "u32"
          }
        },
        {
          "name": "stablePriceDelayGrowthLimitOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "stablePriceGrowthLimitOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "minVaultToDepositsRatioOpt",
          "type": {
            "option": "f64"
          }
        },
        {
          "name": "netBorrowsLimitNativeOpt",
          "type": {
            "option": "i64"
          }
        },
        {
          "name": "netBorrowsWindowSizeTsOpt",
          "type": {
            "option": "u64"
          }
        }
      ]
    },
    {
      "name": "tokenAddBank",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "mint",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "existingBank",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Bank"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "arg",
                "type": "u32",
                "path": "bank_num"
              }
            ]
          }
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Vault"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "arg",
                "type": "u32",
                "path": "bank_num"
              }
            ]
          }
        },
        {
          "name": "mintInfo",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "tokenIndex",
          "type": "u16"
        },
        {
          "name": "bankNum",
          "type": "u32"
        }
      ]
    },
    {
      "name": "tokenDeregister",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "mintInfo",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "dustVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "solDestination",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "tokenUpdateIndexAndRate",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "mintInfo",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "instructions",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "accountCreate",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "MangoAccount"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "owner"
              },
              {
                "kind": "arg",
                "type": "u32",
                "path": "account_num"
              }
            ]
          }
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "accountNum",
          "type": "u32"
        },
        {
          "name": "tokenCount",
          "type": "u8"
        },
        {
          "name": "serum3Count",
          "type": "u8"
        },
        {
          "name": "perpCount",
          "type": "u8"
        },
        {
          "name": "perpOoCount",
          "type": "u8"
        },
        {
          "name": "name",
          "type": "string"
        }
      ]
    },
    {
      "name": "accountExpand",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "tokenCount",
          "type": "u8"
        },
        {
          "name": "serum3Count",
          "type": "u8"
        },
        {
          "name": "perpCount",
          "type": "u8"
        },
        {
          "name": "perpOoCount",
          "type": "u8"
        }
      ]
    },
    {
      "name": "accountEdit",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "nameOpt",
          "type": {
            "option": "string"
          }
        },
        {
          "name": "delegateOpt",
          "type": {
            "option": "publicKey"
          }
        }
      ]
    },
    {
      "name": "accountClose",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "solDestination",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "stubOracleCreate",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "StubOracle"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "account": "Mint",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "mint",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "price",
          "type": {
            "defined": "I80F48"
          }
        }
      ]
    },
    {
      "name": "stubOracleClose",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "oracle",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "solDestination",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "stubOracleSet",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "oracle",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "price",
          "type": {
            "defined": "I80F48"
          }
        }
      ]
    },
    {
      "name": "tokenDeposit",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "tokenAccount",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "tokenDepositIntoExisting",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "tokenAccount",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "tokenWithdraw",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "tokenAccount",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        },
        {
          "name": "allowBorrow",
          "type": "bool"
        }
      ]
    },
    {
      "name": "flashLoanBegin",
      "accounts": [
        {
          "name": "account",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "instructions",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Instructions Sysvar for instruction introspection"
          ]
        }
      ],
      "args": [
        {
          "name": "loanAmounts",
          "type": {
            "vec": "u64"
          }
        }
      ]
    },
    {
      "name": "flashLoanEnd",
      "accounts": [
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "flashLoanType",
          "type": {
            "defined": "FlashLoanType"
          }
        }
      ]
    },
    {
      "name": "healthRegionBegin",
      "accounts": [
        {
          "name": "instructions",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Instructions Sysvar for instruction introspection"
          ]
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "healthRegionEnd",
      "accounts": [
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "serum3RegisterMarket",
      "docs": [
        "",
        "Serum",
        ""
      ],
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "serumProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumMarketExternal",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumMarket",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Serum3Market"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "serum_market_external"
              }
            ]
          }
        },
        {
          "name": "indexReservation",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Serum3Index"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "market_index"
              }
            ]
          }
        },
        {
          "name": "quoteBank",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "baseBank",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "marketIndex",
          "type": "u16"
        },
        {
          "name": "name",
          "type": "string"
        }
      ]
    },
    {
      "name": "serum3DeregisterMarket",
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "serumMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "indexReservation",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "solDestination",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "serum3CreateOpenOrders",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumMarketExternal",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "Serum3OO"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "account"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "serum_market"
              }
            ]
          }
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "serum3CloseOpenOrders",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumMarketExternal",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "solDestination",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "serum3PlaceOrder",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumMarketExternal",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketBids",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketAsks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketEventQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketRequestQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketBaseVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketQuoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketVaultSigner",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "needed for the automatic settle_funds call"
          ]
        },
        {
          "name": "payerBank",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The bank that pays for the order, if necessary"
          ]
        },
        {
          "name": "payerVault",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The bank vault that pays for the order, if necessary"
          ]
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "side",
          "type": {
            "defined": "Serum3Side"
          }
        },
        {
          "name": "limitPrice",
          "type": "u64"
        },
        {
          "name": "maxBaseQty",
          "type": "u64"
        },
        {
          "name": "maxNativeQuoteQtyIncludingFees",
          "type": "u64"
        },
        {
          "name": "selfTradeBehavior",
          "type": {
            "defined": "Serum3SelfTradeBehavior"
          }
        },
        {
          "name": "orderType",
          "type": {
            "defined": "Serum3OrderType"
          }
        },
        {
          "name": "clientOrderId",
          "type": "u64"
        },
        {
          "name": "limit",
          "type": "u16"
        }
      ]
    },
    {
      "name": "serum3CancelOrder",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumMarketExternal",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketBids",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketAsks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketEventQueue",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "side",
          "type": {
            "defined": "Serum3Side"
          }
        },
        {
          "name": "orderId",
          "type": "u128"
        }
      ]
    },
    {
      "name": "serum3CancelAllOrders",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumMarketExternal",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketBids",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketAsks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketEventQueue",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "limit",
          "type": "u8"
        }
      ]
    },
    {
      "name": "serum3SettleFunds",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumMarketExternal",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketBaseVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketQuoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketVaultSigner",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "needed for the automatic settle_funds call"
          ]
        },
        {
          "name": "quoteBank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "quoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "baseBank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "baseVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "serum3LiqForceCancelOrders",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "serumMarketExternal",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketBids",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketAsks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketEventQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketBaseVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketQuoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "marketVaultSigner",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "quoteBank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "quoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "baseBank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "baseVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "limit",
          "type": "u8"
        }
      ]
    },
    {
      "name": "liqTokenWithToken",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "assetTokenIndex",
          "type": "u16"
        },
        {
          "name": "liabTokenIndex",
          "type": "u16"
        },
        {
          "name": "maxLiabTransfer",
          "type": {
            "defined": "I80F48"
          }
        }
      ]
    },
    {
      "name": "liqTokenBankruptcy",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "liabMintInfo",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "quoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "insuranceVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "maxLiabTransfer",
          "type": {
            "defined": "I80F48"
          }
        }
      ]
    },
    {
      "name": "tokenLiqWithToken",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "assetTokenIndex",
          "type": "u16"
        },
        {
          "name": "liabTokenIndex",
          "type": "u16"
        },
        {
          "name": "maxLiabTransfer",
          "type": {
            "defined": "I80F48"
          }
        }
      ]
    },
    {
      "name": "tokenLiqBankruptcy",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "liabMintInfo",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "quoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "insuranceVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "maxLiabTransfer",
          "type": {
            "defined": "I80F48"
          }
        }
      ]
    },
    {
      "name": "perpCreateMarket",
      "docs": [
        "",
        "Perps",
        ""
      ],
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "PerpMarket"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "perp_market_index"
              }
            ]
          }
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Accounts are initialised by client,",
            "anchor discriminator is set first when ix exits,"
          ]
        },
        {
          "name": "eventQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "perpMarketIndex",
          "type": "u16"
        },
        {
          "name": "name",
          "type": "string"
        },
        {
          "name": "oracleConfig",
          "type": {
            "defined": "OracleConfigParams"
          }
        },
        {
          "name": "baseDecimals",
          "type": "u8"
        },
        {
          "name": "quoteLotSize",
          "type": "i64"
        },
        {
          "name": "baseLotSize",
          "type": "i64"
        },
        {
          "name": "maintAssetWeight",
          "type": "f32"
        },
        {
          "name": "initAssetWeight",
          "type": "f32"
        },
        {
          "name": "maintLiabWeight",
          "type": "f32"
        },
        {
          "name": "initLiabWeight",
          "type": "f32"
        },
        {
          "name": "liquidationFee",
          "type": "f32"
        },
        {
          "name": "makerFee",
          "type": "f32"
        },
        {
          "name": "takerFee",
          "type": "f32"
        },
        {
          "name": "minFunding",
          "type": "f32"
        },
        {
          "name": "maxFunding",
          "type": "f32"
        },
        {
          "name": "impactQuantity",
          "type": "i64"
        },
        {
          "name": "groupInsuranceFund",
          "type": "bool"
        },
        {
          "name": "trustedMarket",
          "type": "bool"
        },
        {
          "name": "feePenalty",
          "type": "f32"
        },
        {
          "name": "settleFeeFlat",
          "type": "f32"
        },
        {
          "name": "settleFeeAmountThreshold",
          "type": "f32"
        },
        {
          "name": "settleFeeFractionLowHealth",
          "type": "f32"
        },
        {
          "name": "settleTokenIndex",
          "type": "u16"
        },
        {
          "name": "settlePnlLimitFactor",
          "type": "f32"
        },
        {
          "name": "settlePnlLimitFactorWindowSizeTs",
          "type": "u64"
        }
      ]
    },
    {
      "name": "perpEditMarket",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "oracleOpt",
          "type": {
            "option": "publicKey"
          }
        },
        {
          "name": "oracleConfigOpt",
          "type": {
            "option": {
              "defined": "OracleConfigParams"
            }
          }
        },
        {
          "name": "baseDecimalsOpt",
          "type": {
            "option": "u8"
          }
        },
        {
          "name": "maintAssetWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "initAssetWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "maintLiabWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "initLiabWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "liquidationFeeOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "makerFeeOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "takerFeeOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "minFundingOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "maxFundingOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "impactQuantityOpt",
          "type": {
            "option": "i64"
          }
        },
        {
          "name": "groupInsuranceFundOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "trustedMarketOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "feePenaltyOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "settleFeeFlatOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "settleFeeAmountThresholdOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "settleFeeFractionLowHealthOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "stablePriceDelayIntervalSecondsOpt",
          "type": {
            "option": "u32"
          }
        },
        {
          "name": "stablePriceDelayGrowthLimitOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "stablePriceGrowthLimitOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "settlePnlLimitFactorOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "settlePnlLimitFactorWindowSizeTs",
          "type": {
            "option": "u64"
          }
        }
      ]
    },
    {
      "name": "perpCloseMarket",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "eventQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "solDestination",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "perpDeactivatePosition",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "perpPlaceOrder",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "eventQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "side",
          "type": {
            "defined": "Side"
          }
        },
        {
          "name": "priceLots",
          "type": "i64"
        },
        {
          "name": "maxBaseLots",
          "type": "i64"
        },
        {
          "name": "maxQuoteLots",
          "type": "i64"
        },
        {
          "name": "clientOrderId",
          "type": "u64"
        },
        {
          "name": "orderType",
          "type": {
            "defined": "PlaceOrderType"
          }
        },
        {
          "name": "reduceOnly",
          "type": "bool"
        },
        {
          "name": "expiryTimestamp",
          "type": "u64"
        },
        {
          "name": "limit",
          "type": "u8"
        }
      ]
    },
    {
      "name": "perpPlaceOrderPegged",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "eventQueue",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "side",
          "type": {
            "defined": "Side"
          }
        },
        {
          "name": "priceOffsetLots",
          "type": "i64"
        },
        {
          "name": "pegLimit",
          "type": "i64"
        },
        {
          "name": "maxBaseLots",
          "type": "i64"
        },
        {
          "name": "maxQuoteLots",
          "type": "i64"
        },
        {
          "name": "clientOrderId",
          "type": "u64"
        },
        {
          "name": "orderType",
          "type": {
            "defined": "PlaceOrderType"
          }
        },
        {
          "name": "reduceOnly",
          "type": "bool"
        },
        {
          "name": "expiryTimestamp",
          "type": "u64"
        },
        {
          "name": "limit",
          "type": "u8"
        }
      ]
    },
    {
      "name": "perpCancelOrder",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "orderId",
          "type": "u128"
        }
      ]
    },
    {
      "name": "perpCancelOrderByClientOrderId",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "clientOrderId",
          "type": "u64"
        }
      ]
    },
    {
      "name": "perpCancelAllOrders",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "limit",
          "type": "u8"
        }
      ]
    },
    {
      "name": "perpCancelAllOrdersBySide",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "sideOption",
          "type": {
            "option": {
              "defined": "Side"
            }
          }
        },
        {
          "name": "limit",
          "type": "u8"
        }
      ]
    },
    {
      "name": "perpConsumeEvents",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "eventQueue",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "limit",
          "type": "u64"
        }
      ]
    },
    {
      "name": "perpUpdateFunding",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "perpSettlePnl",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "settler",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "settlerOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "accountA",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "accountB",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "settleBank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "settleOracle",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "perpSettleFees",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "settleBank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "settleOracle",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "maxSettleAmount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "perpLiqBasePosition",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "maxBaseTransfer",
          "type": "i64"
        }
      ]
    },
    {
      "name": "perpLiqForceCancelOrders",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "orderbook",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "limit",
          "type": "u8"
        }
      ]
    },
    {
      "name": "perpLiqBankruptcy",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "settleBank",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "settleVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "settleOracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "insuranceVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "maxLiabTransfer",
          "type": "u64"
        }
      ]
    },
    {
      "name": "altSet",
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "addressLookupTable",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "index",
          "type": "u8"
        }
      ]
    },
    {
      "name": "altExtend",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "payer",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "addressLookupTable",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "index",
          "type": "u8"
        },
        {
          "name": "newAddresses",
          "type": {
            "vec": "publicKey"
          }
        }
      ]
    },
    {
      "name": "computeAccountData",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "benchmark",
      "docs": [
        "",
        "benchmark",
        ""
      ],
      "accounts": [],
      "args": []
    }
  ],
  "accounts": [
    {
      "name": "bank",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "group",
            "type": "publicKey"
          },
          {
            "name": "name",
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          },
          {
            "name": "mint",
            "type": "publicKey"
          },
          {
            "name": "vault",
            "type": "publicKey"
          },
          {
            "name": "oracle",
            "type": "publicKey"
          },
          {
            "name": "oracleConfFilter",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "depositIndex",
            "docs": [
              "the index used to scale the value of an IndexedPosition",
              "TODO: should always be >= 0, add checks?"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "borrowIndex",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "cachedIndexedTotalDeposits",
            "docs": [
              "total deposits/borrows, only updated during UpdateIndexAndRate",
              "TODO: These values could be dropped from the bank, they're written in UpdateIndexAndRate",
              "and never read."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "cachedIndexedTotalBorrows",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "indexedDeposits",
            "docs": [
              "deposits/borrows for this bank",
              "",
              "Note that these may become negative. It's perfectly fine for users to borrow one one bank",
              "(increasing indexed_borrows there) and paying back on another (possibly decreasing indexed_borrows",
              "below zero).",
              "",
              "The vault amount is not deducable from these values.",
              "",
              "These become meaningful when summed over all banks (like in update_index_and_rate)."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "indexedBorrows",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "indexLastUpdated",
            "type": "u64"
          },
          {
            "name": "bankRateLastUpdated",
            "type": "u64"
          },
          {
            "name": "avgUtilization",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "adjustmentFactor",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "util0",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "rate0",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "util1",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "rate1",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maxRate",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "collectedFeesNative",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "loanOriginationFeeRate",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "loanFeeRate",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maintAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maintLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "liquidationFee",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "dust",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "flashLoanTokenAccountInitial",
            "type": "u64"
          },
          {
            "name": "flashLoanApprovedAmount",
            "type": "u64"
          },
          {
            "name": "tokenIndex",
            "type": "u16"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "mintDecimals",
            "type": "u8"
          },
          {
            "name": "bankNum",
            "type": "u32"
          },
          {
            "name": "oracleConfig",
            "type": {
              "defined": "OracleConfig"
            }
          },
          {
            "name": "stablePriceModel",
            "type": {
              "defined": "StablePriceModel"
            }
          },
          {
            "name": "minVaultToDepositsRatio",
            "type": "f64"
          },
          {
            "name": "netBorrowsWindowSizeTs",
            "type": "u64"
          },
          {
            "name": "lastNetBorrowsWindowStartTs",
            "type": "u64"
          },
          {
            "name": "netBorrowsLimitNative",
            "type": "i64"
          },
          {
            "name": "netBorrowsWindowNative",
            "type": "i64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                2136
              ]
            }
          }
        ]
      }
    },
    {
      "name": "group",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "creator",
            "type": "publicKey"
          },
          {
            "name": "groupNum",
            "type": "u32"
          },
          {
            "name": "admin",
            "type": "publicKey"
          },
          {
            "name": "fastListingAdmin",
            "type": "publicKey"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "insuranceVault",
            "type": "publicKey"
          },
          {
            "name": "insuranceMint",
            "type": "publicKey"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "testing",
            "type": "u8"
          },
          {
            "name": "version",
            "type": "u8"
          },
          {
            "name": "padding2",
            "type": {
              "array": [
                "u8",
                5
              ]
            }
          },
          {
            "name": "addressLookupTables",
            "type": {
              "array": [
                "publicKey",
                20
              ]
            }
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                1920
              ]
            }
          }
        ]
      }
    },
    {
      "name": "mangoAccount",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "group",
            "type": "publicKey"
          },
          {
            "name": "owner",
            "type": "publicKey"
          },
          {
            "name": "name",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "delegate",
            "type": "publicKey"
          },
          {
            "name": "accountNum",
            "type": "u32"
          },
          {
            "name": "beingLiquidated",
            "docs": [
              "Tracks that this account should be liquidated until init_health >= 0.",
              "",
              "Normally accounts can not be liquidated while maint_health >= 0. But when an account",
              "reaches maint_health < 0, liquidators will call a liquidation instruction and thereby",
              "set this flag. Now the account may be liquidated until init_health >= 0.",
              "",
              "Many actions should be disabled while the account is being liquidated, even if",
              "its maint health has recovered to positive. Creating new open orders would, for example,",
              "confuse liquidators."
            ],
            "type": "u8"
          },
          {
            "name": "inHealthRegion",
            "docs": [
              "The account is currently inside a health region marked by HealthRegionBegin...HealthRegionEnd.",
              "",
              "Must never be set after a transaction ends."
            ],
            "type": "u8"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                1
              ]
            }
          },
          {
            "name": "netDeposits",
            "type": "i64"
          },
          {
            "name": "perpSpotTransfers",
            "type": "i64"
          },
          {
            "name": "healthRegionBeginInitHealth",
            "docs": [
              "Init health as calculated during HealthReginBegin, rounded up."
            ],
            "type": "i64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                240
              ]
            }
          },
          {
            "name": "headerVersion",
            "type": "u8"
          },
          {
            "name": "padding3",
            "type": {
              "array": [
                "u8",
                7
              ]
            }
          },
          {
            "name": "padding4",
            "type": "u32"
          },
          {
            "name": "tokens",
            "type": {
              "vec": {
                "defined": "TokenPosition"
              }
            }
          },
          {
            "name": "padding5",
            "type": "u32"
          },
          {
            "name": "serum3",
            "type": {
              "vec": {
                "defined": "Serum3Orders"
              }
            }
          },
          {
            "name": "padding6",
            "type": "u32"
          },
          {
            "name": "perps",
            "type": {
              "vec": {
                "defined": "PerpPosition"
              }
            }
          },
          {
            "name": "padding7",
            "type": "u32"
          },
          {
            "name": "perpOpenOrders",
            "type": {
              "vec": {
                "defined": "PerpOpenOrder"
              }
            }
          }
        ]
      }
    },
    {
      "name": "mintInfo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "group",
            "type": "publicKey"
          },
          {
            "name": "tokenIndex",
            "type": "u16"
          },
          {
            "name": "groupInsuranceFund",
            "type": "u8"
          },
          {
            "name": "padding1",
            "type": {
              "array": [
                "u8",
                5
              ]
            }
          },
          {
            "name": "mint",
            "type": "publicKey"
          },
          {
            "name": "banks",
            "type": {
              "array": [
                "publicKey",
                6
              ]
            }
          },
          {
            "name": "vaults",
            "type": {
              "array": [
                "publicKey",
                6
              ]
            }
          },
          {
            "name": "oracle",
            "type": "publicKey"
          },
          {
            "name": "registrationTime",
            "type": "u64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                2560
              ]
            }
          }
        ]
      }
    },
    {
      "name": "stubOracle",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "group",
            "type": "publicKey"
          },
          {
            "name": "mint",
            "type": "publicKey"
          },
          {
            "name": "price",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "lastUpdated",
            "type": "i64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                128
              ]
            }
          }
        ]
      }
    },
    {
      "name": "orderbook",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "bids",
            "type": {
              "defined": "BookSide"
            }
          },
          {
            "name": "asks",
            "type": {
              "defined": "BookSide"
            }
          }
        ]
      }
    },
    {
      "name": "eventQueue",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "header",
            "type": {
              "defined": "EventQueueHeader"
            }
          },
          {
            "name": "buf",
            "type": {
              "array": [
                {
                  "defined": "AnyEvent"
                },
                488
              ]
            }
          }
        ]
      }
    },
    {
      "name": "perpMarket",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "group",
            "type": "publicKey"
          },
          {
            "name": "settleTokenIndex",
            "type": "u16"
          },
          {
            "name": "perpMarketIndex",
            "docs": [
              "Lookup indices"
            ],
            "type": "u16"
          },
          {
            "name": "trustedMarket",
            "docs": [
              "May this market contribute positive values to health?"
            ],
            "type": "u8"
          },
          {
            "name": "groupInsuranceFund",
            "docs": [
              "Is this market covered by the group insurance fund?"
            ],
            "type": "u8"
          },
          {
            "name": "padding1",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "name",
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          },
          {
            "name": "oracle",
            "type": "publicKey"
          },
          {
            "name": "oracleConfig",
            "type": {
              "defined": "OracleConfig"
            }
          },
          {
            "name": "orderbook",
            "type": "publicKey"
          },
          {
            "name": "eventQueue",
            "type": "publicKey"
          },
          {
            "name": "quoteLotSize",
            "docs": [
              "Number of quote native that reresents min tick"
            ],
            "type": "i64"
          },
          {
            "name": "baseLotSize",
            "docs": [
              "Represents number of base native quantity",
              "e.g. if base decimals for underlying asset are 6, base lot size is 100, and base position is 10000, then",
              "UI position is 1"
            ],
            "type": "i64"
          },
          {
            "name": "maintAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maintLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "liquidationFee",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "makerFee",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "takerFee",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "minFunding",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maxFunding",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "impactQuantity",
            "type": "i64"
          },
          {
            "name": "longFunding",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "shortFunding",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "fundingLastUpdated",
            "docs": [
              "timestamp that funding was last updated in"
            ],
            "type": "u64"
          },
          {
            "name": "openInterest",
            "docs": [
              ""
            ],
            "type": "i64"
          },
          {
            "name": "seqNum",
            "docs": [
              "Total number of orders seen"
            ],
            "type": "u64"
          },
          {
            "name": "feesAccrued",
            "docs": [
              "Fees accrued in native quote currency"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "bump",
            "docs": [
              "Liquidity mining metadata",
              "pub liquidity_mining_info: LiquidityMiningInfo,",
              "Token vault which holds mango tokens to be disbursed as liquidity incentives for this perp market",
              "pub mngo_vault: Pubkey,",
              "PDA bump"
            ],
            "type": "u8"
          },
          {
            "name": "baseDecimals",
            "type": "u8"
          },
          {
            "name": "padding2",
            "type": {
              "array": [
                "u8",
                6
              ]
            }
          },
          {
            "name": "registrationTime",
            "type": "u64"
          },
          {
            "name": "feesSettled",
            "docs": [
              "Fees settled in native quote currency"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "feePenalty",
            "type": "f32"
          },
          {
            "name": "settleFeeFlat",
            "docs": [
              "In native units of settlement token, given to each settle call above the",
              "settle_fee_amount_threshold."
            ],
            "type": "f32"
          },
          {
            "name": "settleFeeAmountThreshold",
            "docs": [
              "Pnl settlement amount needed to be eligible for fees."
            ],
            "type": "f32"
          },
          {
            "name": "settleFeeFractionLowHealth",
            "docs": [
              "Fraction of pnl to pay out as fee if +pnl account has low health."
            ],
            "type": "f32"
          },
          {
            "name": "stablePriceModel",
            "type": {
              "defined": "StablePriceModel"
            }
          },
          {
            "name": "settlePnlLimitFactor",
            "type": "f32"
          },
          {
            "name": "settlePnlLimitFactorWindowSizeTs",
            "type": "u64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                1944
              ]
            }
          }
        ]
      }
    },
    {
      "name": "serum3Market",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "group",
            "type": "publicKey"
          },
          {
            "name": "baseTokenIndex",
            "type": "u16"
          },
          {
            "name": "quoteTokenIndex",
            "type": "u16"
          },
          {
            "name": "padding1",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "name",
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          },
          {
            "name": "serumProgram",
            "type": "publicKey"
          },
          {
            "name": "serumMarketExternal",
            "type": "publicKey"
          },
          {
            "name": "marketIndex",
            "type": "u16"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "padding2",
            "type": {
              "array": [
                "u8",
                5
              ]
            }
          },
          {
            "name": "registrationTime",
            "type": "u64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                128
              ]
            }
          }
        ]
      }
    },
    {
      "name": "serum3MarketIndexReservation",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "group",
            "type": "publicKey"
          },
          {
            "name": "marketIndex",
            "type": "u16"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                38
              ]
            }
          }
        ]
      }
    }
  ],
  "types": [
    {
      "name": "Equity",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tokens",
            "type": {
              "vec": {
                "defined": "TokenEquity"
              }
            }
          },
          {
            "name": "perps",
            "type": {
              "vec": {
                "defined": "PerpEquity"
              }
            }
          }
        ]
      }
    },
    {
      "name": "TokenEquity",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tokenIndex",
            "type": "u16"
          },
          {
            "name": "value",
            "type": {
              "defined": "I80F48"
            }
          }
        ]
      }
    },
    {
      "name": "PerpEquity",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "perpMarketIndex",
            "type": "u16"
          },
          {
            "name": "value",
            "type": {
              "defined": "I80F48"
            }
          }
        ]
      }
    },
    {
      "name": "InterestRateParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "util0",
            "type": "f32"
          },
          {
            "name": "rate0",
            "type": "f32"
          },
          {
            "name": "util1",
            "type": "f32"
          },
          {
            "name": "rate1",
            "type": "f32"
          },
          {
            "name": "maxRate",
            "type": "f32"
          },
          {
            "name": "adjustmentFactor",
            "type": "f32"
          }
        ]
      }
    },
    {
      "name": "FlashLoanTokenDetail",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tokenIndex",
            "type": "u16"
          },
          {
            "name": "changeAmount",
            "type": "i128"
          },
          {
            "name": "loan",
            "type": "i128"
          },
          {
            "name": "loanOriginationFee",
            "type": "i128"
          },
          {
            "name": "depositIndex",
            "type": "i128"
          },
          {
            "name": "borrowIndex",
            "type": "i128"
          },
          {
            "name": "price",
            "type": "i128"
          }
        ]
      }
    },
    {
      "name": "Prices",
      "docs": [
        "Information about prices for a bank or perp market."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "oracle",
            "docs": [
              "The current oracle price"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "stable",
            "docs": [
              "A \"stable\" price, provided by StablePriceModel"
            ],
            "type": {
              "defined": "I80F48"
            }
          }
        ]
      }
    },
    {
      "name": "TokenInfo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tokenIndex",
            "type": "u16"
          },
          {
            "name": "maintAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maintLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "prices",
            "type": {
              "defined": "Prices"
            }
          },
          {
            "name": "balanceNative",
            "type": {
              "defined": "I80F48"
            }
          }
        ]
      }
    },
    {
      "name": "Serum3Info",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "reservedBase",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "reservedQuote",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "baseIndex",
            "type": "u64"
          },
          {
            "name": "quoteIndex",
            "type": "u64"
          },
          {
            "name": "marketIndex",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "PerpInfo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "perpMarketIndex",
            "type": "u16"
          },
          {
            "name": "maintAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maintLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "baseLotSize",
            "type": "i64"
          },
          {
            "name": "baseLots",
            "type": "i64"
          },
          {
            "name": "bidsBaseLots",
            "type": "i64"
          },
          {
            "name": "asksBaseLots",
            "type": "i64"
          },
          {
            "name": "quote",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "prices",
            "type": {
              "defined": "Prices"
            }
          },
          {
            "name": "hasOpenOrders",
            "type": "bool"
          },
          {
            "name": "trustedMarket",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "HealthCache",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tokenInfos",
            "type": {
              "vec": {
                "defined": "TokenInfo"
              }
            }
          },
          {
            "name": "serum3Infos",
            "type": {
              "vec": {
                "defined": "Serum3Info"
              }
            }
          },
          {
            "name": "perpInfos",
            "type": {
              "vec": {
                "defined": "PerpInfo"
              }
            }
          },
          {
            "name": "beingLiquidated",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "TokenPosition",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "indexedPosition",
            "docs": [
              "The deposit_index (if positive) or borrow_index (if negative) scaled position"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "tokenIndex",
            "docs": [
              "index into Group.tokens"
            ],
            "type": "u16"
          },
          {
            "name": "inUseCount",
            "docs": [
              "incremented when a market requires this position to stay alive"
            ],
            "type": "u8"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                5
              ]
            }
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                8
              ]
            }
          },
          {
            "name": "previousIndex",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "cumulativeDepositInterest",
            "type": "f64"
          },
          {
            "name": "cumulativeBorrowInterest",
            "type": "f64"
          }
        ]
      }
    },
    {
      "name": "Serum3Orders",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "openOrders",
            "type": "publicKey"
          },
          {
            "name": "baseBorrowsWithoutFee",
            "docs": [
              "Tracks the amount of borrows that have flowed into the serum open orders account.",
              "These borrows did not have the loan origination fee applied, and that may happen",
              "later (in serum3_settle_funds) if we can guarantee that the funds were used.",
              "In particular a place-on-book, cancel, settle should not cost fees."
            ],
            "type": "u64"
          },
          {
            "name": "quoteBorrowsWithoutFee",
            "type": "u64"
          },
          {
            "name": "marketIndex",
            "type": "u16"
          },
          {
            "name": "baseTokenIndex",
            "docs": [
              "Store the base/quote token index, so health computations don't need",
              "to get passed the static SerumMarket to find which tokens a market",
              "uses and look up the correct oracles."
            ],
            "type": "u16"
          },
          {
            "name": "quoteTokenIndex",
            "type": "u16"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          }
        ]
      }
    },
    {
      "name": "PerpPosition",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "marketIndex",
            "type": "u16"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "settlePnlLimitWindow",
            "type": "u32"
          },
          {
            "name": "basePositionLots",
            "docs": [
              "Active position size, measured in base lots"
            ],
            "type": "i64"
          },
          {
            "name": "quotePositionNative",
            "docs": [
              "Active position in quote (conversation rate is that of the time the order was settled)",
              "measured in native quote"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "padding2",
            "docs": [
              "Tracks what the position is to calculate average entry & break even price"
            ],
            "type": {
              "array": [
                "u8",
                8
              ]
            }
          },
          {
            "name": "quoteRunningNative",
            "type": "i64"
          },
          {
            "name": "longSettledFunding",
            "docs": [
              "Already settled funding"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "shortSettledFunding",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "bidsBaseLots",
            "docs": [
              "Base lots in bids"
            ],
            "type": "i64"
          },
          {
            "name": "asksBaseLots",
            "docs": [
              "Base lots in asks"
            ],
            "type": "i64"
          },
          {
            "name": "takerBaseLots",
            "docs": [
              "Liquidity mining rewards",
              "Amount that's on EventQueue waiting to be processed"
            ],
            "type": "i64"
          },
          {
            "name": "takerQuoteLots",
            "type": "i64"
          },
          {
            "name": "cumulativeLongFunding",
            "type": "f64"
          },
          {
            "name": "cumulativeShortFunding",
            "type": "f64"
          },
          {
            "name": "makerVolume",
            "type": "u64"
          },
          {
            "name": "takerVolume",
            "type": "u64"
          },
          {
            "name": "perpSpotTransfers",
            "type": "i64"
          },
          {
            "name": "avgEntryPricePerBaseLot",
            "type": "f64"
          },
          {
            "name": "realizedPnlNative",
            "type": "i64"
          },
          {
            "name": "settlePnlLimitSettledInCurrentWindowNative",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "PerpOpenOrder",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sideAndTree",
            "type": {
              "defined": "SideAndOrderTree"
            }
          },
          {
            "name": "padding1",
            "type": {
              "array": [
                "u8",
                1
              ]
            }
          },
          {
            "name": "market",
            "type": "u16"
          },
          {
            "name": "padding2",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "clientId",
            "type": "u64"
          },
          {
            "name": "id",
            "type": "u128"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          }
        ]
      }
    },
    {
      "name": "OracleConfig",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "confFilter",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maxStalenessSlots",
            "type": "i64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                72
              ]
            }
          }
        ]
      }
    },
    {
      "name": "OracleConfigParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "confFilter",
            "type": "f32"
          },
          {
            "name": "maxStalenessSlots",
            "type": {
              "option": "u32"
            }
          }
        ]
      }
    },
    {
      "name": "BookSide",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "fixed",
            "type": {
              "defined": "OrderTree"
            }
          },
          {
            "name": "oraclePegged",
            "type": {
              "defined": "OrderTree"
            }
          }
        ]
      }
    },
    {
      "name": "InnerNode",
      "docs": [
        "InnerNodes and LeafNodes compose the binary tree of orders.",
        "",
        "Each InnerNode has exactly two children, which are either InnerNodes themselves,",
        "or LeafNodes. The children share the top `prefix_len` bits of `key`. The left",
        "child has a 0 in the next bit, and the right a 1."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tag",
            "type": "u32"
          },
          {
            "name": "prefixLen",
            "docs": [
              "number of highest `key` bits that all children share",
              "e.g. if it's 2, the two highest bits of `key` will be the same on all children"
            ],
            "type": "u32"
          },
          {
            "name": "key",
            "docs": [
              "only the top `prefix_len` bits of `key` are relevant"
            ],
            "type": "u128"
          },
          {
            "name": "children",
            "docs": [
              "indexes into `BookSide::nodes`"
            ],
            "type": {
              "array": [
                "u32",
                2
              ]
            }
          },
          {
            "name": "childEarliestExpiry",
            "docs": [
              "The earliest expiry timestamp for the left and right subtrees.",
              "",
              "Needed to be able to find and remove expired orders without having to",
              "iterate through the whole bookside."
            ],
            "type": {
              "array": [
                "u64",
                2
              ]
            }
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                48
              ]
            }
          }
        ]
      }
    },
    {
      "name": "LeafNode",
      "docs": [
        "LeafNodes represent an order in the binary tree"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tag",
            "type": "u32"
          },
          {
            "name": "ownerSlot",
            "type": "u8"
          },
          {
            "name": "orderType",
            "type": {
              "defined": "PostOrderType"
            }
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                1
              ]
            }
          },
          {
            "name": "timeInForce",
            "docs": [
              "Time in seconds after `timestamp` at which the order expires.",
              "A value of 0 means no expiry."
            ],
            "type": "u8"
          },
          {
            "name": "key",
            "docs": [
              "The binary tree key"
            ],
            "type": "u128"
          },
          {
            "name": "owner",
            "type": "publicKey"
          },
          {
            "name": "quantity",
            "type": "i64"
          },
          {
            "name": "clientOrderId",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "u64"
          },
          {
            "name": "pegLimit",
            "type": "i64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                8
              ]
            }
          }
        ]
      }
    },
    {
      "name": "AnyNode",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tag",
            "type": "u32"
          },
          {
            "name": "data",
            "type": {
              "array": [
                "u8",
                92
              ]
            }
          }
        ]
      }
    },
    {
      "name": "OrderTree",
      "docs": [
        "A binary tree on AnyNode::key()",
        "",
        "The key encodes the price in the top 64 bits."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "orderTreeType",
            "type": {
              "defined": "OrderTreeType"
            }
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                3
              ]
            }
          },
          {
            "name": "bumpIndex",
            "type": "u32"
          },
          {
            "name": "freeListLen",
            "type": "u32"
          },
          {
            "name": "freeListHead",
            "type": "u32"
          },
          {
            "name": "rootNode",
            "type": "u32"
          },
          {
            "name": "leafCount",
            "type": "u32"
          },
          {
            "name": "nodes",
            "type": {
              "array": [
                {
                  "defined": "AnyNode"
                },
                1024
              ]
            }
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                256
              ]
            }
          }
        ]
      }
    },
    {
      "name": "EventQueueHeader",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "head",
            "type": "u32"
          },
          {
            "name": "count",
            "type": "u32"
          },
          {
            "name": "seqNum",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "AnyEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "eventType",
            "type": "u8"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                207
              ]
            }
          }
        ]
      }
    },
    {
      "name": "FillEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "eventType",
            "type": "u8"
          },
          {
            "name": "takerSide",
            "type": {
              "defined": "Side"
            }
          },
          {
            "name": "makerOut",
            "type": "bool"
          },
          {
            "name": "makerSlot",
            "type": "u8"
          },
          {
            "name": "marketFeesApplied",
            "type": "bool"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                3
              ]
            }
          },
          {
            "name": "timestamp",
            "type": "u64"
          },
          {
            "name": "seqNum",
            "type": "u64"
          },
          {
            "name": "maker",
            "type": "publicKey"
          },
          {
            "name": "makerOrderId",
            "type": "u128"
          },
          {
            "name": "makerClientOrderId",
            "type": "u64"
          },
          {
            "name": "makerFee",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "makerTimestamp",
            "type": "u64"
          },
          {
            "name": "taker",
            "type": "publicKey"
          },
          {
            "name": "takerOrderId",
            "type": "u128"
          },
          {
            "name": "takerClientOrderId",
            "type": "u64"
          },
          {
            "name": "takerFee",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "price",
            "type": "i64"
          },
          {
            "name": "quantity",
            "type": "i64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          }
        ]
      }
    },
    {
      "name": "OutEvent",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "eventType",
            "type": "u8"
          },
          {
            "name": "side",
            "type": {
              "defined": "Side"
            }
          },
          {
            "name": "ownerSlot",
            "type": "u8"
          },
          {
            "name": "padding0",
            "type": {
              "array": [
                "u8",
                5
              ]
            }
          },
          {
            "name": "timestamp",
            "type": "u64"
          },
          {
            "name": "seqNum",
            "type": "u64"
          },
          {
            "name": "owner",
            "type": "publicKey"
          },
          {
            "name": "quantity",
            "type": "i64"
          },
          {
            "name": "padding1",
            "type": {
              "array": [
                "u8",
                144
              ]
            }
          }
        ]
      }
    },
    {
      "name": "StablePriceModel",
      "docs": [
        "Maintains a \"stable_price\" based on the oracle price.",
        "",
        "The stable price follows the oracle price, but its relative rate of",
        "change is limited (to `stable_growth_limit`) and futher reduced if",
        "the oracle price is far from the `delay_price`.",
        "",
        "Conceptually the `delay_price` is itself a time delayed",
        "(`24 * delay_interval_seconds`, assume 24h) and relative rate of change limited",
        "function of the oracle price. It is implemented as averaging the oracle",
        "price over every `delay_interval_seconds` (assume 1h) and then applying the",
        "`delay_growth_limit` between intervals."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "stablePrice",
            "docs": [
              "Current stable price to use in health"
            ],
            "type": "f64"
          },
          {
            "name": "lastUpdateTimestamp",
            "type": "u64"
          },
          {
            "name": "delayPrices",
            "docs": [
              "Stored delay_price for each delay_interval.",
              "If we want the delay_price to be 24h delayed, we would store one for each hour.",
              "This is used in a cyclical way: We use the maximally-delayed value at delay_interval_index",
              "and once enough time passes to move to the next delay interval, that gets overwritten and",
              "we use the next one."
            ],
            "type": {
              "array": [
                "f64",
                24
              ]
            }
          },
          {
            "name": "delayAccumulatorPrice",
            "docs": [
              "The delay price is based on an average over each delay_interval. The contributions",
              "to the average are summed up here."
            ],
            "type": "f64"
          },
          {
            "name": "delayAccumulatorTime",
            "docs": [
              "Accumulating the total time for the above average."
            ],
            "type": "u32"
          },
          {
            "name": "delayIntervalSeconds",
            "docs": [
              "Length of a delay_interval"
            ],
            "type": "u32"
          },
          {
            "name": "delayGrowthLimit",
            "docs": [
              "Maximal relative difference between two delay_price in consecutive intervals."
            ],
            "type": "f32"
          },
          {
            "name": "stableGrowthLimit",
            "docs": [
              "Maximal per-second relative difference of the stable price.",
              "It gets further reduced if stable and delay price disagree."
            ],
            "type": "f32"
          },
          {
            "name": "lastDelayIntervalIndex",
            "docs": [
              "The delay_interval_index that update() was last called on."
            ],
            "type": "u8"
          },
          {
            "name": "padding",
            "type": {
              "array": [
                "u8",
                7
              ]
            }
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                48
              ]
            }
          }
        ]
      }
    },
    {
      "name": "TokenIndex",
      "docs": [
        "Nothing in Rust shall use these types. They only exist so that the Anchor IDL",
        "knows about them and typescript can deserialize it."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "val",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "Serum3MarketIndex",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "val",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "PerpMarketIndex",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "val",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "I80F48",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "val",
            "type": "i128"
          }
        ]
      }
    },
    {
      "name": "FlashLoanType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Unknown"
          },
          {
            "name": "Swap"
          }
        ]
      }
    },
    {
      "name": "Serum3SelfTradeBehavior",
      "docs": [
        "Copy paste a bunch of enums so that we could AnchorSerialize & AnchorDeserialize them"
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "DecrementTake"
          },
          {
            "name": "CancelProvide"
          },
          {
            "name": "AbortTransaction"
          }
        ]
      }
    },
    {
      "name": "Serum3OrderType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Limit"
          },
          {
            "name": "ImmediateOrCancel"
          },
          {
            "name": "PostOnly"
          }
        ]
      }
    },
    {
      "name": "Serum3Side",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Bid"
          },
          {
            "name": "Ask"
          }
        ]
      }
    },
    {
      "name": "LoanOriginationFeeInstruction",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Unknown"
          },
          {
            "name": "LiqTokenBankruptcy"
          },
          {
            "name": "LiqTokenWithToken"
          },
          {
            "name": "Serum3LiqForceCancelOrders"
          },
          {
            "name": "Serum3PlaceOrder"
          },
          {
            "name": "Serum3SettleFunds"
          },
          {
            "name": "TokenWithdraw"
          }
        ]
      }
    },
    {
      "name": "HealthType",
      "docs": [
        "There are two types of health, initial health used for opening new positions and maintenance",
        "health used for liquidations. They are both calculated as a weighted sum of the assets",
        "minus the liabilities but the maint. health uses slightly larger weights for assets and",
        "slightly smaller weights for the liabilities. Zero is used as the bright line for both",
        "i.e. if your init health falls below zero, you cannot open new positions and if your maint. health",
        "falls below zero you will be liquidated."
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Init"
          },
          {
            "name": "Maint"
          }
        ]
      }
    },
    {
      "name": "OracleType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Pyth"
          },
          {
            "name": "Stub"
          },
          {
            "name": "SwitchboardV1"
          },
          {
            "name": "SwitchboardV2"
          }
        ]
      }
    },
    {
      "name": "OrderState",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Valid"
          },
          {
            "name": "Invalid"
          },
          {
            "name": "Skipped"
          }
        ]
      }
    },
    {
      "name": "BookSideOrderTree",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Fixed"
          },
          {
            "name": "OraclePegged"
          }
        ]
      }
    },
    {
      "name": "NodeTag",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Uninitialized"
          },
          {
            "name": "InnerNode"
          },
          {
            "name": "LeafNode"
          },
          {
            "name": "FreeNode"
          },
          {
            "name": "LastFreeNode"
          }
        ]
      }
    },
    {
      "name": "PlaceOrderType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Limit"
          },
          {
            "name": "ImmediateOrCancel"
          },
          {
            "name": "PostOnly"
          },
          {
            "name": "Market"
          },
          {
            "name": "PostOnlySlide"
          }
        ]
      }
    },
    {
      "name": "PostOrderType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Limit"
          },
          {
            "name": "PostOnly"
          },
          {
            "name": "PostOnlySlide"
          }
        ]
      }
    },
    {
      "name": "Side",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Bid"
          },
          {
            "name": "Ask"
          }
        ]
      }
    },
    {
      "name": "SideAndOrderTree",
      "docs": [
        "SideAndOrderTree is a storage optimization, so we don't need two bytes for the data"
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "BidFixed"
          },
          {
            "name": "AskFixed"
          },
          {
            "name": "BidOraclePegged"
          },
          {
            "name": "AskOraclePegged"
          }
        ]
      }
    },
    {
      "name": "OrderParams",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Market"
          },
          {
            "name": "ImmediateOrCancel",
            "fields": [
              {
                "name": "price_lots",
                "type": "i64"
              }
            ]
          },
          {
            "name": "Fixed",
            "fields": [
              {
                "name": "price_lots",
                "type": "i64"
              },
              {
                "name": "order_type",
                "type": {
                  "defined": "PostOrderType"
                }
              }
            ]
          },
          {
            "name": "OraclePegged",
            "fields": [
              {
                "name": "price_offset_lots",
                "type": "i64"
              },
              {
                "name": "order_type",
                "type": {
                  "defined": "PostOrderType"
                }
              },
              {
                "name": "peg_limit",
                "type": "i64"
              }
            ]
          }
        ]
      }
    },
    {
      "name": "OrderTreeType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Bids"
          },
          {
            "name": "Asks"
          }
        ]
      }
    },
    {
      "name": "EventType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Fill"
          },
          {
            "name": "Out"
          },
          {
            "name": "Liquidate"
          }
        ]
      }
    }
  ],
  "events": [
    {
      "name": "MangoAccountData",
      "fields": [
        {
          "name": "healthCache",
          "type": {
            "defined": "HealthCache"
          },
          "index": false
        },
        {
          "name": "initHealth",
          "type": {
            "defined": "I80F48"
          },
          "index": false
        },
        {
          "name": "maintHealth",
          "type": {
            "defined": "I80F48"
          },
          "index": false
        },
        {
          "name": "equity",
          "type": {
            "defined": "Equity"
          },
          "index": false
        }
      ]
    },
    {
      "name": "PerpBalanceLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "marketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "basePosition",
          "type": "i64",
          "index": false
        },
        {
          "name": "quotePosition",
          "type": "i128",
          "index": false
        },
        {
          "name": "longSettledFunding",
          "type": "i128",
          "index": false
        },
        {
          "name": "shortSettledFunding",
          "type": "i128",
          "index": false
        },
        {
          "name": "longFunding",
          "type": "i128",
          "index": false
        },
        {
          "name": "shortFunding",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "TokenBalanceLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "indexedPosition",
          "type": "i128",
          "index": false
        },
        {
          "name": "depositIndex",
          "type": "i128",
          "index": false
        },
        {
          "name": "borrowIndex",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "FlashLoanLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenLoanDetails",
          "type": {
            "vec": {
              "defined": "FlashLoanTokenDetail"
            }
          },
          "index": false
        },
        {
          "name": "flashLoanType",
          "type": {
            "defined": "FlashLoanType"
          },
          "index": false
        }
      ]
    },
    {
      "name": "WithdrawLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "signer",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "quantity",
          "type": "u64",
          "index": false
        },
        {
          "name": "price",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "DepositLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "signer",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "quantity",
          "type": "u64",
          "index": false
        },
        {
          "name": "price",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "FillLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "marketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "takerSide",
          "type": "u8",
          "index": false
        },
        {
          "name": "makerSlot",
          "type": "u8",
          "index": false
        },
        {
          "name": "marketFeesApplied",
          "type": "bool",
          "index": false
        },
        {
          "name": "makerOut",
          "type": "bool",
          "index": false
        },
        {
          "name": "timestamp",
          "type": "u64",
          "index": false
        },
        {
          "name": "seqNum",
          "type": "u64",
          "index": false
        },
        {
          "name": "maker",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "makerOrderId",
          "type": "u128",
          "index": false
        },
        {
          "name": "makerClientOrderId",
          "type": "u64",
          "index": false
        },
        {
          "name": "makerFee",
          "type": "i128",
          "index": false
        },
        {
          "name": "makerTimestamp",
          "type": "u64",
          "index": false
        },
        {
          "name": "taker",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "takerOrderId",
          "type": "u128",
          "index": false
        },
        {
          "name": "takerClientOrderId",
          "type": "u64",
          "index": false
        },
        {
          "name": "takerFee",
          "type": "i128",
          "index": false
        },
        {
          "name": "price",
          "type": "i64",
          "index": false
        },
        {
          "name": "quantity",
          "type": "i64",
          "index": false
        }
      ]
    },
    {
      "name": "PerpUpdateFundingLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "marketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "longFunding",
          "type": "i128",
          "index": false
        },
        {
          "name": "shortFunding",
          "type": "i128",
          "index": false
        },
        {
          "name": "price",
          "type": "i128",
          "index": false
        },
        {
          "name": "stablePrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "feesAccrued",
          "type": "i128",
          "index": false
        },
        {
          "name": "openInterest",
          "type": "i64",
          "index": false
        }
      ]
    },
    {
      "name": "UpdateIndexLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "depositIndex",
          "type": "i128",
          "index": false
        },
        {
          "name": "borrowIndex",
          "type": "i128",
          "index": false
        },
        {
          "name": "avgUtilization",
          "type": "i128",
          "index": false
        },
        {
          "name": "price",
          "type": "i128",
          "index": false
        },
        {
          "name": "stablePrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "collectedFees",
          "type": "i128",
          "index": false
        },
        {
          "name": "loanFeeRate",
          "type": "i128",
          "index": false
        },
        {
          "name": "totalBorrows",
          "type": "i128",
          "index": false
        },
        {
          "name": "totalDeposits",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "UpdateRateLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "rate0",
          "type": "i128",
          "index": false
        },
        {
          "name": "rate1",
          "type": "i128",
          "index": false
        },
        {
          "name": "maxRate",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "TokenLiqWithTokenLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "liqee",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "liqor",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "assetTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "liabTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "assetTransfer",
          "type": "i128",
          "index": false
        },
        {
          "name": "liabTransfer",
          "type": "i128",
          "index": false
        },
        {
          "name": "assetPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "liabPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "bankruptcy",
          "type": "bool",
          "index": false
        }
      ]
    },
    {
      "name": "Serum3OpenOrdersBalanceLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "baseTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "quoteTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "baseTotal",
          "type": "u64",
          "index": false
        },
        {
          "name": "baseFree",
          "type": "u64",
          "index": false
        },
        {
          "name": "quoteTotal",
          "type": "u64",
          "index": false
        },
        {
          "name": "quoteFree",
          "type": "u64",
          "index": false
        },
        {
          "name": "referrerRebatesAccrued",
          "type": "u64",
          "index": false
        }
      ]
    },
    {
      "name": "WithdrawLoanOriginationFeeLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "loanOriginationFee",
          "type": "i128",
          "index": false
        },
        {
          "name": "instruction",
          "type": {
            "defined": "LoanOriginationFeeInstruction"
          },
          "index": false
        }
      ]
    },
    {
      "name": "TokenLiqBankruptcyLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "liqee",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "liqor",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "liabTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "initialLiabNative",
          "type": "i128",
          "index": false
        },
        {
          "name": "liabPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "insuranceTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "insuranceTransfer",
          "type": "i128",
          "index": false
        },
        {
          "name": "socializedLoss",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "DeactivateTokenPositionLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "cumulativeDepositInterest",
          "type": "f64",
          "index": false
        },
        {
          "name": "cumulativeBorrowInterest",
          "type": "f64",
          "index": false
        }
      ]
    },
    {
      "name": "DeactivatePerpPositionLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "marketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "cumulativeLongFunding",
          "type": "f64",
          "index": false
        },
        {
          "name": "cumulativeShortFunding",
          "type": "f64",
          "index": false
        },
        {
          "name": "makerVolume",
          "type": "u64",
          "index": false
        },
        {
          "name": "takerVolume",
          "type": "u64",
          "index": false
        },
        {
          "name": "perpSpotTransfers",
          "type": "i64",
          "index": false
        }
      ]
    },
    {
      "name": "TokenMetaDataLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mint",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "mintDecimals",
          "type": "u8",
          "index": false
        },
        {
          "name": "oracle",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mintInfo",
          "type": "publicKey",
          "index": false
        }
      ]
    },
    {
      "name": "PerpMarketMetaDataLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "perpMarket",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "perpMarketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "baseDecimals",
          "type": "u8",
          "index": false
        },
        {
          "name": "baseLotSize",
          "type": "i64",
          "index": false
        },
        {
          "name": "quoteLotSize",
          "type": "i64",
          "index": false
        },
        {
          "name": "oracle",
          "type": "publicKey",
          "index": false
        }
      ]
    },
    {
      "name": "Serum3RegisterMarketLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "serumMarket",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "marketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "baseTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "quoteTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "serumProgram",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "serumProgramExternal",
          "type": "publicKey",
          "index": false
        }
      ]
    },
    {
      "name": "PerpLiqBasePositionLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "perpMarketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "liqor",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "liqee",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "baseTransfer",
          "type": "i64",
          "index": false
        },
        {
          "name": "quoteTransfer",
          "type": "i128",
          "index": false
        },
        {
          "name": "price",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "PerpLiqBankruptcyLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "liqee",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "liqor",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "perpMarketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "insuranceTransfer",
          "type": "i128",
          "index": false
        },
        {
          "name": "socializedLoss",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "PerpSettlePnlLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccountA",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccountB",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "perpMarketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "settlement",
          "type": "i128",
          "index": false
        },
        {
          "name": "settler",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "fee",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "PerpSettleFeesLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "perpMarketIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "settlement",
          "type": "i128",
          "index": false
        }
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "SomeError",
      "msg": ""
    },
    {
      "code": 6001,
      "name": "NotImplementedError",
      "msg": ""
    },
    {
      "code": 6002,
      "name": "MathError",
      "msg": "checked math error"
    },
    {
      "code": 6003,
      "name": "UnexpectedOracle",
      "msg": ""
    },
    {
      "code": 6004,
      "name": "UnknownOracleType",
      "msg": "oracle type cannot be determined"
    },
    {
      "code": 6005,
      "name": "InvalidFlashLoanTargetCpiProgram",
      "msg": ""
    },
    {
      "code": 6006,
      "name": "HealthMustBePositive",
      "msg": "health must be positive"
    },
    {
      "code": 6007,
      "name": "HealthMustBePositiveOrIncrease",
      "msg": "health must be positive or increase"
    },
    {
      "code": 6008,
      "name": "HealthMustBeNegative",
      "msg": "health must be negative"
    },
    {
      "code": 6009,
      "name": "IsBankrupt",
      "msg": "the account is bankrupt"
    },
    {
      "code": 6010,
      "name": "IsNotBankrupt",
      "msg": "the account is not bankrupt"
    },
    {
      "code": 6011,
      "name": "NoFreeTokenPositionIndex",
      "msg": "no free token position index"
    },
    {
      "code": 6012,
      "name": "NoFreeSerum3OpenOrdersIndex",
      "msg": "no free serum3 open orders index"
    },
    {
      "code": 6013,
      "name": "NoFreePerpPositionIndex",
      "msg": "no free perp position index"
    },
    {
      "code": 6014,
      "name": "Serum3OpenOrdersExistAlready",
      "msg": "serum3 open orders exist already"
    },
    {
      "code": 6015,
      "name": "InsufficentBankVaultFunds",
      "msg": "bank vault has insufficent funds"
    },
    {
      "code": 6016,
      "name": "BeingLiquidated",
      "msg": "account is currently being liquidated"
    },
    {
      "code": 6017,
      "name": "InvalidBank",
      "msg": "invalid bank"
    },
    {
      "code": 6018,
      "name": "ProfitabilityMismatch",
      "msg": "account profitability is mismatched"
    },
    {
      "code": 6019,
      "name": "CannotSettleWithSelf",
      "msg": "cannot settle with self"
    },
    {
      "code": 6020,
      "name": "PerpPositionDoesNotExist",
      "msg": "perp position does not exist"
    },
    {
      "code": 6021,
      "name": "MaxSettleAmountMustBeGreaterThanZero",
      "msg": "max settle amount must be greater than zero"
    },
    {
      "code": 6022,
      "name": "HasOpenPerpOrders",
      "msg": "the perp position has open orders or unprocessed fill events"
    },
    {
      "code": 6023,
      "name": "OracleConfidence",
      "msg": "an oracle does not reach the confidence threshold"
    },
    {
      "code": 6024,
      "name": "OracleStale",
      "msg": "an oracle is stale"
    },
    {
      "code": 6025,
      "name": "SettlementAmountMustBePositive",
      "msg": "settlement amount must always be positive"
    },
    {
      "code": 6026,
      "name": "BankBorrowLimitReached",
      "msg": "bank utilization has reached limit"
    },
    {
      "code": 6027,
      "name": "BankNetBorrowsLimitReached",
      "msg": "bank net borrows has reached limit - this is an intermittent error - the limit will reset regularly"
    }
  ]
};
