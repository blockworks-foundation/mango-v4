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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "InsuranceVault"
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
          "name": "newAdmin",
          "type": "publicKey"
        },
        {
          "name": "newFastListingAdmin",
          "type": "publicKey"
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "Bank"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "arg",
                "type": "u64",
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "Vault"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "arg",
                "type": "u64",
                "path": "bank_num"
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "MintInfo"
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
          "name": "bankNum",
          "type": "u64"
        },
        {
          "name": "name",
          "type": "string"
        },
        {
          "name": "oracleConfig",
          "type": {
            "defined": "OracleConfig"
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
        }
      ],
      "args": [
        {
          "name": "bankNum",
          "type": "u64"
        },
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
              "defined": "OracleConfig"
            }
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "Bank"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "arg",
                "type": "u64",
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "Vault"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "arg",
                "type": "u64",
                "path": "bank_num"
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "MintInfo"
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
          "type": "u64"
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
      "args": [
        {
          "name": "tokenIndex",
          "type": "u16"
        }
      ]
    },
    {
      "name": "tokenUpdateIndexAndRate",
      "accounts": [
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "MangoAccount"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "owner"
              },
              {
                "kind": "arg",
                "type": "u8",
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
          "type": "u8"
        },
        {
          "name": "accountSize",
          "type": {
            "defined": "AccountSize"
          }
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
      "args": []
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "StubOracle"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "account": "Mint",
                "path": "token_mint"
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
          "name": "tokenMint",
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
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
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
      "name": "flashLoan",
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
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "withdraws",
          "type": {
            "vec": {
              "defined": "FlashLoanWithdraw"
            }
          }
        },
        {
          "name": "cpiDatas",
          "type": {
            "vec": {
              "defined": "CpiData"
            }
          }
        }
      ]
    },
    {
      "name": "flashLoan2Begin",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "temporaryVaultAuthority",
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
          "isSigner": false
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
      "name": "flashLoan2End",
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
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "flashLoan3Begin",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
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
      "name": "flashLoan3End",
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "Serum3Market"
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
                "kind": "account",
                "type": "publicKey",
                "path": "account"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "Serum3OO"
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "PerpMarket"
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
          "name": "bids",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Accounts are initialised by client,",
            "anchor discriminator is set first when ix exits,"
          ]
        },
        {
          "name": "asks",
          "isMut": true,
          "isSigner": false
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
            "defined": "OracleConfig"
          }
        },
        {
          "name": "baseTokenIndexOpt",
          "type": {
            "option": "u16"
          }
        },
        {
          "name": "baseTokenDecimals",
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
              "defined": "OracleConfig"
            }
          }
        },
        {
          "name": "baseTokenIndexOpt",
          "type": {
            "option": "u16"
          }
        },
        {
          "name": "baseTokenDecimalsOpt",
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
          "name": "bids",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "asks",
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
          "name": "asks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "bids",
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
            "defined": "OrderType"
          }
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
          "name": "asks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "bids",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "orderId",
          "type": "i128"
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
          "name": "asks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "bids",
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
          "name": "asks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "bids",
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
          "name": "asks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "bids",
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
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "asks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "bids",
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
            "name": "oracleConfig",
            "type": {
              "defined": "OracleConfig"
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
            "type": "i64"
          },
          {
            "name": "bankRateLastUpdated",
            "type": "i64"
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
            "name": "flashLoanVaultInitial",
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
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "bankNum",
            "type": "u64"
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
            "name": "beingLiquidated",
            "docs": [
              "This account cannot open new positions or borrow until `init_health >= 0`"
            ],
            "type": "u8"
          },
          {
            "name": "isBankrupt",
            "docs": [
              "This account cannot do anything except go through `resolve_bankruptcy`"
            ],
            "type": "u8"
          },
          {
            "name": "accountNum",
            "type": "u8"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "netDeposits",
            "type": "f32"
          },
          {
            "name": "netSettled",
            "type": "f32"
          },
          {
            "name": "padding1",
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
            "name": "padding2",
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
            "name": "padding3",
            "type": "u32"
          },
          {
            "name": "perps",
            "type": {
              "vec": {
                "defined": "PerpPositions"
              }
            }
          },
          {
            "name": "padding4",
            "type": "u32"
          },
          {
            "name": "perpOpenOrders",
            "type": {
              "vec": {
                "defined": "PerpOpenOrders"
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
            "name": "padding",
            "type": {
              "array": [
                "u8",
                6
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
            "name": "addressLookupTable",
            "type": "publicKey"
          },
          {
            "name": "addressLookupTableBankIndex",
            "type": "u8"
          },
          {
            "name": "addressLookupTableOracleIndex",
            "type": "u8"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                6
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
                8
              ]
            }
          }
        ]
      }
    },
    {
      "name": "bookSide",
      "docs": [
        "A binary tree on AnyNode::key()",
        "",
        "The key encodes the price in the top 64 bits."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "bookSideType",
            "type": {
              "defined": "BookSideType"
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
                512
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
            "name": "baseTokenIndex",
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
            "name": "padding",
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
            "name": "bids",
            "type": "publicKey"
          },
          {
            "name": "asks",
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
            "type": "i64"
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
            "name": "baseTokenDecimals",
            "type": "u8"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                6
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
            "name": "padding",
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
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                5
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
      "name": "FlashLoanWithdraw",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "index",
            "docs": [
              "Account index of the vault to withdraw from in the target_accounts section.",
              "Index is counted after health accounts."
            ],
            "type": "u8"
          },
          {
            "name": "amount",
            "docs": [
              "Requested withdraw amount."
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "CpiData",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "accountStart",
            "type": "u8"
          },
          {
            "name": "data",
            "type": "bytes"
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
            "name": "oraclePrice",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "balance",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "serum3MaxReserved",
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
            "name": "reserved",
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
            "name": "base",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "quote",
            "type": {
              "defined": "I80F48"
            }
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
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                5
              ]
            }
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
            "name": "previousNativeCoinReserved",
            "type": "u64"
          },
          {
            "name": "previousNativePcReserved",
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
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          }
        ]
      }
    },
    {
      "name": "PerpPositions",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "marketIndex",
            "type": "u16"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                6
              ]
            }
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
          }
        ]
      }
    },
    {
      "name": "PerpOpenOrders",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "orderSide",
            "type": {
              "defined": "Side"
            }
          },
          {
            "name": "reserved1",
            "type": {
              "array": [
                "u8",
                1
              ]
            }
          },
          {
            "name": "orderMarket",
            "type": "u16"
          },
          {
            "name": "reserved2",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "clientOrderId",
            "type": "u64"
          },
          {
            "name": "orderId",
            "type": "i128"
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
                84
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
                199
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
      "name": "AccountSize",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Small"
          },
          {
            "name": "Large"
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
      "name": "BookSideType",
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
      "name": "OrderType",
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
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "marketIndex",
          "type": "u64",
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
          "name": "price",
          "type": "i64",
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
        },
        {
          "name": "price",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "MarginTradeLog",
      "fields": [
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenIndexes",
          "type": {
            "vec": "u16"
          },
          "index": false
        },
        {
          "name": "preIndexedPositions",
          "type": {
            "vec": "i128"
          },
          "index": false
        },
        {
          "name": "postIndexedPositions",
          "type": {
            "vec": "i128"
          },
          "index": false
        }
      ]
    },
    {
      "name": "FlashLoanLog",
      "fields": [
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
        }
      ]
    },
    {
      "name": "WithdrawLog",
      "fields": [
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
          "type": "i128",
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
          "type": "i128",
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
      "name": "UpdateFundingLog",
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
      "name": "LiquidateTokenAndTokenLog",
      "fields": [
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
        }
      ]
    },
    {
      "name": "OpenOrdersBalanceLog",
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
        },
        {
          "name": "price",
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
      "name": "MathError",
      "msg": "checked math error"
    },
    {
      "code": 6002,
      "name": "UnexpectedOracle",
      "msg": ""
    },
    {
      "code": 6003,
      "name": "UnknownOracleType",
      "msg": "oracle type cannot be determined"
    },
    {
      "code": 6004,
      "name": "InvalidFlashLoanTargetCpiProgram",
      "msg": ""
    },
    {
      "code": 6005,
      "name": "HealthMustBePositive",
      "msg": "health must be positive"
    },
    {
      "code": 6006,
      "name": "HealthMustBeNegative",
      "msg": "health must be negative"
    },
    {
      "code": 6007,
      "name": "IsBankrupt",
      "msg": "the account is bankrupt"
    },
    {
      "code": 6008,
      "name": "IsNotBankrupt",
      "msg": "the account is not bankrupt"
    },
    {
      "code": 6009,
      "name": "NoFreeTokenPositionIndex",
      "msg": "no free token position index"
    },
    {
      "code": 6010,
      "name": "NoFreeSerum3OpenOrdersIndex",
      "msg": "no free serum3 open orders index"
    },
    {
      "code": 6011,
      "name": "NoFreePerpPositionIndex",
      "msg": "no free perp position index"
    },
    {
      "code": 6012,
      "name": "Serum3OpenOrdersExistAlready",
      "msg": "serum3 open orders exist already"
    },
    {
      "code": 6013,
      "name": "InsufficentBankVaultFunds",
      "msg": "bank vault has insufficent funds"
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "InsuranceVault"
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
          "name": "newAdmin",
          "type": "publicKey"
        },
        {
          "name": "newFastListingAdmin",
          "type": "publicKey"
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "Bank"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "arg",
                "type": "u64",
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "Vault"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "arg",
                "type": "u64",
                "path": "bank_num"
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "MintInfo"
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
          "name": "bankNum",
          "type": "u64"
        },
        {
          "name": "name",
          "type": "string"
        },
        {
          "name": "oracleConfig",
          "type": {
            "defined": "OracleConfig"
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
        }
      ],
      "args": [
        {
          "name": "bankNum",
          "type": "u64"
        },
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
              "defined": "OracleConfig"
            }
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "Bank"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "arg",
                "type": "u64",
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "Vault"
              },
              {
                "kind": "arg",
                "type": "u16",
                "path": "token_index"
              },
              {
                "kind": "arg",
                "type": "u64",
                "path": "bank_num"
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "MintInfo"
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
          "type": "u64"
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
      "args": [
        {
          "name": "tokenIndex",
          "type": "u16"
        }
      ]
    },
    {
      "name": "tokenUpdateIndexAndRate",
      "accounts": [
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "MangoAccount"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "owner"
              },
              {
                "kind": "arg",
                "type": "u8",
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
          "type": "u8"
        },
        {
          "name": "accountSize",
          "type": {
            "defined": "AccountSize"
          }
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
      "args": []
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "StubOracle"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "account": "Mint",
                "path": "token_mint"
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
          "name": "tokenMint",
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
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
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
      "name": "flashLoan",
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
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "withdraws",
          "type": {
            "vec": {
              "defined": "FlashLoanWithdraw"
            }
          }
        },
        {
          "name": "cpiDatas",
          "type": {
            "vec": {
              "defined": "CpiData"
            }
          }
        }
      ]
    },
    {
      "name": "flashLoan2Begin",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "temporaryVaultAuthority",
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
          "isSigner": false
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
      "name": "flashLoan2End",
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
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "flashLoan3Begin",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
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
      "name": "flashLoan3End",
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "Serum3Market"
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
                "kind": "account",
                "type": "publicKey",
                "path": "account"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "Serum3OO"
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
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "const",
                "type": "string",
                "value": "PerpMarket"
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
          "name": "bids",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Accounts are initialised by client,",
            "anchor discriminator is set first when ix exits,"
          ]
        },
        {
          "name": "asks",
          "isMut": true,
          "isSigner": false
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
            "defined": "OracleConfig"
          }
        },
        {
          "name": "baseTokenIndexOpt",
          "type": {
            "option": "u16"
          }
        },
        {
          "name": "baseTokenDecimals",
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
              "defined": "OracleConfig"
            }
          }
        },
        {
          "name": "baseTokenIndexOpt",
          "type": {
            "option": "u16"
          }
        },
        {
          "name": "baseTokenDecimalsOpt",
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
          "name": "bids",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "asks",
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
          "name": "asks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "bids",
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
            "defined": "OrderType"
          }
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
          "name": "asks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "bids",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "orderId",
          "type": "i128"
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
          "name": "asks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "bids",
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
          "name": "asks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "bids",
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
          "name": "asks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "bids",
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
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "asks",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "bids",
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
            "name": "oracleConfig",
            "type": {
              "defined": "OracleConfig"
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
            "type": "i64"
          },
          {
            "name": "bankRateLastUpdated",
            "type": "i64"
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
            "name": "flashLoanVaultInitial",
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
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "bankNum",
            "type": "u64"
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
            "name": "beingLiquidated",
            "docs": [
              "This account cannot open new positions or borrow until `init_health >= 0`"
            ],
            "type": "u8"
          },
          {
            "name": "isBankrupt",
            "docs": [
              "This account cannot do anything except go through `resolve_bankruptcy`"
            ],
            "type": "u8"
          },
          {
            "name": "accountNum",
            "type": "u8"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "netDeposits",
            "type": "f32"
          },
          {
            "name": "netSettled",
            "type": "f32"
          },
          {
            "name": "padding1",
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
            "name": "padding2",
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
            "name": "padding3",
            "type": "u32"
          },
          {
            "name": "perps",
            "type": {
              "vec": {
                "defined": "PerpPositions"
              }
            }
          },
          {
            "name": "padding4",
            "type": "u32"
          },
          {
            "name": "perpOpenOrders",
            "type": {
              "vec": {
                "defined": "PerpOpenOrders"
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
            "name": "padding",
            "type": {
              "array": [
                "u8",
                6
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
            "name": "addressLookupTable",
            "type": "publicKey"
          },
          {
            "name": "addressLookupTableBankIndex",
            "type": "u8"
          },
          {
            "name": "addressLookupTableOracleIndex",
            "type": "u8"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                6
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
                8
              ]
            }
          }
        ]
      }
    },
    {
      "name": "bookSide",
      "docs": [
        "A binary tree on AnyNode::key()",
        "",
        "The key encodes the price in the top 64 bits."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "bookSideType",
            "type": {
              "defined": "BookSideType"
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
                512
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
            "name": "baseTokenIndex",
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
            "name": "padding",
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
            "name": "bids",
            "type": "publicKey"
          },
          {
            "name": "asks",
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
            "type": "i64"
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
            "name": "baseTokenDecimals",
            "type": "u8"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                6
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
            "name": "padding",
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
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                5
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
      "name": "FlashLoanWithdraw",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "index",
            "docs": [
              "Account index of the vault to withdraw from in the target_accounts section.",
              "Index is counted after health accounts."
            ],
            "type": "u8"
          },
          {
            "name": "amount",
            "docs": [
              "Requested withdraw amount."
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "CpiData",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "accountStart",
            "type": "u8"
          },
          {
            "name": "data",
            "type": "bytes"
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
            "name": "oraclePrice",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "balance",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "serum3MaxReserved",
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
            "name": "reserved",
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
            "name": "base",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "quote",
            "type": {
              "defined": "I80F48"
            }
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
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                5
              ]
            }
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
            "name": "previousNativeCoinReserved",
            "type": "u64"
          },
          {
            "name": "previousNativePcReserved",
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
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          }
        ]
      }
    },
    {
      "name": "PerpPositions",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "marketIndex",
            "type": "u16"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                6
              ]
            }
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
          }
        ]
      }
    },
    {
      "name": "PerpOpenOrders",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "orderSide",
            "type": {
              "defined": "Side"
            }
          },
          {
            "name": "reserved1",
            "type": {
              "array": [
                "u8",
                1
              ]
            }
          },
          {
            "name": "orderMarket",
            "type": "u16"
          },
          {
            "name": "reserved2",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "clientOrderId",
            "type": "u64"
          },
          {
            "name": "orderId",
            "type": "i128"
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
                84
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
                199
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
      "name": "AccountSize",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Small"
          },
          {
            "name": "Large"
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
      "name": "BookSideType",
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
      "name": "OrderType",
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
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "marketIndex",
          "type": "u64",
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
          "name": "price",
          "type": "i64",
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
        },
        {
          "name": "price",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "MarginTradeLog",
      "fields": [
        {
          "name": "mangoAccount",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenIndexes",
          "type": {
            "vec": "u16"
          },
          "index": false
        },
        {
          "name": "preIndexedPositions",
          "type": {
            "vec": "i128"
          },
          "index": false
        },
        {
          "name": "postIndexedPositions",
          "type": {
            "vec": "i128"
          },
          "index": false
        }
      ]
    },
    {
      "name": "FlashLoanLog",
      "fields": [
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
        }
      ]
    },
    {
      "name": "WithdrawLog",
      "fields": [
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
          "type": "i128",
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
          "type": "i128",
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
      "name": "UpdateFundingLog",
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
      "name": "LiquidateTokenAndTokenLog",
      "fields": [
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
        }
      ]
    },
    {
      "name": "OpenOrdersBalanceLog",
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
        },
        {
          "name": "price",
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
      "name": "MathError",
      "msg": "checked math error"
    },
    {
      "code": 6002,
      "name": "UnexpectedOracle",
      "msg": ""
    },
    {
      "code": 6003,
      "name": "UnknownOracleType",
      "msg": "oracle type cannot be determined"
    },
    {
      "code": 6004,
      "name": "InvalidFlashLoanTargetCpiProgram",
      "msg": ""
    },
    {
      "code": 6005,
      "name": "HealthMustBePositive",
      "msg": "health must be positive"
    },
    {
      "code": 6006,
      "name": "HealthMustBeNegative",
      "msg": "health must be negative"
    },
    {
      "code": 6007,
      "name": "IsBankrupt",
      "msg": "the account is bankrupt"
    },
    {
      "code": 6008,
      "name": "IsNotBankrupt",
      "msg": "the account is not bankrupt"
    },
    {
      "code": 6009,
      "name": "NoFreeTokenPositionIndex",
      "msg": "no free token position index"
    },
    {
      "code": 6010,
      "name": "NoFreeSerum3OpenOrdersIndex",
      "msg": "no free serum3 open orders index"
    },
    {
      "code": 6011,
      "name": "NoFreePerpPositionIndex",
      "msg": "no free perp position index"
    },
    {
      "code": 6012,
      "name": "Serum3OpenOrdersExistAlready",
      "msg": "serum3 open orders exist already"
    },
    {
      "code": 6013,
      "name": "InsufficentBankVaultFunds",
      "msg": "bank vault has insufficent funds"
    }
  ]
};
