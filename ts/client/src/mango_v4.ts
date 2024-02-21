export type MangoV4 = {
  "version": "0.22.0",
  "name": "mango_v4",
  "instructions": [
    {
      "name": "adminTokenWithdrawFees",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "vault"
          ]
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
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": []
    },
    {
      "name": "adminPerpWithdrawFees",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "vault"
          ]
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
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": []
    },
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
          "isSigner": false,
          "relations": [
            "admin"
          ]
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
          "name": "securityAdminOpt",
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
        },
        {
          "name": "depositLimitQuoteOpt",
          "type": {
            "option": "u64"
          }
        },
        {
          "name": "buybackFeesOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "buybackFeesBonusFactorOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "buybackFeesSwapMangoAccountOpt",
          "type": {
            "option": "publicKey"
          }
        },
        {
          "name": "mngoTokenIndexOpt",
          "type": {
            "option": "u16"
          }
        },
        {
          "name": "buybackFeesExpiryIntervalOpt",
          "type": {
            "option": "u64"
          }
        },
        {
          "name": "allowedFastListingsPerIntervalOpt",
          "type": {
            "option": "u16"
          }
        },
        {
          "name": "collateralFeeIntervalOpt",
          "type": {
            "option": "u64"
          }
        }
      ]
    },
    {
      "name": "groupWithdrawInsuranceFund",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "insurance_vault",
            "admin"
          ]
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
          "name": "destination",
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
        }
      ]
    },
    {
      "name": "ixGateSet",
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
          "name": "ixGate",
          "type": "u128"
        }
      ]
    },
    {
      "name": "groupClose",
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "admin",
            "insurance_vault"
          ]
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
          "isSigner": false,
          "relations": [
            "admin"
          ]
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
          "name": "fallbackOracle",
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
          "name": "stablePriceDelayIntervalSeconds",
          "type": "u32"
        },
        {
          "name": "stablePriceDelayGrowthLimit",
          "type": "f32"
        },
        {
          "name": "stablePriceGrowthLimit",
          "type": "f32"
        },
        {
          "name": "minVaultToDepositsRatio",
          "type": "f64"
        },
        {
          "name": "netBorrowLimitWindowSizeTs",
          "type": "u64"
        },
        {
          "name": "netBorrowLimitPerWindowQuote",
          "type": "i64"
        },
        {
          "name": "borrowWeightScaleStartQuote",
          "type": "f64"
        },
        {
          "name": "depositWeightScaleStartQuote",
          "type": "f64"
        },
        {
          "name": "reduceOnly",
          "type": "u8"
        },
        {
          "name": "tokenConditionalSwapTakerFeeRate",
          "type": "f32"
        },
        {
          "name": "tokenConditionalSwapMakerFeeRate",
          "type": "f32"
        },
        {
          "name": "flashLoanSwapFeeRate",
          "type": "f32"
        },
        {
          "name": "interestCurveScaling",
          "type": "f32"
        },
        {
          "name": "interestTargetUtilization",
          "type": "f32"
        },
        {
          "name": "groupInsuranceFund",
          "type": "bool"
        },
        {
          "name": "depositLimit",
          "type": "u64"
        },
        {
          "name": "zeroUtilRate",
          "type": "f32"
        },
        {
          "name": "platformLiquidationFee",
          "type": "f32"
        },
        {
          "name": "disableAssetLiquidation",
          "type": "bool"
        },
        {
          "name": "collateralFeePerDay",
          "type": "f32"
        }
      ]
    },
    {
      "name": "tokenRegisterTrustless",
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
          "name": "fallbackOracle",
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The oracle account is optional and only used when reset_stable_price is set.",
            ""
          ]
        },
        {
          "name": "fallbackOracle",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The fallback oracle account is optional and only used when set_fallback_oracle is true.",
            ""
          ]
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
          "name": "netBorrowLimitPerWindowQuoteOpt",
          "type": {
            "option": "i64"
          }
        },
        {
          "name": "netBorrowLimitWindowSizeTsOpt",
          "type": {
            "option": "u64"
          }
        },
        {
          "name": "borrowWeightScaleStartQuoteOpt",
          "type": {
            "option": "f64"
          }
        },
        {
          "name": "depositWeightScaleStartQuoteOpt",
          "type": {
            "option": "f64"
          }
        },
        {
          "name": "resetStablePrice",
          "type": "bool"
        },
        {
          "name": "resetNetBorrowLimit",
          "type": "bool"
        },
        {
          "name": "reduceOnlyOpt",
          "type": {
            "option": "u8"
          }
        },
        {
          "name": "nameOpt",
          "type": {
            "option": "string"
          }
        },
        {
          "name": "forceCloseOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "tokenConditionalSwapTakerFeeRateOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "tokenConditionalSwapMakerFeeRateOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "flashLoanSwapFeeRateOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "interestCurveScalingOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "interestTargetUtilizationOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "maintWeightShiftStartOpt",
          "type": {
            "option": "u64"
          }
        },
        {
          "name": "maintWeightShiftEndOpt",
          "type": {
            "option": "u64"
          }
        },
        {
          "name": "maintWeightShiftAssetTargetOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "maintWeightShiftLiabTargetOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "maintWeightShiftAbort",
          "type": "bool"
        },
        {
          "name": "setFallbackOracle",
          "type": "bool"
        },
        {
          "name": "depositLimitOpt",
          "type": {
            "option": "u64"
          }
        },
        {
          "name": "zeroUtilRateOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "platformLiquidationFeeOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "disableAssetLiquidationOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "collateralFeePerDayOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "forceWithdrawOpt",
          "type": {
            "option": "bool"
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
          "isSigner": false,
          "relations": [
            "admin"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "mint"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "mint"
          ]
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
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "mintInfo",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "oracle",
            "group"
          ]
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
      "name": "accountCreateV2",
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
          "name": "tokenConditionalSwapCount",
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
          "isSigner": false,
          "relations": [
            "group",
            "owner"
          ]
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
      "name": "accountExpandV2",
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
          "relations": [
            "group",
            "owner"
          ]
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
        },
        {
          "name": "tokenConditionalSwapCount",
          "type": "u8"
        }
      ]
    },
    {
      "name": "accountSizeMigration",
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
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "owner"
          ]
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
        },
        {
          "name": "temporaryDelegateOpt",
          "type": {
            "option": "publicKey"
          }
        },
        {
          "name": "temporaryDelegateExpiryOpt",
          "type": {
            "option": "u64"
          }
        }
      ]
    },
    {
      "name": "accountToggleFreeze",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "freeze",
          "type": "bool"
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
          "isSigner": false,
          "relations": [
            "group",
            "owner"
          ]
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
      "args": [
        {
          "name": "forceClose",
          "type": "bool"
        }
      ]
    },
    {
      "name": "accountBuybackFeesWithMngo",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "daoAccount",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "mngoBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "mngoOracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "feesBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "feesOracle",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "maxBuybackUsd",
          "type": "u64"
        }
      ]
    },
    {
      "name": "stubOracleCreate",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "oracle",
          "isMut": true,
          "isSigner": true
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
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "oracle",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "oracle",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "stubOracleSetTest",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "oracle",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        }
      ],
      "args": [
        {
          "name": "price",
          "type": {
            "defined": "I80F48"
          }
        },
        {
          "name": "lastUpdateSlot",
          "type": "u64"
        },
        {
          "name": "deviation",
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "vault",
            "oracle"
          ]
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
        },
        {
          "name": "reduceOnly",
          "type": "bool"
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "vault",
            "oracle"
          ]
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
        },
        {
          "name": "reduceOnly",
          "type": "bool"
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "vault",
            "oracle"
          ]
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
      "name": "flashLoanSwapBegin",
      "docs": [
        "A version of flash_loan_begin that's specialized for swaps and needs fewer",
        "bytes in the transaction"
      ],
      "accounts": [
        {
          "name": "account",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "inputMint",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "outputMint",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "associatedTokenProgram",
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
          "name": "loanAmount",
          "type": "u64"
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
      "name": "flashLoanEndV2",
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
          "name": "numLoans",
          "type": "u8"
        },
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
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": true,
          "docs": [
            "group admin or fast listing admin, checked at #1"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "baseBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
        },
        {
          "name": "oraclePriceBand",
          "type": "f32"
        }
      ]
    },
    {
      "name": "serum3EditMarket",
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
          "name": "market",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        }
      ],
      "args": [
        {
          "name": "reduceOnlyOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "forceCloseOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "nameOpt",
          "type": {
            "option": "string"
          }
        },
        {
          "name": "oraclePriceBandOpt",
          "type": {
            "option": "f32"
          }
        }
      ]
    },
    {
      "name": "serum3DeregisterMarket",
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "serumMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "indexReservation",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "serum_program",
            "serum_market_external"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "serum_program",
            "serum_market_external"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "serum_program",
            "serum_market_external"
          ]
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
          ],
          "relations": [
            "group"
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
          "name": "payerOracle",
          "isMut": false,
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
      "name": "serum3PlaceOrderV2",
      "docs": [
        "requires the receiver_bank in the health account list to be writable"
      ],
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
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "serum_program",
            "serum_market_external"
          ]
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
          ],
          "relations": [
            "group"
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
          "name": "payerOracle",
          "isMut": false,
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
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "serum_program",
            "serum_market_external"
          ]
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
      "name": "serum3CancelOrderByClientOrderId",
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
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "serum_program",
            "serum_market_external"
          ]
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
          "name": "clientOrderId",
          "type": "u64"
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
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "serum_program",
            "serum_market_external"
          ]
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
      "docs": [
        "Deprecated instruction that used to settles all free funds from the OpenOrders account",
        "into the MangoAccount.",
        "",
        "Any serum \"referrer rebates\" (ui fees) are considered Mango fees."
      ],
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
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "serum_program",
            "serum_market_external"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "quoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "baseBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "serum3SettleFundsV2",
      "docs": [
        "Like Serum3SettleFunds, but `fees_to_dao` determines if referrer rebates are considered fees",
        "or are credited to the MangoAccount."
      ],
      "accounts": [
        {
          "name": "v1",
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
              "relations": [
                "group"
              ]
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
              "isSigner": false,
              "relations": [
                "group",
                "serum_program",
                "serum_market_external"
              ]
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
              "isSigner": false,
              "relations": [
                "group"
              ]
            },
            {
              "name": "quoteVault",
              "isMut": true,
              "isSigner": false
            },
            {
              "name": "baseBank",
              "isMut": true,
              "isSigner": false,
              "relations": [
                "group"
              ]
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
          ]
        },
        {
          "name": "v2",
          "accounts": [
            {
              "name": "quoteOracle",
              "isMut": false,
              "isSigner": false
            },
            {
              "name": "baseOracle",
              "isMut": false,
              "isSigner": false
            }
          ]
        }
      ],
      "args": [
        {
          "name": "feesToDao",
          "type": "bool"
        }
      ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "serum_program",
            "serum_market_external"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "quoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "baseBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "insurance_vault"
          ]
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liabMintInfo",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "tokenForceCloseBorrowsWithToken",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "type": "u64"
        }
      ]
    },
    {
      "name": "tokenLiqBankruptcy",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "insurance_vault"
          ]
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liabMintInfo",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "tokenForceWithdraw",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "vault",
            "oracle"
          ]
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
          "name": "ownerAtaTokenAccount",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "alternateOwnerTokenAccount",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Only for the unusual case where the owner_ata account is not owned by account.owner"
          ]
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
          "isSigner": false,
          "relations": [
            "admin"
          ]
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
          "name": "maintBaseAssetWeight",
          "type": "f32"
        },
        {
          "name": "initBaseAssetWeight",
          "type": "f32"
        },
        {
          "name": "maintBaseLiabWeight",
          "type": "f32"
        },
        {
          "name": "initBaseLiabWeight",
          "type": "f32"
        },
        {
          "name": "maintOverallAssetWeight",
          "type": "f32"
        },
        {
          "name": "initOverallAssetWeight",
          "type": "f32"
        },
        {
          "name": "baseLiquidationFee",
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
          "name": "settlePnlLimitWindowSizeTs",
          "type": "u64"
        },
        {
          "name": "positivePnlLiquidationFee",
          "type": "f32"
        },
        {
          "name": "platformLiquidationFee",
          "type": "f32"
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The oracle account is optional and only used when reset_stable_price is set.",
            ""
          ]
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
          "name": "maintBaseAssetWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "initBaseAssetWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "maintBaseLiabWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "initBaseLiabWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "maintOverallAssetWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "initOverallAssetWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "baseLiquidationFeeOpt",
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
          "name": "settlePnlLimitWindowSizeTsOpt",
          "type": {
            "option": "u64"
          }
        },
        {
          "name": "reduceOnlyOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "resetStablePrice",
          "type": "bool"
        },
        {
          "name": "positivePnlLiquidationFeeOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "nameOpt",
          "type": {
            "option": "string"
          }
        },
        {
          "name": "forceCloseOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "platformLiquidationFeeOpt",
          "type": {
            "option": "f32"
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
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks",
            "event_queue"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks",
            "event_queue",
            "oracle"
          ]
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
      ],
      "returns": {
        "option": "u128"
      }
    },
    {
      "name": "perpPlaceOrderV2",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks",
            "event_queue",
            "oracle"
          ]
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
          "name": "selfTradeBehavior",
          "type": {
            "defined": "SelfTradeBehavior"
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
      ],
      "returns": {
        "option": "u128"
      }
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks",
            "event_queue",
            "oracle"
          ]
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
        },
        {
          "name": "maxOracleStalenessSlots",
          "type": "i32"
        }
      ],
      "returns": {
        "option": "u128"
      }
    },
    {
      "name": "perpPlaceOrderPeggedV2",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks",
            "event_queue",
            "oracle"
          ]
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
          "name": "selfTradeBehavior",
          "type": {
            "defined": "SelfTradeBehavior"
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
        },
        {
          "name": "maxOracleStalenessSlots",
          "type": "i32"
        }
      ],
      "returns": {
        "option": "u128"
      }
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "event_queue"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks",
            "oracle"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "settlerOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "oracle"
          ]
        },
        {
          "name": "accountA",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "accountB",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "settleBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "perpForceClosePosition",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "oracle"
          ]
        },
        {
          "name": "accountA",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "accountB",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "oracle"
          ]
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "settleBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "perpLiqBaseOrPositivePnl",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "oracle"
          ]
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "settleBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
        }
      ],
      "args": [
        {
          "name": "maxBaseTransfer",
          "type": "i64"
        },
        {
          "name": "maxPnlTransfer",
          "type": "u64"
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks"
          ]
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
      "name": "perpLiqNegativePnlOrBankruptcy",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "insurance_vault"
          ]
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "oracle"
          ]
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "settleBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "perpLiqNegativePnlOrBankruptcyV2",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "insurance_vault"
          ]
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "oracle"
          ]
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "settleBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "name": "insuranceBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "insuranceBankVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "insuranceOracle",
          "isMut": false,
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
      "name": "tokenConditionalSwapCreate",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "buyBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "sellBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
        }
      ],
      "args": [
        {
          "name": "maxBuy",
          "type": "u64"
        },
        {
          "name": "maxSell",
          "type": "u64"
        },
        {
          "name": "expiryTimestamp",
          "type": "u64"
        },
        {
          "name": "priceLowerLimit",
          "type": "f64"
        },
        {
          "name": "priceUpperLimit",
          "type": "f64"
        },
        {
          "name": "pricePremiumRate",
          "type": "f64"
        },
        {
          "name": "allowCreatingDeposits",
          "type": "bool"
        },
        {
          "name": "allowCreatingBorrows",
          "type": "bool"
        }
      ]
    },
    {
      "name": "tokenConditionalSwapCreateV2",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "buyBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "sellBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
        }
      ],
      "args": [
        {
          "name": "maxBuy",
          "type": "u64"
        },
        {
          "name": "maxSell",
          "type": "u64"
        },
        {
          "name": "expiryTimestamp",
          "type": "u64"
        },
        {
          "name": "priceLowerLimit",
          "type": "f64"
        },
        {
          "name": "priceUpperLimit",
          "type": "f64"
        },
        {
          "name": "pricePremiumRate",
          "type": "f64"
        },
        {
          "name": "allowCreatingDeposits",
          "type": "bool"
        },
        {
          "name": "allowCreatingBorrows",
          "type": "bool"
        },
        {
          "name": "displayPriceStyle",
          "type": {
            "defined": "TokenConditionalSwapDisplayPriceStyle"
          }
        },
        {
          "name": "intention",
          "type": {
            "defined": "TokenConditionalSwapIntention"
          }
        }
      ]
    },
    {
      "name": "tokenConditionalSwapCreatePremiumAuction",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "buyBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "sellBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
        }
      ],
      "args": [
        {
          "name": "maxBuy",
          "type": "u64"
        },
        {
          "name": "maxSell",
          "type": "u64"
        },
        {
          "name": "expiryTimestamp",
          "type": "u64"
        },
        {
          "name": "priceLowerLimit",
          "type": "f64"
        },
        {
          "name": "priceUpperLimit",
          "type": "f64"
        },
        {
          "name": "maxPricePremiumRate",
          "type": "f64"
        },
        {
          "name": "allowCreatingDeposits",
          "type": "bool"
        },
        {
          "name": "allowCreatingBorrows",
          "type": "bool"
        },
        {
          "name": "displayPriceStyle",
          "type": {
            "defined": "TokenConditionalSwapDisplayPriceStyle"
          }
        },
        {
          "name": "intention",
          "type": {
            "defined": "TokenConditionalSwapIntention"
          }
        },
        {
          "name": "durationSeconds",
          "type": "u64"
        }
      ]
    },
    {
      "name": "tokenConditionalSwapCreateLinearAuction",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "buyBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "sellBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
        }
      ],
      "args": [
        {
          "name": "maxBuy",
          "type": "u64"
        },
        {
          "name": "maxSell",
          "type": "u64"
        },
        {
          "name": "expiryTimestamp",
          "type": "u64"
        },
        {
          "name": "priceStart",
          "type": "f64"
        },
        {
          "name": "priceEnd",
          "type": "f64"
        },
        {
          "name": "allowCreatingDeposits",
          "type": "bool"
        },
        {
          "name": "allowCreatingBorrows",
          "type": "bool"
        },
        {
          "name": "displayPriceStyle",
          "type": {
            "defined": "TokenConditionalSwapDisplayPriceStyle"
          }
        },
        {
          "name": "startTimestamp",
          "type": "u64"
        },
        {
          "name": "durationSeconds",
          "type": "u64"
        }
      ]
    },
    {
      "name": "tokenConditionalSwapCancel",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "buyBank",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The bank's token_index is checked at #1"
          ],
          "relations": [
            "group"
          ]
        },
        {
          "name": "sellBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        }
      ],
      "args": [
        {
          "name": "tokenConditionalSwapIndex",
          "type": "u8"
        },
        {
          "name": "tokenConditionalSwapId",
          "type": "u64"
        }
      ]
    },
    {
      "name": "tokenConditionalSwapTrigger",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorAuthority",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "tokenConditionalSwapIndex",
          "type": "u8"
        },
        {
          "name": "tokenConditionalSwapId",
          "type": "u64"
        },
        {
          "name": "maxBuyTokenToLiqee",
          "type": "u64"
        },
        {
          "name": "maxSellTokenToLiqor",
          "type": "u64"
        }
      ]
    },
    {
      "name": "tokenConditionalSwapTriggerV2",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorAuthority",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "tokenConditionalSwapIndex",
          "type": "u8"
        },
        {
          "name": "tokenConditionalSwapId",
          "type": "u64"
        },
        {
          "name": "maxBuyTokenToLiqee",
          "type": "u64"
        },
        {
          "name": "maxSellTokenToLiqor",
          "type": "u64"
        },
        {
          "name": "minBuyToken",
          "type": "u64"
        },
        {
          "name": "minTakerPrice",
          "type": "f32"
        }
      ]
    },
    {
      "name": "tokenConditionalSwapStart",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorAuthority",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "tokenConditionalSwapIndex",
          "type": "u8"
        },
        {
          "name": "tokenConditionalSwapId",
          "type": "u64"
        }
      ]
    },
    {
      "name": "tokenChargeCollateralFees",
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
          "relations": [
            "group"
          ]
        }
      ],
      "args": []
    },
    {
      "name": "altSet",
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "admin"
          ]
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
          "isSigner": false,
          "relations": [
            "admin"
          ]
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
      "docs": [
        "Warning, this instruction is for testing purposes only!"
      ],
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
        }
      ],
      "args": []
    },
    {
      "name": "openbookV2RegisterMarket",
      "docs": [
        "",
        "OpenbookV2",
        ""
      ],
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openbookV2Program",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2MarketExternal",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2Market",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "OpenbookV2Market"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "openbook_v2_market_external"
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
                "value": "OpenbookV2Index"
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "baseBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "openbookV2EditMarket",
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
          "name": "market",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        }
      ],
      "args": [
        {
          "name": "reduceOnlyOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "forceCloseOpt",
          "type": {
            "option": "bool"
          }
        }
      ]
    },
    {
      "name": "openbookV2DeregisterMarket",
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openbookV2Market",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "indexReservation",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "openbookV2CreateOpenOrders",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openbookV2Market",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "openbook_v2_program",
            "openbook_v2_market_external"
          ]
        },
        {
          "name": "openbookV2Program",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2MarketExternal",
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
                "value": "OpenOrders"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "openbook_v2_market"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "openbook_v2_market_external"
              },
              {
                "kind": "arg",
                "type": "u32",
                "path": "account_num"
              }
            ],
            "programId": {
              "kind": "account",
              "type": "publicKey",
              "path": "openbook_v2_program"
            }
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
      "args": [
        {
          "name": "accountNum",
          "type": "u32"
        }
      ]
    },
    {
      "name": "openbookV2CloseOpenOrders",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openbookV2Market",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "openbook_v2_program",
            "openbook_v2_market_external"
          ]
        },
        {
          "name": "openbookV2Program",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2MarketExternal",
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
      "name": "openbookV2PlaceOrder",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "openbookV2Market",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2Program",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2MarketExternal",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "bids",
            "asks",
            "event_heap"
          ]
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
          "name": "eventHeap",
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
          "name": "payerBank",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The bank that pays for the order, if necessary"
          ],
          "relations": [
            "group"
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
          "name": "payerOracle",
          "isMut": false,
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
          "type": "u8"
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
          "type": "u8"
        },
        {
          "name": "orderType",
          "type": "u8"
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
      "name": "openbookV2PlaceTakerOrder",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openbookV2Market",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "openbook_v2_program",
            "openbook_v2_market_external"
          ]
        },
        {
          "name": "openbookV2Program",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2MarketExternal",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "bids",
            "asks",
            "event_heap"
          ]
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
          "name": "eventHeap",
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
          "isSigner": false
        },
        {
          "name": "payerBank",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The bank that pays for the order, if necessary"
          ],
          "relations": [
            "group"
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
          "name": "payerOracle",
          "isMut": false,
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
          "type": "u8"
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
          "type": "u8"
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
      "name": "openbookV2CancelOrder",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "openbookV2Market",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "openbook_v2_program",
            "openbook_v2_market_external"
          ]
        },
        {
          "name": "openbookV2Program",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2MarketExternal",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "bids",
            "asks"
          ]
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
        }
      ],
      "args": [
        {
          "name": "side",
          "type": "u8"
        },
        {
          "name": "orderId",
          "type": "u128"
        }
      ]
    },
    {
      "name": "openbookV2SettleFunds",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "openbookV2Market",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "openbook_v2_program",
            "openbook_v2_market_external"
          ]
        },
        {
          "name": "openbookV2Program",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2MarketExternal",
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "quoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "baseBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "baseVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "quoteOracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "baseOracle",
          "isMut": false,
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
          "name": "feesToDao",
          "type": "bool"
        }
      ]
    },
    {
      "name": "openbookV2LiqForceCancelOrders",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "openbookV2Market",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "openbook_v2_program",
            "openbook_v2_market_external"
          ]
        },
        {
          "name": "openbookV2Program",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2MarketExternal",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "bids",
            "asks",
            "event_heap"
          ]
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
          "name": "eventHeap",
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "quoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "baseBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "openbookV2CancelAllOrders",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "openbookV2Market",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "openbook_v2_program",
            "openbook_v2_market_external"
          ]
        },
        {
          "name": "openbookV2Program",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2MarketExternal",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "bids",
            "asks"
          ]
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
      "name": "benchmark",
      "docs": [
        "",
        "benchmark",
        ""
      ],
      "accounts": [
        {
          "name": "dummy",
          "isMut": false,
          "isSigner": false
        }
      ],
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
            "name": "stablePriceModel",
            "type": {
              "defined": "StablePriceModel"
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
            "docs": [
              "The unscaled borrow interest curve is defined as continuous piecewise linear with the points:",
              "",
              "- 0% util: zero_util_rate",
              "- util0% util: rate0",
              "- util1% util: rate1",
              "- 100% util: max_rate",
              "",
              "The final rate is this unscaled curve multiplied by interest_curve_scaling."
            ],
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
            "docs": [
              "the 100% utilization rate",
              "",
              "This isn't the max_rate, since this still gets scaled by interest_curve_scaling,",
              "which is >=1."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "collectedFeesNative",
            "docs": [
              "Fees collected over the lifetime of the bank",
              "",
              "See fees_withdrawn for how much of the fees was withdrawn.",
              "See collected_liquidation_fees for the (included) subtotal for liquidation related fees."
            ],
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
            "docs": [
              "Liquidation fee that goes to the liqor.",
              "",
              "Liquidation always involves two tokens, and the sum of the two configured fees is used.",
              "",
              "A fraction of the price, like 0.05 for a 5% fee during liquidation.",
              "",
              "See also platform_liquidation_fee."
            ],
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
            "name": "minVaultToDepositsRatio",
            "docs": [
              "The maximum utilization allowed when borrowing is 1-this value",
              "WARNING: Outdated name, kept for IDL compatibility"
            ],
            "type": "f64"
          },
          {
            "name": "netBorrowLimitWindowSizeTs",
            "docs": [
              "Size in seconds of a net borrows window"
            ],
            "type": "u64"
          },
          {
            "name": "lastNetBorrowsWindowStartTs",
            "docs": [
              "Timestamp at which the last net borrows window started"
            ],
            "type": "u64"
          },
          {
            "name": "netBorrowLimitPerWindowQuote",
            "docs": [
              "Net borrow limit per window in quote native; set to -1 to disable."
            ],
            "type": "i64"
          },
          {
            "name": "netBorrowsInWindow",
            "docs": [
              "Sum of all deposits and borrows in the last window, in native units."
            ],
            "type": "i64"
          },
          {
            "name": "borrowWeightScaleStartQuote",
            "docs": [
              "Soft borrow limit in native quote",
              "",
              "Once the borrows on the bank exceed this quote value, init_liab_weight is scaled up.",
              "Set to f64::MAX to disable.",
              "",
              "See scaled_init_liab_weight()."
            ],
            "type": "f64"
          },
          {
            "name": "depositWeightScaleStartQuote",
            "docs": [
              "Limit for collateral of deposits in native quote",
              "",
              "Once the deposits in the bank exceed this quote value, init_asset_weight is scaled",
              "down to keep the total collateral value constant.",
              "Set to f64::MAX to disable.",
              "",
              "See scaled_init_asset_weight()."
            ],
            "type": "f64"
          },
          {
            "name": "reduceOnly",
            "type": "u8"
          },
          {
            "name": "forceClose",
            "type": "u8"
          },
          {
            "name": "disableAssetLiquidation",
            "docs": [
              "If set to 1, deposits cannot be liquidated when an account is liquidatable.",
              "That means bankrupt accounts may still have assets of this type deposited."
            ],
            "type": "u8"
          },
          {
            "name": "forceWithdraw",
            "type": "u8"
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
            "name": "feesWithdrawn",
            "type": "u64"
          },
          {
            "name": "tokenConditionalSwapTakerFeeRate",
            "docs": [
              "Fees for the token conditional swap feature"
            ],
            "type": "f32"
          },
          {
            "name": "tokenConditionalSwapMakerFeeRate",
            "type": "f32"
          },
          {
            "name": "flashLoanSwapFeeRate",
            "type": "f32"
          },
          {
            "name": "interestTargetUtilization",
            "docs": [
              "Target utilization: If actual utilization is higher, scale up interest.",
              "If it's lower, scale down interest (if possible)"
            ],
            "type": "f32"
          },
          {
            "name": "interestCurveScaling",
            "docs": [
              "Current interest curve scaling, always >= 1.0",
              "",
              "Except when first migrating to having this field, then 0.0"
            ],
            "type": "f64"
          },
          {
            "name": "potentialSerumTokens",
            "docs": [
              "Largest amount of tokens that might be added the the bank based on",
              "serum open order execution."
            ],
            "type": "u64"
          },
          {
            "name": "maintWeightShiftStart",
            "docs": [
              "Start timestamp in seconds at which maint weights should start to change away",
              "from maint_asset_weight, maint_liab_weight towards _asset_target and _liab_target.",
              "If _start and _end and _duration_inv are 0, no shift is configured."
            ],
            "type": "u64"
          },
          {
            "name": "maintWeightShiftEnd",
            "docs": [
              "End timestamp in seconds until which the maint weights should reach the configured targets."
            ],
            "type": "u64"
          },
          {
            "name": "maintWeightShiftDurationInv",
            "docs": [
              "Cache of the inverse of maint_weight_shift_end - maint_weight_shift_start,",
              "or zero if no shift is configured"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maintWeightShiftAssetTarget",
            "docs": [
              "Maint asset weight to reach at _shift_end."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maintWeightShiftLiabTarget",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "fallbackOracle",
            "docs": [
              "Oracle that may be used if the main oracle is stale or not confident enough.",
              "If this is Pubkey::default(), no fallback is available."
            ],
            "type": "publicKey"
          },
          {
            "name": "depositLimit",
            "docs": [
              "zero means none, in token native"
            ],
            "type": "u64"
          },
          {
            "name": "zeroUtilRate",
            "docs": [
              "The unscaled borrow interest curve point for zero utilization.",
              "",
              "See util0, rate0, util1, rate1, max_rate"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "platformLiquidationFee",
            "docs": [
              "Additional to liquidation_fee, but goes to the group owner instead of the liqor"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "collectedLiquidationFees",
            "docs": [
              "Platform fees that were collected during liquidation (in native tokens)",
              "",
              "See also collected_fees_native and fees_withdrawn."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "collectedCollateralFees",
            "docs": [
              "Collateral fees that have been collected (in native tokens)",
              "",
              "See also collected_fees_native and fees_withdrawn."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "collateralFeePerDay",
            "docs": [
              "The daily collateral fees rate for fully utilized collateral."
            ],
            "type": "f32"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                1900
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
            "name": "mngoTokenIndex",
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
            "name": "buybackFees",
            "type": "u8"
          },
          {
            "name": "buybackFeesMngoBonusFactor",
            "type": "f32"
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
            "name": "securityAdmin",
            "type": "publicKey"
          },
          {
            "name": "depositLimitQuote",
            "type": "u64"
          },
          {
            "name": "ixGate",
            "type": "u128"
          },
          {
            "name": "buybackFeesSwapMangoAccount",
            "type": "publicKey"
          },
          {
            "name": "buybackFeesExpiryInterval",
            "docs": [
              "Number of seconds after which fees that could be used with the fees buyback feature expire.",
              "",
              "The actual expiry is staggered such that the fees users accumulate are always",
              "available for at least this interval - but may be available for up to twice this time.",
              "",
              "When set to 0, there's no expiry of buyback fees."
            ],
            "type": "u64"
          },
          {
            "name": "fastListingIntervalStart",
            "docs": [
              "Fast-listings are limited per week, this is the start of the current fast-listing interval",
              "in seconds since epoch"
            ],
            "type": "u64"
          },
          {
            "name": "fastListingsInInterval",
            "docs": [
              "Number of fast listings that happened this interval"
            ],
            "type": "u16"
          },
          {
            "name": "allowedFastListingsPerInterval",
            "docs": [
              "Number of fast listings that are allowed per interval"
            ],
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
            "name": "collateralFeeInterval",
            "docs": [
              "Intervals in which collateral fee is applied"
            ],
            "type": "u64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                1800
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
            "name": "frozenUntil",
            "type": "u64"
          },
          {
            "name": "buybackFeesAccruedCurrent",
            "docs": [
              "Fees usable with the \"fees buyback\" feature.",
              "This tracks the ones that accrued in the current expiry interval."
            ],
            "type": "u64"
          },
          {
            "name": "buybackFeesAccruedPrevious",
            "docs": [
              "Fees buyback amount from the previous expiry interval."
            ],
            "type": "u64"
          },
          {
            "name": "buybackFeesExpiryTimestamp",
            "docs": [
              "End timestamp of the current expiry interval of the buyback fees amount."
            ],
            "type": "u64"
          },
          {
            "name": "nextTokenConditionalSwapId",
            "docs": [
              "Next id to use when adding a token condition swap"
            ],
            "type": "u64"
          },
          {
            "name": "temporaryDelegate",
            "type": "publicKey"
          },
          {
            "name": "temporaryDelegateExpiry",
            "type": "u64"
          },
          {
            "name": "lastCollateralFeeCharge",
            "docs": [
              "Time at which the last collateral fee was charged"
            ],
            "type": "u64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                152
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
          },
          {
            "name": "padding8",
            "type": "u32"
          },
          {
            "name": "tokenConditionalSwaps",
            "type": {
              "vec": {
                "defined": "TokenConditionalSwap"
              }
            }
          },
          {
            "name": "reservedDynamic",
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
            "name": "fallbackOracle",
            "type": "publicKey"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                2528
              ]
            }
          }
        ]
      }
    },
    {
      "name": "openbookV2Market",
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
            "name": "reduceOnly",
            "type": "u8"
          },
          {
            "name": "forceClose",
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
            "name": "openbookV2Program",
            "type": "publicKey"
          },
          {
            "name": "openbookV2MarketExternal",
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
                512
              ]
            }
          }
        ]
      }
    },
    {
      "name": "openbookV2MarketIndexReservation",
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
            "name": "lastUpdateTs",
            "type": "i64"
          },
          {
            "name": "lastUpdateSlot",
            "type": "u64"
          },
          {
            "name": "deviation",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                104
              ]
            }
          }
        ]
      }
    },
    {
      "name": "bookSide",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "roots",
            "type": {
              "array": [
                {
                  "defined": "OrderTreeRoot"
                },
                2
              ]
            }
          },
          {
            "name": "reservedRoots",
            "type": {
              "array": [
                {
                  "defined": "OrderTreeRoot"
                },
                4
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
          },
          {
            "name": "nodes",
            "type": {
              "defined": "OrderTreeNodes"
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
            "docs": [
              "Token index that settlements happen in.",
              "",
              "Currently required to be 0, USDC. In the future settlement",
              "may be allowed to happen in other tokens."
            ],
            "type": "u16"
          },
          {
            "name": "perpMarketIndex",
            "docs": [
              "Index of this perp market. Other data, like the MangoAccount's PerpPosition",
              "reference this market via this index. Unique for this group's perp markets."
            ],
            "type": "u16"
          },
          {
            "name": "blocked1",
            "docs": [
              "Field used to contain the trusted_market flag and is now unused."
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
            "name": "bump",
            "docs": [
              "PDA bump"
            ],
            "type": "u8"
          },
          {
            "name": "baseDecimals",
            "docs": [
              "Number of decimals used for the base token.",
              "",
              "Used to convert the oracle's price into a native/native price."
            ],
            "type": "u8"
          },
          {
            "name": "name",
            "docs": [
              "Name. Trailing zero bytes are ignored."
            ],
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          },
          {
            "name": "bids",
            "docs": [
              "Address of the BookSide account for bids"
            ],
            "type": "publicKey"
          },
          {
            "name": "asks",
            "docs": [
              "Address of the BookSide account for asks"
            ],
            "type": "publicKey"
          },
          {
            "name": "eventQueue",
            "docs": [
              "Address of the EventQueue account"
            ],
            "type": "publicKey"
          },
          {
            "name": "oracle",
            "docs": [
              "Oracle account address"
            ],
            "type": "publicKey"
          },
          {
            "name": "oracleConfig",
            "docs": [
              "Oracle configuration"
            ],
            "type": {
              "defined": "OracleConfig"
            }
          },
          {
            "name": "stablePriceModel",
            "docs": [
              "Maintains a stable price based on the oracle price that is less volatile."
            ],
            "type": {
              "defined": "StablePriceModel"
            }
          },
          {
            "name": "quoteLotSize",
            "docs": [
              "Number of quote native in a quote lot. Must be a power of 10.",
              "",
              "Primarily useful for increasing the tick size on the market: A lot price",
              "of 1 becomes a native price of quote_lot_size/base_lot_size becomes a",
              "ui price of quote_lot_size*base_decimals/base_lot_size/quote_decimals."
            ],
            "type": "i64"
          },
          {
            "name": "baseLotSize",
            "docs": [
              "Number of base native in a base lot. Must be a power of 10.",
              "",
              "Example: If base decimals for the underlying asset is 6, base lot size",
              "is 100 and and base position lots is 10_000 then base position native is",
              "1_000_000 and base position ui is 1."
            ],
            "type": "i64"
          },
          {
            "name": "maintBaseAssetWeight",
            "docs": [
              "These weights apply to the base position. The quote position has",
              "no explicit weight (but may be covered by the overall pnl asset weight)."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initBaseAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maintBaseLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initBaseLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "openInterest",
            "docs": [
              "Number of base lots currently active in the market. Always >= 0.",
              "",
              "Since this counts positive base lots and negative base lots, the more relevant",
              "number of open base lot pairs is half this value."
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
            "name": "registrationTime",
            "docs": [
              "Timestamp in seconds that the market was registered at."
            ],
            "type": "u64"
          },
          {
            "name": "minFunding",
            "docs": [
              "Minimal funding rate per day, must be <= 0."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maxFunding",
            "docs": [
              "Maximal funding rate per day, must be >= 0."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "impactQuantity",
            "docs": [
              "For funding, get the impact price this many base lots deep into the book."
            ],
            "type": "i64"
          },
          {
            "name": "longFunding",
            "docs": [
              "Current long funding value. Increasing it means that every long base lot",
              "needs to pay that amount of quote native in funding.",
              "",
              "PerpPosition uses and tracks it settle funding. Updated by the perp",
              "keeper instruction."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "shortFunding",
            "docs": [
              "See long_funding."
            ],
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
            "name": "baseLiquidationFee",
            "docs": [
              "Fees",
              "Fee for base position liquidation"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "makerFee",
            "docs": [
              "Fee when matching maker orders. May be negative."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "takerFee",
            "docs": [
              "Fee for taker orders, may not be negative."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "feesAccrued",
            "docs": [
              "Fees accrued in native quote currency",
              "these are increased when new fees are paid and decreased when perp_settle_fees is called"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "feesSettled",
            "docs": [
              "Fees settled in native quote currency",
              "these are increased when perp_settle_fees is called, and never decreased"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "feePenalty",
            "docs": [
              "Fee (in quote native) to charge for ioc orders"
            ],
            "type": "f32"
          },
          {
            "name": "settleFeeFlat",
            "docs": [
              "In native units of settlement token, given to each settle call above the",
              "settle_fee_amount_threshold if settling at least 1% of perp base pos value."
            ],
            "type": "f32"
          },
          {
            "name": "settleFeeAmountThreshold",
            "docs": [
              "Pnl settlement amount needed to be eligible for the flat fee."
            ],
            "type": "f32"
          },
          {
            "name": "settleFeeFractionLowHealth",
            "docs": [
              "Fraction of pnl to pay out as fee if +pnl account has low health.",
              "(limited to 2x settle_fee_flat)"
            ],
            "type": "f32"
          },
          {
            "name": "settlePnlLimitFactor",
            "docs": [
              "Controls the strictness of the settle limit.",
              "Set to a negative value to disable the limit.",
              "",
              "This factor applies to the settle limit in two ways",
              "- for the unrealized pnl settle limit, the factor is multiplied with the stable perp base value",
              "(i.e. limit_factor * base_native * stable_price)",
              "- when increasing the realized pnl settle limit (stored per PerpPosition), the factor is",
              "multiplied with the stable value of the perp pnl being realized",
              "(i.e. limit_factor * reduced_native * stable_price)",
              "",
              "See also PerpPosition::settle_pnl_limit_realized_trade"
            ],
            "type": "f32"
          },
          {
            "name": "padding3",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "settlePnlLimitWindowSizeTs",
            "docs": [
              "Window size in seconds for the perp settlement limit"
            ],
            "type": "u64"
          },
          {
            "name": "reduceOnly",
            "docs": [
              "If true, users may no longer increase their market exposure. Only actions",
              "that reduce their position are still allowed."
            ],
            "type": "u8"
          },
          {
            "name": "forceClose",
            "type": "u8"
          },
          {
            "name": "padding4",
            "type": {
              "array": [
                "u8",
                6
              ]
            }
          },
          {
            "name": "maintOverallAssetWeight",
            "docs": [
              "Weights for full perp market health, if positive"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initOverallAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "positivePnlLiquidationFee",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "feesWithdrawn",
            "type": "u64"
          },
          {
            "name": "platformLiquidationFee",
            "docs": [
              "Additional to liquidation_fee, but goes to the group owner instead of the liqor"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "accruedLiquidationFees",
            "docs": [
              "Platform fees that were accrued during liquidation (in native tokens)",
              "",
              "These fees are also added to fees_accrued, this is just for bookkeeping the total",
              "liquidation fees that happened. So never decreases (different to fees_accrued)."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                1848
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
            "name": "reduceOnly",
            "type": "u8"
          },
          {
            "name": "forceClose",
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
                1
              ]
            }
          },
          {
            "name": "oraclePriceBand",
            "docs": [
              "Limit orders must be <= oracle * (1+band) and >= oracle / (1+band)",
              "",
              "Zero value is the default due to migration and disables the limit,",
              "same as f32::MAX."
            ],
            "type": "f32"
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
      "name": "FlashLoanTokenDetailV2",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tokenIndex",
            "type": "u16"
          },
          {
            "name": "changeAmount",
            "docs": [
              "The amount by which the user's token position changed at the end",
              "",
              "So if the user repaid the approved_amount in full, it'd be 0.",
              "",
              "Does NOT include the loan_origination_fee or deposit_fee, so the true",
              "change is `change_amount - loan_origination_fee - deposit_fee`."
            ],
            "type": "i128"
          },
          {
            "name": "loan",
            "docs": [
              "The amount that was a loan (<= approved_amount, depends on user's deposits)"
            ],
            "type": "i128"
          },
          {
            "name": "loanOriginationFee",
            "docs": [
              "The fee paid on the loan, not included in `loan` or `change_amount`"
            ],
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
          },
          {
            "name": "depositFee",
            "docs": [
              "Deposit fee paid for positive change_amount.",
              "",
              "Not factored into change_amount."
            ],
            "type": "i128"
          },
          {
            "name": "approvedAmount",
            "docs": [
              "The amount that was transfered out to the user"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "FlashLoanTokenDetailV3",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tokenIndex",
            "type": "u16"
          },
          {
            "name": "changeAmount",
            "docs": [
              "The amount by which the user's token position changed at the end",
              "",
              "So if the user repaid the approved_amount in full, it'd be 0.",
              "",
              "Does NOT include the loan_origination_fee or deposit_fee, so the true",
              "change is `change_amount - loan_origination_fee - deposit_fee`."
            ],
            "type": "i128"
          },
          {
            "name": "loan",
            "docs": [
              "The amount that was a loan (<= approved_amount, depends on user's deposits)"
            ],
            "type": "i128"
          },
          {
            "name": "loanOriginationFee",
            "docs": [
              "The fee paid on the loan, not included in `loan` or `change_amount`"
            ],
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
          },
          {
            "name": "swapFee",
            "docs": [
              "Swap fee paid on the in token of a swap.",
              "",
              "Not factored into change_amount."
            ],
            "type": "i128"
          },
          {
            "name": "approvedAmount",
            "docs": [
              "The amount that was transfered out to the user"
            ],
            "type": "u64"
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
            "name": "highestPlacedBidInv",
            "docs": [
              "Track something like the highest open bid / lowest open ask, in native/native units.",
              "",
              "Tracking it exactly isn't possible since we don't see fills. So instead track",
              "the min/max of the _placed_ bids and asks.",
              "",
              "The value is reset in serum3_place_order when a new order is placed without an",
              "existing one on the book.",
              "",
              "0 is a special \"unset\" state."
            ],
            "type": "f64"
          },
          {
            "name": "lowestPlacedAsk",
            "type": "f64"
          },
          {
            "name": "potentialBaseTokens",
            "docs": [
              "An overestimate of the amount of tokens that might flow out of the open orders account.",
              "",
              "The bank still considers these amounts user deposits (see Bank::potential_serum_tokens)",
              "and that value needs to be updated in conjunction with these numbers.",
              "",
              "This estimation is based on the amount of tokens in the open orders account",
              "(see update_bank_potential_tokens() in serum3_place_order and settle)"
            ],
            "type": "u64"
          },
          {
            "name": "potentialQuoteTokens",
            "type": "u64"
          },
          {
            "name": "lowestPlacedBidInv",
            "docs": [
              "Track lowest bid/highest ask, same way as for highest bid/lowest ask.",
              "",
              "0 is a special \"unset\" state."
            ],
            "type": "f64"
          },
          {
            "name": "highestPlacedAsk",
            "type": "f64"
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
            "docs": [
              "Index of the current settle pnl limit window"
            ],
            "type": "u32"
          },
          {
            "name": "settlePnlLimitSettledInCurrentWindowNative",
            "docs": [
              "Amount of realized trade pnl and unrealized pnl that was already settled this window.",
              "",
              "Will be negative when negative pnl was settled.",
              "",
              "Note that this will be adjusted for bookkeeping reasons when the realized_trade settle",
              "limitchanges and is not useable for actually tracking how much pnl was settled",
              "on balance."
            ],
            "type": "i64"
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
              "Active position in oracle quote native. At the same time this is 1:1 a settle_token native amount.",
              "",
              "Example: Say there's a perp market on the BTC/USD price using SOL for settlement. The user buys",
              "one long contract for $20k, then base = 1, quote = -20k. The price goes to $21k. Now their",
              "unsettled pnl is (1 * 21k - 20k) __SOL__ = 1000 SOL. This is because the perp contract arbitrarily",
              "decides that each unit of price difference creates 1 SOL worth of settlement.",
              "(yes, causing 1 SOL of settlement for each $1 price change implies a lot of extra leverage; likely",
              "there should be an extra configurable scaling factor before we use this for cases like that)"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "quoteRunningNative",
            "docs": [
              "Tracks what the position is to calculate average entry & break even price"
            ],
            "type": "i64"
          },
          {
            "name": "longSettledFunding",
            "docs": [
              "Already settled long funding"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "shortSettledFunding",
            "docs": [
              "Already settled short funding"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "bidsBaseLots",
            "docs": [
              "Base lots in open bids"
            ],
            "type": "i64"
          },
          {
            "name": "asksBaseLots",
            "docs": [
              "Base lots in open asks"
            ],
            "type": "i64"
          },
          {
            "name": "takerBaseLots",
            "docs": [
              "Amount of base lots on the EventQueue waiting to be processed"
            ],
            "type": "i64"
          },
          {
            "name": "takerQuoteLots",
            "docs": [
              "Amount of quote lots on the EventQueue waiting to be processed"
            ],
            "type": "i64"
          },
          {
            "name": "cumulativeLongFunding",
            "docs": [
              "Cumulative long funding in quote native units.",
              "If the user paid $1 in funding for a long position, this would be 1e6.",
              "Beware of the sign!",
              "",
              "(Display only)"
            ],
            "type": "f64"
          },
          {
            "name": "cumulativeShortFunding",
            "docs": [
              "Cumulative short funding in quote native units",
              "If the user paid $1 in funding for a short position, this would be -1e6.",
              "",
              "(Display only)"
            ],
            "type": "f64"
          },
          {
            "name": "makerVolume",
            "docs": [
              "Cumulative maker volume in quote native units",
              "",
              "(Display only)"
            ],
            "type": "u64"
          },
          {
            "name": "takerVolume",
            "docs": [
              "Cumulative taker volume in quote native units",
              "",
              "(Display only)"
            ],
            "type": "u64"
          },
          {
            "name": "perpSpotTransfers",
            "docs": [
              "Cumulative number of quote native units transfered from the perp position",
              "to the settle token spot position.",
              "",
              "For example, if the user settled $1 of positive pnl into their USDC spot",
              "position, this would be 1e6.",
              "",
              "(Display only)"
            ],
            "type": "i64"
          },
          {
            "name": "avgEntryPricePerBaseLot",
            "docs": [
              "The native average entry price for the base lots of the current position.",
              "Reset to 0 when the base position reaches or crosses 0."
            ],
            "type": "f64"
          },
          {
            "name": "deprecatedRealizedTradePnlNative",
            "docs": [
              "Deprecated field: Amount of pnl that was realized by bringing the base position closer to 0."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "oneshotSettlePnlAllowance",
            "docs": [
              "Amount of pnl that can be settled once.",
              "",
              "- The value is signed: a negative number means negative pnl can be settled.",
              "- A settlement in the right direction will decrease this amount.",
              "",
              "Typically added for fees, funding and liquidation."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "recurringSettlePnlAllowance",
            "docs": [
              "Amount of pnl that can be settled in each settle window.",
              "",
              "- Unsigned, the settlement can happen in both directions. Value is >= 0.",
              "- Previously stored a similar value that was signed, so in migration cases",
              "this value can be negative and should be .abs()ed.",
              "- If this value exceeds the current stable-upnl, it should be decreased,",
              "see apply_recurring_settle_pnl_allowance_constraint()",
              "",
              "When the base position is reduced, the settle limit contribution from the reduced",
              "base position is materialized into this value. When the base position increases,",
              "some of the allowance is taken away.",
              "",
              "This also gets increased when a liquidator takes over pnl."
            ],
            "type": "i64"
          },
          {
            "name": "realizedPnlForPositionNative",
            "docs": [
              "Trade pnl, fees, funding that were added over the current position's lifetime.",
              "",
              "Reset when the position changes sign or goes to zero.",
              "Not decreased by settling.",
              "",
              "This is tracked for display purposes: this value plus the difference between entry",
              "price and current price of the base position is the overall pnl."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                88
              ]
            }
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
            "type": "u8"
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
            "name": "quantity",
            "type": "i64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                56
              ]
            }
          }
        ]
      }
    },
    {
      "name": "MangoAccountFixed",
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
            "type": "u8"
          },
          {
            "name": "inHealthRegion",
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
            "type": "i64"
          },
          {
            "name": "frozenUntil",
            "type": "u64"
          },
          {
            "name": "buybackFeesAccruedCurrent",
            "type": "u64"
          },
          {
            "name": "buybackFeesAccruedPrevious",
            "type": "u64"
          },
          {
            "name": "buybackFeesExpiryTimestamp",
            "type": "u64"
          },
          {
            "name": "nextTokenConditionalSwapId",
            "type": "u64"
          },
          {
            "name": "temporaryDelegate",
            "type": "publicKey"
          },
          {
            "name": "temporaryDelegateExpiry",
            "type": "u64"
          },
          {
            "name": "lastCollateralFeeCharge",
            "type": "u64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                152
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
            "type": "u8"
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
                72
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
            "docs": [
              "NodeTag"
            ],
            "type": "u8"
          },
          {
            "name": "ownerSlot",
            "docs": [
              "Index into the owning MangoAccount's PerpOpenOrders"
            ],
            "type": "u8"
          },
          {
            "name": "orderType",
            "docs": [
              "PostOrderType, this was added for TradingView move order"
            ],
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
            "name": "timeInForce",
            "docs": [
              "Time in seconds after `timestamp` at which the order expires.",
              "A value of 0 means no expiry."
            ],
            "type": "u16"
          },
          {
            "name": "padding2",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "key",
            "docs": [
              "The binary tree key, see new_node_key()"
            ],
            "type": "u128"
          },
          {
            "name": "owner",
            "docs": [
              "Address of the owning MangoAccount"
            ],
            "type": "publicKey"
          },
          {
            "name": "quantity",
            "docs": [
              "Number of base lots to buy or sell, always >=1"
            ],
            "type": "i64"
          },
          {
            "name": "timestamp",
            "docs": [
              "The time the order was placed"
            ],
            "type": "u64"
          },
          {
            "name": "pegLimit",
            "docs": [
              "If the effective price of an oracle pegged order exceeds this limit,",
              "it will be considered invalid and may be removed.",
              "",
              "Only applicable in the oracle_pegged OrderTree"
            ],
            "type": "i64"
          },
          {
            "name": "clientOrderId",
            "docs": [
              "User defined id for this order, used in FillEvents"
            ],
            "type": "u64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                32
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
            "type": "u8"
          },
          {
            "name": "data",
            "type": {
              "array": [
                "u8",
                119
              ]
            }
          }
        ]
      }
    },
    {
      "name": "OrderTreeRoot",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "maybeNode",
            "type": "u32"
          },
          {
            "name": "leafCount",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "OrderTreeNodes",
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
            "type": "u8"
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
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                512
              ]
            }
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
            "type": "u8"
          },
          {
            "name": "makerOut",
            "type": "u8"
          },
          {
            "name": "makerSlot",
            "type": "u8"
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
            "name": "padding2",
            "type": {
              "array": [
                "u8",
                32
              ]
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
            "name": "padding3",
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          },
          {
            "name": "takerClientOrderId",
            "type": "u64"
          },
          {
            "name": "makerOrderId",
            "type": "u128"
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
            "name": "makerClientOrderId",
            "type": "u64"
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
            "type": "u8"
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
            "name": "orderId",
            "type": "u128"
          },
          {
            "name": "padding1",
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
            "name": "resetOnNonzeroPrice",
            "docs": [
              "If set to 1, the stable price will reset on the next non-zero price it sees."
            ],
            "type": "u8"
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
      "name": "TokenConditionalSwap",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "id",
            "type": "u64"
          },
          {
            "name": "maxBuy",
            "docs": [
              "maximum amount of native tokens to buy or sell"
            ],
            "type": "u64"
          },
          {
            "name": "maxSell",
            "type": "u64"
          },
          {
            "name": "bought",
            "docs": [
              "how many native tokens were already bought/sold"
            ],
            "type": "u64"
          },
          {
            "name": "sold",
            "type": "u64"
          },
          {
            "name": "expiryTimestamp",
            "docs": [
              "timestamp until which the conditional swap is valid"
            ],
            "type": "u64"
          },
          {
            "name": "priceLowerLimit",
            "docs": [
              "The lower or starting price:",
              "- For FixedPremium or PremiumAuctions, it's the lower end of the price range:",
              "the tcs can only be triggered if the oracle price exceeds this value.",
              "- For LinearAuctions it's the starting price that's offered at start_timestamp.",
              "",
              "The price is always in \"sell_token per buy_token\" units, which can be computed",
              "by dividing the buy token price by the sell token price.",
              "",
              "For FixedPremium or PremiumAuctions:",
              "",
              "The price must exceed this threshold to allow execution.",
              "",
              "This threshold is compared to the \"sell_token per buy_token\" oracle price.",
              "If that price is >= lower_limit and <= upper_limit the tcs may be executable.",
              "",
              "Example: Stop loss to get out of a SOL long: The user bought SOL at 20 USDC/SOL",
              "and wants to stop loss at 18 USDC/SOL. They'd set buy_token=USDC, sell_token=SOL",
              "so the reference price is in SOL/USDC units. Set price_lower_limit=toNative(1/18)",
              "and price_upper_limit=toNative(1/10). Also set allow_borrows=false.",
              "",
              "Example: Want to buy SOL with USDC if the price falls below 22 USDC/SOL.",
              "buy_token=SOL, sell_token=USDC, reference price is in USDC/SOL units. Set",
              "price_upper_limit=toNative(22), price_lower_limit=0."
            ],
            "type": "f64"
          },
          {
            "name": "priceUpperLimit",
            "docs": [
              "Parallel to price_lower_limit, but an upper limit / auction end price."
            ],
            "type": "f64"
          },
          {
            "name": "pricePremiumRate",
            "docs": [
              "The premium to pay over oracle price to incentivize execution."
            ],
            "type": "f64"
          },
          {
            "name": "takerFeeRate",
            "docs": [
              "The taker receives only premium_price * (1 - taker_fee_rate)"
            ],
            "type": "f32"
          },
          {
            "name": "makerFeeRate",
            "docs": [
              "The maker has to pay premium_price * (1 + maker_fee_rate)"
            ],
            "type": "f32"
          },
          {
            "name": "buyTokenIndex",
            "docs": [
              "indexes of tokens for the swap"
            ],
            "type": "u16"
          },
          {
            "name": "sellTokenIndex",
            "type": "u16"
          },
          {
            "name": "isConfigured",
            "docs": [
              "If this struct is in use. (tcs are stored in a static-length array)"
            ],
            "type": "u8"
          },
          {
            "name": "allowCreatingDeposits",
            "docs": [
              "may token purchases create deposits? (often users just want to get out of a borrow)"
            ],
            "type": "u8"
          },
          {
            "name": "allowCreatingBorrows",
            "docs": [
              "may token selling create borrows? (often users just want to get out of a long)"
            ],
            "type": "u8"
          },
          {
            "name": "displayPriceStyle",
            "docs": [
              "The stored prices are always \"sell token per buy token\", but if the user",
              "used \"buy token per sell token\" when creating the tcs order, we should continue",
              "to show them prices in that way.",
              "",
              "Stores a TokenConditionalSwapDisplayPriceStyle enum value"
            ],
            "type": "u8"
          },
          {
            "name": "intention",
            "docs": [
              "The intention the user had when placing this order, display-only",
              "",
              "Stores a TokenConditionalSwapIntention enum value"
            ],
            "type": "u8"
          },
          {
            "name": "tcsType",
            "docs": [
              "Stores a TokenConditionalSwapType enum value"
            ],
            "type": "u8"
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
            "name": "startTimestamp",
            "docs": [
              "In seconds since epoch. 0 means not-started.",
              "",
              "FixedPremium: Time of first trigger call. No other effect.",
              "PremiumAuction: Time of start or first trigger call. Can continue to trigger once started.",
              "LinearAuction: Set during creation, auction starts with price_lower_limit at this timestamp."
            ],
            "type": "u64"
          },
          {
            "name": "durationSeconds",
            "docs": [
              "Duration of the auction mechanism",
              "",
              "FixedPremium: ignored",
              "PremiumAuction: time after start that the premium needs to scale to price_premium_rate",
              "LinearAuction: time after start to go from price_lower_limit to price_upper_limit"
            ],
            "type": "u64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                88
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
          },
          {
            "name": "SwapWithoutFee"
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
        "There are three types of health:",
        "- initial health (\"init\"): users can only open new positions if it's >= 0",
        "- maintenance health (\"maint\"): users get liquidated if it's < 0",
        "- liquidation end health: once liquidation started (see being_liquidated), it",
        "only stops once this is >= 0",
        "",
        "The ordering is",
        "init health <= liquidation end health <= maint health",
        "",
        "The different health types are realized by using different weights and prices:",
        "- init health: init weights with scaling, stable-price adjusted prices",
        "- liq end health: init weights without scaling, oracle prices",
        "- maint health: maint weights, oracle prices",
        ""
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Init"
          },
          {
            "name": "Maint"
          },
          {
            "name": "LiquidationEnd"
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
          },
          {
            "name": "TokenConditionalSwapTrigger"
          }
        ]
      }
    },
    {
      "name": "IxGate",
      "docs": [
        "Enum for lookup into ix gate",
        "note:",
        "total ix files 56,",
        "ix files included 48,",
        "ix files not included 8,",
        "- Benchmark,",
        "- ComputeAccountData,",
        "- GroupCreate",
        "- GroupEdit",
        "- IxGateSet,",
        "- PerpZeroOut,",
        "- PerpEditMarket,",
        "- TokenEdit,"
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "AccountClose"
          },
          {
            "name": "AccountCreate"
          },
          {
            "name": "AccountEdit"
          },
          {
            "name": "AccountExpand"
          },
          {
            "name": "AccountToggleFreeze"
          },
          {
            "name": "AltExtend"
          },
          {
            "name": "AltSet"
          },
          {
            "name": "FlashLoan"
          },
          {
            "name": "GroupClose"
          },
          {
            "name": "GroupCreate"
          },
          {
            "name": "HealthRegion"
          },
          {
            "name": "PerpCancelAllOrders"
          },
          {
            "name": "PerpCancelAllOrdersBySide"
          },
          {
            "name": "PerpCancelOrder"
          },
          {
            "name": "PerpCancelOrderByClientOrderId"
          },
          {
            "name": "PerpCloseMarket"
          },
          {
            "name": "PerpConsumeEvents"
          },
          {
            "name": "PerpCreateMarket"
          },
          {
            "name": "PerpDeactivatePosition"
          },
          {
            "name": "PerpLiqBaseOrPositivePnl"
          },
          {
            "name": "PerpLiqForceCancelOrders"
          },
          {
            "name": "PerpLiqNegativePnlOrBankruptcy"
          },
          {
            "name": "PerpPlaceOrder"
          },
          {
            "name": "PerpSettleFees"
          },
          {
            "name": "PerpSettlePnl"
          },
          {
            "name": "PerpUpdateFunding"
          },
          {
            "name": "Serum3CancelAllOrders"
          },
          {
            "name": "Serum3CancelOrder"
          },
          {
            "name": "Serum3CloseOpenOrders"
          },
          {
            "name": "Serum3CreateOpenOrders"
          },
          {
            "name": "Serum3DeregisterMarket"
          },
          {
            "name": "Serum3EditMarket"
          },
          {
            "name": "Serum3LiqForceCancelOrders"
          },
          {
            "name": "Serum3PlaceOrder"
          },
          {
            "name": "Serum3RegisterMarket"
          },
          {
            "name": "Serum3SettleFunds"
          },
          {
            "name": "StubOracleClose"
          },
          {
            "name": "StubOracleCreate"
          },
          {
            "name": "StubOracleSet"
          },
          {
            "name": "TokenAddBank"
          },
          {
            "name": "TokenDeposit"
          },
          {
            "name": "TokenDeregister"
          },
          {
            "name": "TokenLiqBankruptcy"
          },
          {
            "name": "TokenLiqWithToken"
          },
          {
            "name": "TokenRegister"
          },
          {
            "name": "TokenRegisterTrustless"
          },
          {
            "name": "TokenUpdateIndexAndRate"
          },
          {
            "name": "TokenWithdraw"
          },
          {
            "name": "AccountBuybackFeesWithMngo"
          },
          {
            "name": "TokenForceCloseBorrowsWithToken"
          },
          {
            "name": "PerpForceClosePosition"
          },
          {
            "name": "GroupWithdrawInsuranceFund"
          },
          {
            "name": "TokenConditionalSwapCreate"
          },
          {
            "name": "TokenConditionalSwapTrigger"
          },
          {
            "name": "TokenConditionalSwapCancel"
          },
          {
            "name": "OpenbookV2CancelOrder"
          },
          {
            "name": "OpenbookV2CloseOpenOrders"
          },
          {
            "name": "OpenbookV2CreateOpenOrders"
          },
          {
            "name": "OpenbookV2DeregisterMarket"
          },
          {
            "name": "OpenbookV2EditMarket"
          },
          {
            "name": "OpenbookV2LiqForceCancelOrders"
          },
          {
            "name": "OpenbookV2PlaceOrder"
          },
          {
            "name": "OpenbookV2PlaceTakeOrder"
          },
          {
            "name": "OpenbookV2RegisterMarket"
          },
          {
            "name": "OpenbookV2SettleFunds"
          },
          {
            "name": "AdminTokenWithdrawFees"
          },
          {
            "name": "AdminPerpWithdrawFees"
          },
          {
            "name": "AccountSizeMigration"
          },
          {
            "name": "TokenConditionalSwapStart"
          },
          {
            "name": "TokenConditionalSwapCreatePremiumAuction"
          },
          {
            "name": "TokenConditionalSwapCreateLinearAuction"
          },
          {
            "name": "Serum3PlaceOrderV2"
          },
          {
            "name": "TokenForceWithdraw"
          }
        ]
      }
    },
    {
      "name": "CheckLiquidatable",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "NotLiquidatable"
          },
          {
            "name": "Liquidatable"
          },
          {
            "name": "BecameNotLiquidatable"
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
          },
          {
            "name": "OrcaCLMM"
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
      "name": "SelfTradeBehavior",
      "docs": [
        "Self trade behavior controls how taker orders interact with resting limit orders of the same account.",
        "This setting has no influence on placing a resting or oracle pegged limit order that does not match",
        "immediately, instead it's the responsibility of the user to correctly configure his taker orders."
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
              },
              {
                "name": "max_oracle_staleness_slots",
                "type": "i32"
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
    },
    {
      "name": "TokenConditionalSwapDisplayPriceStyle",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "SellTokenPerBuyToken"
          },
          {
            "name": "BuyTokenPerSellToken"
          }
        ]
      }
    },
    {
      "name": "TokenConditionalSwapIntention",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Unknown"
          },
          {
            "name": "StopLoss"
          },
          {
            "name": "TakeProfit"
          }
        ]
      }
    },
    {
      "name": "TokenConditionalSwapType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "FixedPremium"
          },
          {
            "name": "PremiumAuction"
          },
          {
            "name": "LinearAuction"
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
      "name": "FlashLoanLogV2",
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
              "defined": "FlashLoanTokenDetailV2"
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
      "name": "FlashLoanLogV3",
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
              "defined": "FlashLoanTokenDetailV3"
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
      "name": "FillLogV2",
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
          "name": "makerClientOrderId",
          "type": "u64",
          "index": false
        },
        {
          "name": "makerFee",
          "type": "f32",
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
          "name": "takerClientOrderId",
          "type": "u64",
          "index": false
        },
        {
          "name": "takerFee",
          "type": "f32",
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
      "name": "FillLogV3",
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
          "name": "makerClientOrderId",
          "type": "u64",
          "index": false
        },
        {
          "name": "makerFee",
          "type": "f32",
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
          "name": "takerClientOrderId",
          "type": "u64",
          "index": false
        },
        {
          "name": "takerFee",
          "type": "f32",
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
        },
        {
          "name": "makerClosedPnl",
          "type": "f64",
          "index": false
        },
        {
          "name": "takerClosedPnl",
          "type": "f64",
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
          "name": "oracleSlot",
          "type": "u64",
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
          "name": "feesSettled",
          "type": "i128",
          "index": false
        },
        {
          "name": "openInterest",
          "type": "i64",
          "index": false
        },
        {
          "name": "instantaneousFundingRate",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "PerpUpdateFundingLogV2",
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
          "name": "oracleSlot",
          "type": "u64",
          "index": false
        },
        {
          "name": "oracleConfidence",
          "type": "i128",
          "index": false
        },
        {
          "name": "oracleType",
          "type": {
            "defined": "OracleType"
          },
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
          "name": "feesSettled",
          "type": "i128",
          "index": false
        },
        {
          "name": "openInterest",
          "type": "i64",
          "index": false
        },
        {
          "name": "instantaneousFundingRate",
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
        },
        {
          "name": "borrowRate",
          "type": "i128",
          "index": false
        },
        {
          "name": "depositRate",
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
      "name": "UpdateRateLogV2",
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
          "name": "util0",
          "type": "i128",
          "index": false
        },
        {
          "name": "rate1",
          "type": "i128",
          "index": false
        },
        {
          "name": "util1",
          "type": "i128",
          "index": false
        },
        {
          "name": "maxRate",
          "type": "i128",
          "index": false
        },
        {
          "name": "curveScaling",
          "type": "f64",
          "index": false
        },
        {
          "name": "targetUtilization",
          "type": "f32",
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
      "name": "TokenLiqWithTokenLogV2",
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
          "name": "assetTransferFromLiqee",
          "type": "i128",
          "index": false
        },
        {
          "name": "assetTransferToLiqor",
          "type": "i128",
          "index": false
        },
        {
          "name": "assetLiquidationFee",
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
      "name": "Serum3OpenOrdersBalanceLogV2",
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
      "name": "WithdrawLoanLog",
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
          "name": "loanAmount",
          "type": "i128",
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
        },
        {
          "name": "price",
          "type": {
            "option": "i128"
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
        },
        {
          "name": "startingLiabDepositIndex",
          "type": "i128",
          "index": false
        },
        {
          "name": "endingLiabDepositIndex",
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
      "name": "TokenMetaDataLogV2",
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
          "name": "fallbackOracle",
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
      "name": "PerpLiqBaseOrPositivePnlLog",
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
          "name": "pnlTransfer",
          "type": "i128",
          "index": false
        },
        {
          "name": "pnlSettleLimitTransfer",
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
      "name": "PerpLiqBaseOrPositivePnlLogV2",
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
          "name": "baseTransferLiqee",
          "type": "i64",
          "index": false
        },
        {
          "name": "quoteTransferLiqee",
          "type": "i128",
          "index": false
        },
        {
          "name": "quoteTransferLiqor",
          "type": "i128",
          "index": false
        },
        {
          "name": "quotePlatformFee",
          "type": "i128",
          "index": false
        },
        {
          "name": "pnlTransfer",
          "type": "i128",
          "index": false
        },
        {
          "name": "pnlSettleLimitTransfer",
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
        },
        {
          "name": "startingLongFunding",
          "type": "i128",
          "index": false
        },
        {
          "name": "startingShortFunding",
          "type": "i128",
          "index": false
        },
        {
          "name": "endingLongFunding",
          "type": "i128",
          "index": false
        },
        {
          "name": "endingShortFunding",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "PerpLiqNegativePnlOrBankruptcyLog",
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
          "name": "settlement",
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
    },
    {
      "name": "AccountBuybackFeesWithMngoLog",
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
          "name": "buybackFees",
          "type": "i128",
          "index": false
        },
        {
          "name": "buybackMngo",
          "type": "i128",
          "index": false
        },
        {
          "name": "mngoBuybackPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "oraclePrice",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "FilledPerpOrderLog",
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
          "name": "seqNum",
          "type": "u64",
          "index": false
        }
      ]
    },
    {
      "name": "PerpTakerTradeLog",
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
          "name": "takerSide",
          "type": "u8",
          "index": false
        },
        {
          "name": "totalBaseLotsTaken",
          "type": "i64",
          "index": false
        },
        {
          "name": "totalBaseLotsDecremented",
          "type": "i64",
          "index": false
        },
        {
          "name": "totalQuoteLotsTaken",
          "type": "i64",
          "index": false
        },
        {
          "name": "totalQuoteLotsDecremented",
          "type": "i64",
          "index": false
        },
        {
          "name": "takerFeesPaid",
          "type": "i128",
          "index": false
        },
        {
          "name": "feePenalty",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "PerpForceClosePositionLog",
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
          "name": "accountA",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "accountB",
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
      "name": "TokenForceCloseBorrowsWithTokenLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
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
          "name": "feeFactor",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "TokenForceCloseBorrowsWithTokenLogV2",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
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
          "name": "assetTransferFromLiqee",
          "type": "i128",
          "index": false
        },
        {
          "name": "assetTransferToLiqor",
          "type": "i128",
          "index": false
        },
        {
          "name": "assetLiquidationFee",
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
          "name": "feeFactor",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "TokenConditionalSwapCreateLog",
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
          "name": "id",
          "type": "u64",
          "index": false
        },
        {
          "name": "maxBuy",
          "type": "u64",
          "index": false
        },
        {
          "name": "maxSell",
          "type": "u64",
          "index": false
        },
        {
          "name": "expiryTimestamp",
          "type": "u64",
          "index": false
        },
        {
          "name": "priceLowerLimit",
          "type": "f64",
          "index": false
        },
        {
          "name": "priceUpperLimit",
          "type": "f64",
          "index": false
        },
        {
          "name": "pricePremiumRate",
          "type": "f64",
          "index": false
        },
        {
          "name": "takerFeeRate",
          "type": "f32",
          "index": false
        },
        {
          "name": "makerFeeRate",
          "type": "f32",
          "index": false
        },
        {
          "name": "buyTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "sellTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "allowCreatingDeposits",
          "type": "bool",
          "index": false
        },
        {
          "name": "allowCreatingBorrows",
          "type": "bool",
          "index": false
        }
      ]
    },
    {
      "name": "TokenConditionalSwapCreateLogV2",
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
          "name": "id",
          "type": "u64",
          "index": false
        },
        {
          "name": "maxBuy",
          "type": "u64",
          "index": false
        },
        {
          "name": "maxSell",
          "type": "u64",
          "index": false
        },
        {
          "name": "expiryTimestamp",
          "type": "u64",
          "index": false
        },
        {
          "name": "priceLowerLimit",
          "type": "f64",
          "index": false
        },
        {
          "name": "priceUpperLimit",
          "type": "f64",
          "index": false
        },
        {
          "name": "pricePremiumRate",
          "type": "f64",
          "index": false
        },
        {
          "name": "takerFeeRate",
          "type": "f32",
          "index": false
        },
        {
          "name": "makerFeeRate",
          "type": "f32",
          "index": false
        },
        {
          "name": "buyTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "sellTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "allowCreatingDeposits",
          "type": "bool",
          "index": false
        },
        {
          "name": "allowCreatingBorrows",
          "type": "bool",
          "index": false
        },
        {
          "name": "displayPriceStyle",
          "type": "u8",
          "index": false
        },
        {
          "name": "intention",
          "type": "u8",
          "index": false
        }
      ]
    },
    {
      "name": "TokenConditionalSwapCreateLogV3",
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
          "name": "id",
          "type": "u64",
          "index": false
        },
        {
          "name": "maxBuy",
          "type": "u64",
          "index": false
        },
        {
          "name": "maxSell",
          "type": "u64",
          "index": false
        },
        {
          "name": "expiryTimestamp",
          "type": "u64",
          "index": false
        },
        {
          "name": "priceLowerLimit",
          "type": "f64",
          "index": false
        },
        {
          "name": "priceUpperLimit",
          "type": "f64",
          "index": false
        },
        {
          "name": "pricePremiumRate",
          "type": "f64",
          "index": false
        },
        {
          "name": "takerFeeRate",
          "type": "f32",
          "index": false
        },
        {
          "name": "makerFeeRate",
          "type": "f32",
          "index": false
        },
        {
          "name": "buyTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "sellTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "allowCreatingDeposits",
          "type": "bool",
          "index": false
        },
        {
          "name": "allowCreatingBorrows",
          "type": "bool",
          "index": false
        },
        {
          "name": "displayPriceStyle",
          "type": "u8",
          "index": false
        },
        {
          "name": "intention",
          "type": "u8",
          "index": false
        },
        {
          "name": "tcsType",
          "type": "u8",
          "index": false
        },
        {
          "name": "startTimestamp",
          "type": "u64",
          "index": false
        },
        {
          "name": "durationSeconds",
          "type": "u64",
          "index": false
        }
      ]
    },
    {
      "name": "TokenConditionalSwapTriggerLog",
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
          "name": "tokenConditionalSwapId",
          "type": "u64",
          "index": false
        },
        {
          "name": "buyTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "sellTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "buyAmount",
          "type": "u64",
          "index": false
        },
        {
          "name": "sellAmount",
          "type": "u64",
          "index": false
        },
        {
          "name": "makerFee",
          "type": "u64",
          "index": false
        },
        {
          "name": "takerFee",
          "type": "u64",
          "index": false
        },
        {
          "name": "buyTokenPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "sellTokenPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "closed",
          "type": "bool",
          "index": false
        }
      ]
    },
    {
      "name": "TokenConditionalSwapTriggerLogV2",
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
          "name": "tokenConditionalSwapId",
          "type": "u64",
          "index": false
        },
        {
          "name": "buyTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "sellTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "buyAmount",
          "type": "u64",
          "index": false
        },
        {
          "name": "sellAmount",
          "type": "u64",
          "index": false
        },
        {
          "name": "makerFee",
          "type": "u64",
          "index": false
        },
        {
          "name": "takerFee",
          "type": "u64",
          "index": false
        },
        {
          "name": "buyTokenPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "sellTokenPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "closed",
          "type": "bool",
          "index": false
        },
        {
          "name": "displayPriceStyle",
          "type": "u8",
          "index": false
        },
        {
          "name": "intention",
          "type": "u8",
          "index": false
        }
      ]
    },
    {
      "name": "TokenConditionalSwapTriggerLogV3",
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
          "name": "tokenConditionalSwapId",
          "type": "u64",
          "index": false
        },
        {
          "name": "buyTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "sellTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "buyAmount",
          "type": "u64",
          "index": false
        },
        {
          "name": "sellAmount",
          "type": "u64",
          "index": false
        },
        {
          "name": "makerFee",
          "type": "u64",
          "index": false
        },
        {
          "name": "takerFee",
          "type": "u64",
          "index": false
        },
        {
          "name": "buyTokenPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "sellTokenPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "closed",
          "type": "bool",
          "index": false
        },
        {
          "name": "displayPriceStyle",
          "type": "u8",
          "index": false
        },
        {
          "name": "intention",
          "type": "u8",
          "index": false
        },
        {
          "name": "tcsType",
          "type": "u8",
          "index": false
        },
        {
          "name": "startTimestamp",
          "type": "u64",
          "index": false
        }
      ]
    },
    {
      "name": "TokenConditionalSwapCancelLog",
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
          "name": "id",
          "type": "u64",
          "index": false
        }
      ]
    },
    {
      "name": "TokenConditionalSwapStartLog",
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
          "name": "caller",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenConditionalSwapId",
          "type": "u64",
          "index": false
        },
        {
          "name": "incentiveTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "incentiveAmount",
          "type": "u64",
          "index": false
        }
      ]
    },
    {
      "name": "TokenCollateralFeeLog",
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
          "name": "assetUsageFraction",
          "type": "i128",
          "index": false
        },
        {
          "name": "fee",
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
      "msg": "health must be positive or not decrease"
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
    },
    {
      "code": 6028,
      "name": "TokenPositionDoesNotExist",
      "msg": "token position does not exist"
    },
    {
      "code": 6029,
      "name": "DepositsIntoLiquidatingMustRecover",
      "msg": "token deposits into accounts that are being liquidated must bring their health above the init threshold"
    },
    {
      "code": 6030,
      "name": "TokenInReduceOnlyMode",
      "msg": "token is in reduce only mode"
    },
    {
      "code": 6031,
      "name": "MarketInReduceOnlyMode",
      "msg": "market is in reduce only mode"
    },
    {
      "code": 6032,
      "name": "GroupIsHalted",
      "msg": "group is halted"
    },
    {
      "code": 6033,
      "name": "PerpHasBaseLots",
      "msg": "the perp position has non-zero base lots"
    },
    {
      "code": 6034,
      "name": "HasOpenOrUnsettledSerum3Orders",
      "msg": "there are open or unsettled serum3 orders"
    },
    {
      "code": 6035,
      "name": "HasLiquidatableTokenPosition",
      "msg": "has liquidatable token position"
    },
    {
      "code": 6036,
      "name": "HasLiquidatablePerpBasePosition",
      "msg": "has liquidatable perp base position"
    },
    {
      "code": 6037,
      "name": "HasLiquidatablePositivePerpPnl",
      "msg": "has liquidatable positive perp pnl"
    },
    {
      "code": 6038,
      "name": "AccountIsFrozen",
      "msg": "account is frozen"
    },
    {
      "code": 6039,
      "name": "InitAssetWeightCantBeNegative",
      "msg": "Init Asset Weight can't be negative"
    },
    {
      "code": 6040,
      "name": "HasOpenPerpTakerFills",
      "msg": "has open perp taker fills"
    },
    {
      "code": 6041,
      "name": "DepositLimit",
      "msg": "deposit crosses the current group deposit limit"
    },
    {
      "code": 6042,
      "name": "IxIsDisabled",
      "msg": "instruction is disabled"
    },
    {
      "code": 6043,
      "name": "NoLiquidatablePerpBasePosition",
      "msg": "no liquidatable perp base position"
    },
    {
      "code": 6044,
      "name": "PerpOrderIdNotFound",
      "msg": "perp order id not found on the orderbook"
    },
    {
      "code": 6045,
      "name": "HealthRegionBadInnerInstruction",
      "msg": "HealthRegions allow only specific instructions between Begin and End"
    },
    {
      "code": 6046,
      "name": "TokenInForceClose",
      "msg": "token is in force close"
    },
    {
      "code": 6047,
      "name": "InvalidHealthAccountCount",
      "msg": "incorrect number of health accounts"
    },
    {
      "code": 6048,
      "name": "WouldSelfTrade",
      "msg": "would self trade"
    },
    {
      "code": 6049,
      "name": "TokenConditionalSwapPriceNotInRange",
      "msg": "token conditional swap oracle price is not in execution range"
    },
    {
      "code": 6050,
      "name": "TokenConditionalSwapExpired",
      "msg": "token conditional swap is expired"
    },
    {
      "code": 6051,
      "name": "TokenConditionalSwapNotStarted",
      "msg": "token conditional swap is not available yet"
    },
    {
      "code": 6052,
      "name": "TokenConditionalSwapAlreadyStarted",
      "msg": "token conditional swap was already started"
    },
    {
      "code": 6053,
      "name": "TokenConditionalSwapNotSet",
      "msg": "token conditional swap it not set"
    },
    {
      "code": 6054,
      "name": "TokenConditionalSwapMinBuyTokenNotReached",
      "msg": "token conditional swap trigger did not reach min_buy_token"
    },
    {
      "code": 6055,
      "name": "TokenConditionalSwapCantPayIncentive",
      "msg": "token conditional swap cannot pay incentive"
    },
    {
      "code": 6056,
      "name": "TokenConditionalSwapTakerPriceTooLow",
      "msg": "token conditional swap taker price is too low"
    },
    {
      "code": 6057,
      "name": "TokenConditionalSwapIndexIdMismatch",
      "msg": "token conditional swap index and id don't match"
    },
    {
      "code": 6058,
      "name": "TokenConditionalSwapTooSmallForStartIncentive",
      "msg": "token conditional swap volume is too small compared to the cost of starting it"
    },
    {
      "code": 6059,
      "name": "TokenConditionalSwapTypeNotStartable",
      "msg": "token conditional swap type cannot be started"
    },
    {
      "code": 6060,
      "name": "HealthAccountBankNotWritable",
      "msg": "a bank in the health account list should be writable but is not"
    },
    {
      "code": 6061,
      "name": "Serum3PriceBandExceeded",
      "msg": "the market does not allow limit orders too far from the current oracle value"
    },
    {
      "code": 6062,
      "name": "BankDepositLimit",
      "msg": "deposit crosses the token's deposit limit"
    },
    {
      "code": 6063,
      "name": "DelegateWithdrawOnlyToOwnerAta",
      "msg": "delegates can only withdraw to the owner's associated token account"
    },
    {
      "code": 6064,
      "name": "DelegateWithdrawMustClosePosition",
      "msg": "delegates can only withdraw if they close the token position"
    },
    {
      "code": 6065,
      "name": "DelegateWithdrawSmall",
      "msg": "delegates can only withdraw small amounts"
    },
    {
      "code": 6066,
      "name": "InvalidCLMMOracle",
      "msg": "The provided CLMM oracle is not valid"
    },
    {
      "code": 6067,
      "name": "InvalidFeedForCLMMOracle",
      "msg": "invalid usdc/usd feed provided for the CLMM oracle"
    },
    {
      "code": 6068,
      "name": "MissingFeedForCLMMOracle",
      "msg": "Pyth USDC/USD or SOL/USD feed not found (required by CLMM oracle)"
    },
    {
      "code": 6069,
      "name": "TokenAssetLiquidationDisabled",
      "msg": "the asset does not allow liquidation"
    }
  ]
};

export const IDL: MangoV4 = {
  "version": "0.22.0",
  "name": "mango_v4",
  "instructions": [
    {
      "name": "adminTokenWithdrawFees",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "vault"
          ]
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
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": []
    },
    {
      "name": "adminPerpWithdrawFees",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "vault"
          ]
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
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": []
    },
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
          "isSigner": false,
          "relations": [
            "admin"
          ]
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
          "name": "securityAdminOpt",
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
        },
        {
          "name": "depositLimitQuoteOpt",
          "type": {
            "option": "u64"
          }
        },
        {
          "name": "buybackFeesOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "buybackFeesBonusFactorOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "buybackFeesSwapMangoAccountOpt",
          "type": {
            "option": "publicKey"
          }
        },
        {
          "name": "mngoTokenIndexOpt",
          "type": {
            "option": "u16"
          }
        },
        {
          "name": "buybackFeesExpiryIntervalOpt",
          "type": {
            "option": "u64"
          }
        },
        {
          "name": "allowedFastListingsPerIntervalOpt",
          "type": {
            "option": "u16"
          }
        },
        {
          "name": "collateralFeeIntervalOpt",
          "type": {
            "option": "u64"
          }
        }
      ]
    },
    {
      "name": "groupWithdrawInsuranceFund",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "insurance_vault",
            "admin"
          ]
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
          "name": "destination",
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
        }
      ]
    },
    {
      "name": "ixGateSet",
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
          "name": "ixGate",
          "type": "u128"
        }
      ]
    },
    {
      "name": "groupClose",
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "admin",
            "insurance_vault"
          ]
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
          "isSigner": false,
          "relations": [
            "admin"
          ]
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
          "name": "fallbackOracle",
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
          "name": "stablePriceDelayIntervalSeconds",
          "type": "u32"
        },
        {
          "name": "stablePriceDelayGrowthLimit",
          "type": "f32"
        },
        {
          "name": "stablePriceGrowthLimit",
          "type": "f32"
        },
        {
          "name": "minVaultToDepositsRatio",
          "type": "f64"
        },
        {
          "name": "netBorrowLimitWindowSizeTs",
          "type": "u64"
        },
        {
          "name": "netBorrowLimitPerWindowQuote",
          "type": "i64"
        },
        {
          "name": "borrowWeightScaleStartQuote",
          "type": "f64"
        },
        {
          "name": "depositWeightScaleStartQuote",
          "type": "f64"
        },
        {
          "name": "reduceOnly",
          "type": "u8"
        },
        {
          "name": "tokenConditionalSwapTakerFeeRate",
          "type": "f32"
        },
        {
          "name": "tokenConditionalSwapMakerFeeRate",
          "type": "f32"
        },
        {
          "name": "flashLoanSwapFeeRate",
          "type": "f32"
        },
        {
          "name": "interestCurveScaling",
          "type": "f32"
        },
        {
          "name": "interestTargetUtilization",
          "type": "f32"
        },
        {
          "name": "groupInsuranceFund",
          "type": "bool"
        },
        {
          "name": "depositLimit",
          "type": "u64"
        },
        {
          "name": "zeroUtilRate",
          "type": "f32"
        },
        {
          "name": "platformLiquidationFee",
          "type": "f32"
        },
        {
          "name": "disableAssetLiquidation",
          "type": "bool"
        },
        {
          "name": "collateralFeePerDay",
          "type": "f32"
        }
      ]
    },
    {
      "name": "tokenRegisterTrustless",
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
          "name": "fallbackOracle",
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The oracle account is optional and only used when reset_stable_price is set.",
            ""
          ]
        },
        {
          "name": "fallbackOracle",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The fallback oracle account is optional and only used when set_fallback_oracle is true.",
            ""
          ]
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
          "name": "netBorrowLimitPerWindowQuoteOpt",
          "type": {
            "option": "i64"
          }
        },
        {
          "name": "netBorrowLimitWindowSizeTsOpt",
          "type": {
            "option": "u64"
          }
        },
        {
          "name": "borrowWeightScaleStartQuoteOpt",
          "type": {
            "option": "f64"
          }
        },
        {
          "name": "depositWeightScaleStartQuoteOpt",
          "type": {
            "option": "f64"
          }
        },
        {
          "name": "resetStablePrice",
          "type": "bool"
        },
        {
          "name": "resetNetBorrowLimit",
          "type": "bool"
        },
        {
          "name": "reduceOnlyOpt",
          "type": {
            "option": "u8"
          }
        },
        {
          "name": "nameOpt",
          "type": {
            "option": "string"
          }
        },
        {
          "name": "forceCloseOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "tokenConditionalSwapTakerFeeRateOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "tokenConditionalSwapMakerFeeRateOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "flashLoanSwapFeeRateOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "interestCurveScalingOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "interestTargetUtilizationOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "maintWeightShiftStartOpt",
          "type": {
            "option": "u64"
          }
        },
        {
          "name": "maintWeightShiftEndOpt",
          "type": {
            "option": "u64"
          }
        },
        {
          "name": "maintWeightShiftAssetTargetOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "maintWeightShiftLiabTargetOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "maintWeightShiftAbort",
          "type": "bool"
        },
        {
          "name": "setFallbackOracle",
          "type": "bool"
        },
        {
          "name": "depositLimitOpt",
          "type": {
            "option": "u64"
          }
        },
        {
          "name": "zeroUtilRateOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "platformLiquidationFeeOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "disableAssetLiquidationOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "collateralFeePerDayOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "forceWithdrawOpt",
          "type": {
            "option": "bool"
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
          "isSigner": false,
          "relations": [
            "admin"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "mint"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "mint"
          ]
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
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "mintInfo",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "oracle",
            "group"
          ]
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
      "name": "accountCreateV2",
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
          "name": "tokenConditionalSwapCount",
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
          "isSigner": false,
          "relations": [
            "group",
            "owner"
          ]
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
      "name": "accountExpandV2",
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
          "relations": [
            "group",
            "owner"
          ]
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
        },
        {
          "name": "tokenConditionalSwapCount",
          "type": "u8"
        }
      ]
    },
    {
      "name": "accountSizeMigration",
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
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "owner"
          ]
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
        },
        {
          "name": "temporaryDelegateOpt",
          "type": {
            "option": "publicKey"
          }
        },
        {
          "name": "temporaryDelegateExpiryOpt",
          "type": {
            "option": "u64"
          }
        }
      ]
    },
    {
      "name": "accountToggleFreeze",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "freeze",
          "type": "bool"
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
          "isSigner": false,
          "relations": [
            "group",
            "owner"
          ]
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
      "args": [
        {
          "name": "forceClose",
          "type": "bool"
        }
      ]
    },
    {
      "name": "accountBuybackFeesWithMngo",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "daoAccount",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "mngoBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "mngoOracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "feesBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "feesOracle",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "maxBuybackUsd",
          "type": "u64"
        }
      ]
    },
    {
      "name": "stubOracleCreate",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "oracle",
          "isMut": true,
          "isSigner": true
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
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "oracle",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "oracle",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "stubOracleSetTest",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "oracle",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        }
      ],
      "args": [
        {
          "name": "price",
          "type": {
            "defined": "I80F48"
          }
        },
        {
          "name": "lastUpdateSlot",
          "type": "u64"
        },
        {
          "name": "deviation",
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "vault",
            "oracle"
          ]
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
        },
        {
          "name": "reduceOnly",
          "type": "bool"
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "vault",
            "oracle"
          ]
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
        },
        {
          "name": "reduceOnly",
          "type": "bool"
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "vault",
            "oracle"
          ]
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
      "name": "flashLoanSwapBegin",
      "docs": [
        "A version of flash_loan_begin that's specialized for swaps and needs fewer",
        "bytes in the transaction"
      ],
      "accounts": [
        {
          "name": "account",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "owner",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "inputMint",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "outputMint",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "associatedTokenProgram",
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
          "name": "loanAmount",
          "type": "u64"
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
      "name": "flashLoanEndV2",
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
          "name": "numLoans",
          "type": "u8"
        },
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
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": true,
          "docs": [
            "group admin or fast listing admin, checked at #1"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "baseBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
        },
        {
          "name": "oraclePriceBand",
          "type": "f32"
        }
      ]
    },
    {
      "name": "serum3EditMarket",
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
          "name": "market",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        }
      ],
      "args": [
        {
          "name": "reduceOnlyOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "forceCloseOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "nameOpt",
          "type": {
            "option": "string"
          }
        },
        {
          "name": "oraclePriceBandOpt",
          "type": {
            "option": "f32"
          }
        }
      ]
    },
    {
      "name": "serum3DeregisterMarket",
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "serumMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "indexReservation",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "serum_program",
            "serum_market_external"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "serum_program",
            "serum_market_external"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "serum_program",
            "serum_market_external"
          ]
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
          ],
          "relations": [
            "group"
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
          "name": "payerOracle",
          "isMut": false,
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
      "name": "serum3PlaceOrderV2",
      "docs": [
        "requires the receiver_bank in the health account list to be writable"
      ],
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
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "serum_program",
            "serum_market_external"
          ]
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
          ],
          "relations": [
            "group"
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
          "name": "payerOracle",
          "isMut": false,
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
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "serum_program",
            "serum_market_external"
          ]
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
      "name": "serum3CancelOrderByClientOrderId",
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
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "serum_program",
            "serum_market_external"
          ]
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
          "name": "clientOrderId",
          "type": "u64"
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
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "serum_program",
            "serum_market_external"
          ]
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
      "docs": [
        "Deprecated instruction that used to settles all free funds from the OpenOrders account",
        "into the MangoAccount.",
        "",
        "Any serum \"referrer rebates\" (ui fees) are considered Mango fees."
      ],
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
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "serum_program",
            "serum_market_external"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "quoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "baseBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "serum3SettleFundsV2",
      "docs": [
        "Like Serum3SettleFunds, but `fees_to_dao` determines if referrer rebates are considered fees",
        "or are credited to the MangoAccount."
      ],
      "accounts": [
        {
          "name": "v1",
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
              "relations": [
                "group"
              ]
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
              "isSigner": false,
              "relations": [
                "group",
                "serum_program",
                "serum_market_external"
              ]
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
              "isSigner": false,
              "relations": [
                "group"
              ]
            },
            {
              "name": "quoteVault",
              "isMut": true,
              "isSigner": false
            },
            {
              "name": "baseBank",
              "isMut": true,
              "isSigner": false,
              "relations": [
                "group"
              ]
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
          ]
        },
        {
          "name": "v2",
          "accounts": [
            {
              "name": "quoteOracle",
              "isMut": false,
              "isSigner": false
            },
            {
              "name": "baseOracle",
              "isMut": false,
              "isSigner": false
            }
          ]
        }
      ],
      "args": [
        {
          "name": "feesToDao",
          "type": "bool"
        }
      ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "serumMarket",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "serum_program",
            "serum_market_external"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "quoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "baseBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "insurance_vault"
          ]
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liabMintInfo",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "tokenForceCloseBorrowsWithToken",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "type": "u64"
        }
      ]
    },
    {
      "name": "tokenLiqBankruptcy",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "insurance_vault"
          ]
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liabMintInfo",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "tokenForceWithdraw",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "bank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "vault",
            "oracle"
          ]
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
          "name": "ownerAtaTokenAccount",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "alternateOwnerTokenAccount",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Only for the unusual case where the owner_ata account is not owned by account.owner"
          ]
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
          "isSigner": false,
          "relations": [
            "admin"
          ]
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
          "name": "maintBaseAssetWeight",
          "type": "f32"
        },
        {
          "name": "initBaseAssetWeight",
          "type": "f32"
        },
        {
          "name": "maintBaseLiabWeight",
          "type": "f32"
        },
        {
          "name": "initBaseLiabWeight",
          "type": "f32"
        },
        {
          "name": "maintOverallAssetWeight",
          "type": "f32"
        },
        {
          "name": "initOverallAssetWeight",
          "type": "f32"
        },
        {
          "name": "baseLiquidationFee",
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
          "name": "settlePnlLimitWindowSizeTs",
          "type": "u64"
        },
        {
          "name": "positivePnlLiquidationFee",
          "type": "f32"
        },
        {
          "name": "platformLiquidationFee",
          "type": "f32"
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The oracle account is optional and only used when reset_stable_price is set.",
            ""
          ]
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
          "name": "maintBaseAssetWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "initBaseAssetWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "maintBaseLiabWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "initBaseLiabWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "maintOverallAssetWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "initOverallAssetWeightOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "baseLiquidationFeeOpt",
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
          "name": "settlePnlLimitWindowSizeTsOpt",
          "type": {
            "option": "u64"
          }
        },
        {
          "name": "reduceOnlyOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "resetStablePrice",
          "type": "bool"
        },
        {
          "name": "positivePnlLiquidationFeeOpt",
          "type": {
            "option": "f32"
          }
        },
        {
          "name": "nameOpt",
          "type": {
            "option": "string"
          }
        },
        {
          "name": "forceCloseOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "platformLiquidationFeeOpt",
          "type": {
            "option": "f32"
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
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks",
            "event_queue"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks",
            "event_queue",
            "oracle"
          ]
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
      ],
      "returns": {
        "option": "u128"
      }
    },
    {
      "name": "perpPlaceOrderV2",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks",
            "event_queue",
            "oracle"
          ]
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
          "name": "selfTradeBehavior",
          "type": {
            "defined": "SelfTradeBehavior"
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
      ],
      "returns": {
        "option": "u128"
      }
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks",
            "event_queue",
            "oracle"
          ]
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
        },
        {
          "name": "maxOracleStalenessSlots",
          "type": "i32"
        }
      ],
      "returns": {
        "option": "u128"
      }
    },
    {
      "name": "perpPlaceOrderPeggedV2",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks",
            "event_queue",
            "oracle"
          ]
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
          "name": "selfTradeBehavior",
          "type": {
            "defined": "SelfTradeBehavior"
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
        },
        {
          "name": "maxOracleStalenessSlots",
          "type": "i32"
        }
      ],
      "returns": {
        "option": "u128"
      }
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "owner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "event_queue"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks",
            "oracle"
          ]
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "settlerOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "perpMarket",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "oracle"
          ]
        },
        {
          "name": "accountA",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "accountB",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "settleBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "perpForceClosePosition",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "oracle"
          ]
        },
        {
          "name": "accountA",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "accountB",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "isSigner": false,
          "relations": [
            "group",
            "oracle"
          ]
        },
        {
          "name": "account",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "settleBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "perpLiqBaseOrPositivePnl",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "oracle"
          ]
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "settleBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
        }
      ],
      "args": [
        {
          "name": "maxBaseTransfer",
          "type": "i64"
        },
        {
          "name": "maxPnlTransfer",
          "type": "u64"
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "bids",
            "asks"
          ]
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
      "name": "perpLiqNegativePnlOrBankruptcy",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "insurance_vault"
          ]
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "oracle"
          ]
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "settleBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "perpLiqNegativePnlOrBankruptcyV2",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "insurance_vault"
          ]
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "perpMarket",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group",
            "oracle"
          ]
        },
        {
          "name": "oracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "settleBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
          "name": "insuranceBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "insuranceBankVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "insuranceOracle",
          "isMut": false,
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
      "name": "tokenConditionalSwapCreate",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "buyBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "sellBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
        }
      ],
      "args": [
        {
          "name": "maxBuy",
          "type": "u64"
        },
        {
          "name": "maxSell",
          "type": "u64"
        },
        {
          "name": "expiryTimestamp",
          "type": "u64"
        },
        {
          "name": "priceLowerLimit",
          "type": "f64"
        },
        {
          "name": "priceUpperLimit",
          "type": "f64"
        },
        {
          "name": "pricePremiumRate",
          "type": "f64"
        },
        {
          "name": "allowCreatingDeposits",
          "type": "bool"
        },
        {
          "name": "allowCreatingBorrows",
          "type": "bool"
        }
      ]
    },
    {
      "name": "tokenConditionalSwapCreateV2",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "buyBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "sellBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
        }
      ],
      "args": [
        {
          "name": "maxBuy",
          "type": "u64"
        },
        {
          "name": "maxSell",
          "type": "u64"
        },
        {
          "name": "expiryTimestamp",
          "type": "u64"
        },
        {
          "name": "priceLowerLimit",
          "type": "f64"
        },
        {
          "name": "priceUpperLimit",
          "type": "f64"
        },
        {
          "name": "pricePremiumRate",
          "type": "f64"
        },
        {
          "name": "allowCreatingDeposits",
          "type": "bool"
        },
        {
          "name": "allowCreatingBorrows",
          "type": "bool"
        },
        {
          "name": "displayPriceStyle",
          "type": {
            "defined": "TokenConditionalSwapDisplayPriceStyle"
          }
        },
        {
          "name": "intention",
          "type": {
            "defined": "TokenConditionalSwapIntention"
          }
        }
      ]
    },
    {
      "name": "tokenConditionalSwapCreatePremiumAuction",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "buyBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "sellBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
        }
      ],
      "args": [
        {
          "name": "maxBuy",
          "type": "u64"
        },
        {
          "name": "maxSell",
          "type": "u64"
        },
        {
          "name": "expiryTimestamp",
          "type": "u64"
        },
        {
          "name": "priceLowerLimit",
          "type": "f64"
        },
        {
          "name": "priceUpperLimit",
          "type": "f64"
        },
        {
          "name": "maxPricePremiumRate",
          "type": "f64"
        },
        {
          "name": "allowCreatingDeposits",
          "type": "bool"
        },
        {
          "name": "allowCreatingBorrows",
          "type": "bool"
        },
        {
          "name": "displayPriceStyle",
          "type": {
            "defined": "TokenConditionalSwapDisplayPriceStyle"
          }
        },
        {
          "name": "intention",
          "type": {
            "defined": "TokenConditionalSwapIntention"
          }
        },
        {
          "name": "durationSeconds",
          "type": "u64"
        }
      ]
    },
    {
      "name": "tokenConditionalSwapCreateLinearAuction",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "buyBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "sellBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
        }
      ],
      "args": [
        {
          "name": "maxBuy",
          "type": "u64"
        },
        {
          "name": "maxSell",
          "type": "u64"
        },
        {
          "name": "expiryTimestamp",
          "type": "u64"
        },
        {
          "name": "priceStart",
          "type": "f64"
        },
        {
          "name": "priceEnd",
          "type": "f64"
        },
        {
          "name": "allowCreatingDeposits",
          "type": "bool"
        },
        {
          "name": "allowCreatingBorrows",
          "type": "bool"
        },
        {
          "name": "displayPriceStyle",
          "type": {
            "defined": "TokenConditionalSwapDisplayPriceStyle"
          }
        },
        {
          "name": "startTimestamp",
          "type": "u64"
        },
        {
          "name": "durationSeconds",
          "type": "u64"
        }
      ]
    },
    {
      "name": "tokenConditionalSwapCancel",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "buyBank",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The bank's token_index is checked at #1"
          ],
          "relations": [
            "group"
          ]
        },
        {
          "name": "sellBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        }
      ],
      "args": [
        {
          "name": "tokenConditionalSwapIndex",
          "type": "u8"
        },
        {
          "name": "tokenConditionalSwapId",
          "type": "u64"
        }
      ]
    },
    {
      "name": "tokenConditionalSwapTrigger",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorAuthority",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "tokenConditionalSwapIndex",
          "type": "u8"
        },
        {
          "name": "tokenConditionalSwapId",
          "type": "u64"
        },
        {
          "name": "maxBuyTokenToLiqee",
          "type": "u64"
        },
        {
          "name": "maxSellTokenToLiqor",
          "type": "u64"
        }
      ]
    },
    {
      "name": "tokenConditionalSwapTriggerV2",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorAuthority",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "tokenConditionalSwapIndex",
          "type": "u8"
        },
        {
          "name": "tokenConditionalSwapId",
          "type": "u64"
        },
        {
          "name": "maxBuyTokenToLiqee",
          "type": "u64"
        },
        {
          "name": "maxSellTokenToLiqor",
          "type": "u64"
        },
        {
          "name": "minBuyToken",
          "type": "u64"
        },
        {
          "name": "minTakerPrice",
          "type": "f32"
        }
      ]
    },
    {
      "name": "tokenConditionalSwapStart",
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "liqee",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqor",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "liqorAuthority",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "tokenConditionalSwapIndex",
          "type": "u8"
        },
        {
          "name": "tokenConditionalSwapId",
          "type": "u64"
        }
      ]
    },
    {
      "name": "tokenChargeCollateralFees",
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
          "relations": [
            "group"
          ]
        }
      ],
      "args": []
    },
    {
      "name": "altSet",
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "admin"
          ]
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
          "isSigner": false,
          "relations": [
            "admin"
          ]
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
      "docs": [
        "Warning, this instruction is for testing purposes only!"
      ],
      "accounts": [
        {
          "name": "group",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "account",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
        }
      ],
      "args": []
    },
    {
      "name": "openbookV2RegisterMarket",
      "docs": [
        "",
        "OpenbookV2",
        ""
      ],
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openbookV2Program",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2MarketExternal",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2Market",
          "isMut": true,
          "isSigner": false,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "OpenbookV2Market"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "group"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "openbook_v2_market_external"
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
                "value": "OpenbookV2Index"
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "baseBank",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "openbookV2EditMarket",
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
          "name": "market",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        }
      ],
      "args": [
        {
          "name": "reduceOnlyOpt",
          "type": {
            "option": "bool"
          }
        },
        {
          "name": "forceCloseOpt",
          "type": {
            "option": "bool"
          }
        }
      ]
    },
    {
      "name": "openbookV2DeregisterMarket",
      "accounts": [
        {
          "name": "group",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "admin"
          ]
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openbookV2Market",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "indexReservation",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "openbookV2CreateOpenOrders",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openbookV2Market",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "openbook_v2_program",
            "openbook_v2_market_external"
          ]
        },
        {
          "name": "openbookV2Program",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2MarketExternal",
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
                "value": "OpenOrders"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "openbook_v2_market"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "openbook_v2_market_external"
              },
              {
                "kind": "arg",
                "type": "u32",
                "path": "account_num"
              }
            ],
            "programId": {
              "kind": "account",
              "type": "publicKey",
              "path": "openbook_v2_program"
            }
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
      "args": [
        {
          "name": "accountNum",
          "type": "u32"
        }
      ]
    },
    {
      "name": "openbookV2CloseOpenOrders",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openbookV2Market",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "openbook_v2_program",
            "openbook_v2_market_external"
          ]
        },
        {
          "name": "openbookV2Program",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2MarketExternal",
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
      "name": "openbookV2PlaceOrder",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "openbookV2Market",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2Program",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2MarketExternal",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "bids",
            "asks",
            "event_heap"
          ]
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
          "name": "eventHeap",
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
          "name": "payerBank",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The bank that pays for the order, if necessary"
          ],
          "relations": [
            "group"
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
          "name": "payerOracle",
          "isMut": false,
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
          "type": "u8"
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
          "type": "u8"
        },
        {
          "name": "orderType",
          "type": "u8"
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
      "name": "openbookV2PlaceTakerOrder",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openbookV2Market",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "openbook_v2_program",
            "openbook_v2_market_external"
          ]
        },
        {
          "name": "openbookV2Program",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2MarketExternal",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "bids",
            "asks",
            "event_heap"
          ]
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
          "name": "eventHeap",
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
          "isSigner": false
        },
        {
          "name": "payerBank",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The bank that pays for the order, if necessary"
          ],
          "relations": [
            "group"
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
          "name": "payerOracle",
          "isMut": false,
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
          "type": "u8"
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
          "type": "u8"
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
      "name": "openbookV2CancelOrder",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "openbookV2Market",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "openbook_v2_program",
            "openbook_v2_market_external"
          ]
        },
        {
          "name": "openbookV2Program",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2MarketExternal",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "bids",
            "asks"
          ]
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
        }
      ],
      "args": [
        {
          "name": "side",
          "type": "u8"
        },
        {
          "name": "orderId",
          "type": "u128"
        }
      ]
    },
    {
      "name": "openbookV2SettleFunds",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "openbookV2Market",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "openbook_v2_program",
            "openbook_v2_market_external"
          ]
        },
        {
          "name": "openbookV2Program",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2MarketExternal",
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "quoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "baseBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "baseVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "quoteOracle",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "baseOracle",
          "isMut": false,
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
          "name": "feesToDao",
          "type": "bool"
        }
      ]
    },
    {
      "name": "openbookV2LiqForceCancelOrders",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "openbookV2Market",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "openbook_v2_program",
            "openbook_v2_market_external"
          ]
        },
        {
          "name": "openbookV2Program",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2MarketExternal",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "bids",
            "asks",
            "event_heap"
          ]
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
          "name": "eventHeap",
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
          "isSigner": false,
          "relations": [
            "group"
          ]
        },
        {
          "name": "quoteVault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "baseBank",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "group"
          ]
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
      "name": "openbookV2CancelAllOrders",
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
          "relations": [
            "group"
          ]
        },
        {
          "name": "authority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "openOrders",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "openbookV2Market",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "group",
            "openbook_v2_program",
            "openbook_v2_market_external"
          ]
        },
        {
          "name": "openbookV2Program",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "openbookV2MarketExternal",
          "isMut": false,
          "isSigner": false,
          "relations": [
            "bids",
            "asks"
          ]
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
      "name": "benchmark",
      "docs": [
        "",
        "benchmark",
        ""
      ],
      "accounts": [
        {
          "name": "dummy",
          "isMut": false,
          "isSigner": false
        }
      ],
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
            "name": "stablePriceModel",
            "type": {
              "defined": "StablePriceModel"
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
            "docs": [
              "The unscaled borrow interest curve is defined as continuous piecewise linear with the points:",
              "",
              "- 0% util: zero_util_rate",
              "- util0% util: rate0",
              "- util1% util: rate1",
              "- 100% util: max_rate",
              "",
              "The final rate is this unscaled curve multiplied by interest_curve_scaling."
            ],
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
            "docs": [
              "the 100% utilization rate",
              "",
              "This isn't the max_rate, since this still gets scaled by interest_curve_scaling,",
              "which is >=1."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "collectedFeesNative",
            "docs": [
              "Fees collected over the lifetime of the bank",
              "",
              "See fees_withdrawn for how much of the fees was withdrawn.",
              "See collected_liquidation_fees for the (included) subtotal for liquidation related fees."
            ],
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
            "docs": [
              "Liquidation fee that goes to the liqor.",
              "",
              "Liquidation always involves two tokens, and the sum of the two configured fees is used.",
              "",
              "A fraction of the price, like 0.05 for a 5% fee during liquidation.",
              "",
              "See also platform_liquidation_fee."
            ],
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
            "name": "minVaultToDepositsRatio",
            "docs": [
              "The maximum utilization allowed when borrowing is 1-this value",
              "WARNING: Outdated name, kept for IDL compatibility"
            ],
            "type": "f64"
          },
          {
            "name": "netBorrowLimitWindowSizeTs",
            "docs": [
              "Size in seconds of a net borrows window"
            ],
            "type": "u64"
          },
          {
            "name": "lastNetBorrowsWindowStartTs",
            "docs": [
              "Timestamp at which the last net borrows window started"
            ],
            "type": "u64"
          },
          {
            "name": "netBorrowLimitPerWindowQuote",
            "docs": [
              "Net borrow limit per window in quote native; set to -1 to disable."
            ],
            "type": "i64"
          },
          {
            "name": "netBorrowsInWindow",
            "docs": [
              "Sum of all deposits and borrows in the last window, in native units."
            ],
            "type": "i64"
          },
          {
            "name": "borrowWeightScaleStartQuote",
            "docs": [
              "Soft borrow limit in native quote",
              "",
              "Once the borrows on the bank exceed this quote value, init_liab_weight is scaled up.",
              "Set to f64::MAX to disable.",
              "",
              "See scaled_init_liab_weight()."
            ],
            "type": "f64"
          },
          {
            "name": "depositWeightScaleStartQuote",
            "docs": [
              "Limit for collateral of deposits in native quote",
              "",
              "Once the deposits in the bank exceed this quote value, init_asset_weight is scaled",
              "down to keep the total collateral value constant.",
              "Set to f64::MAX to disable.",
              "",
              "See scaled_init_asset_weight()."
            ],
            "type": "f64"
          },
          {
            "name": "reduceOnly",
            "type": "u8"
          },
          {
            "name": "forceClose",
            "type": "u8"
          },
          {
            "name": "disableAssetLiquidation",
            "docs": [
              "If set to 1, deposits cannot be liquidated when an account is liquidatable.",
              "That means bankrupt accounts may still have assets of this type deposited."
            ],
            "type": "u8"
          },
          {
            "name": "forceWithdraw",
            "type": "u8"
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
            "name": "feesWithdrawn",
            "type": "u64"
          },
          {
            "name": "tokenConditionalSwapTakerFeeRate",
            "docs": [
              "Fees for the token conditional swap feature"
            ],
            "type": "f32"
          },
          {
            "name": "tokenConditionalSwapMakerFeeRate",
            "type": "f32"
          },
          {
            "name": "flashLoanSwapFeeRate",
            "type": "f32"
          },
          {
            "name": "interestTargetUtilization",
            "docs": [
              "Target utilization: If actual utilization is higher, scale up interest.",
              "If it's lower, scale down interest (if possible)"
            ],
            "type": "f32"
          },
          {
            "name": "interestCurveScaling",
            "docs": [
              "Current interest curve scaling, always >= 1.0",
              "",
              "Except when first migrating to having this field, then 0.0"
            ],
            "type": "f64"
          },
          {
            "name": "potentialSerumTokens",
            "docs": [
              "Largest amount of tokens that might be added the the bank based on",
              "serum open order execution."
            ],
            "type": "u64"
          },
          {
            "name": "maintWeightShiftStart",
            "docs": [
              "Start timestamp in seconds at which maint weights should start to change away",
              "from maint_asset_weight, maint_liab_weight towards _asset_target and _liab_target.",
              "If _start and _end and _duration_inv are 0, no shift is configured."
            ],
            "type": "u64"
          },
          {
            "name": "maintWeightShiftEnd",
            "docs": [
              "End timestamp in seconds until which the maint weights should reach the configured targets."
            ],
            "type": "u64"
          },
          {
            "name": "maintWeightShiftDurationInv",
            "docs": [
              "Cache of the inverse of maint_weight_shift_end - maint_weight_shift_start,",
              "or zero if no shift is configured"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maintWeightShiftAssetTarget",
            "docs": [
              "Maint asset weight to reach at _shift_end."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maintWeightShiftLiabTarget",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "fallbackOracle",
            "docs": [
              "Oracle that may be used if the main oracle is stale or not confident enough.",
              "If this is Pubkey::default(), no fallback is available."
            ],
            "type": "publicKey"
          },
          {
            "name": "depositLimit",
            "docs": [
              "zero means none, in token native"
            ],
            "type": "u64"
          },
          {
            "name": "zeroUtilRate",
            "docs": [
              "The unscaled borrow interest curve point for zero utilization.",
              "",
              "See util0, rate0, util1, rate1, max_rate"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "platformLiquidationFee",
            "docs": [
              "Additional to liquidation_fee, but goes to the group owner instead of the liqor"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "collectedLiquidationFees",
            "docs": [
              "Platform fees that were collected during liquidation (in native tokens)",
              "",
              "See also collected_fees_native and fees_withdrawn."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "collectedCollateralFees",
            "docs": [
              "Collateral fees that have been collected (in native tokens)",
              "",
              "See also collected_fees_native and fees_withdrawn."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "collateralFeePerDay",
            "docs": [
              "The daily collateral fees rate for fully utilized collateral."
            ],
            "type": "f32"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                1900
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
            "name": "mngoTokenIndex",
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
            "name": "buybackFees",
            "type": "u8"
          },
          {
            "name": "buybackFeesMngoBonusFactor",
            "type": "f32"
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
            "name": "securityAdmin",
            "type": "publicKey"
          },
          {
            "name": "depositLimitQuote",
            "type": "u64"
          },
          {
            "name": "ixGate",
            "type": "u128"
          },
          {
            "name": "buybackFeesSwapMangoAccount",
            "type": "publicKey"
          },
          {
            "name": "buybackFeesExpiryInterval",
            "docs": [
              "Number of seconds after which fees that could be used with the fees buyback feature expire.",
              "",
              "The actual expiry is staggered such that the fees users accumulate are always",
              "available for at least this interval - but may be available for up to twice this time.",
              "",
              "When set to 0, there's no expiry of buyback fees."
            ],
            "type": "u64"
          },
          {
            "name": "fastListingIntervalStart",
            "docs": [
              "Fast-listings are limited per week, this is the start of the current fast-listing interval",
              "in seconds since epoch"
            ],
            "type": "u64"
          },
          {
            "name": "fastListingsInInterval",
            "docs": [
              "Number of fast listings that happened this interval"
            ],
            "type": "u16"
          },
          {
            "name": "allowedFastListingsPerInterval",
            "docs": [
              "Number of fast listings that are allowed per interval"
            ],
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
            "name": "collateralFeeInterval",
            "docs": [
              "Intervals in which collateral fee is applied"
            ],
            "type": "u64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                1800
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
            "name": "frozenUntil",
            "type": "u64"
          },
          {
            "name": "buybackFeesAccruedCurrent",
            "docs": [
              "Fees usable with the \"fees buyback\" feature.",
              "This tracks the ones that accrued in the current expiry interval."
            ],
            "type": "u64"
          },
          {
            "name": "buybackFeesAccruedPrevious",
            "docs": [
              "Fees buyback amount from the previous expiry interval."
            ],
            "type": "u64"
          },
          {
            "name": "buybackFeesExpiryTimestamp",
            "docs": [
              "End timestamp of the current expiry interval of the buyback fees amount."
            ],
            "type": "u64"
          },
          {
            "name": "nextTokenConditionalSwapId",
            "docs": [
              "Next id to use when adding a token condition swap"
            ],
            "type": "u64"
          },
          {
            "name": "temporaryDelegate",
            "type": "publicKey"
          },
          {
            "name": "temporaryDelegateExpiry",
            "type": "u64"
          },
          {
            "name": "lastCollateralFeeCharge",
            "docs": [
              "Time at which the last collateral fee was charged"
            ],
            "type": "u64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                152
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
          },
          {
            "name": "padding8",
            "type": "u32"
          },
          {
            "name": "tokenConditionalSwaps",
            "type": {
              "vec": {
                "defined": "TokenConditionalSwap"
              }
            }
          },
          {
            "name": "reservedDynamic",
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
            "name": "fallbackOracle",
            "type": "publicKey"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                2528
              ]
            }
          }
        ]
      }
    },
    {
      "name": "openbookV2Market",
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
            "name": "reduceOnly",
            "type": "u8"
          },
          {
            "name": "forceClose",
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
            "name": "openbookV2Program",
            "type": "publicKey"
          },
          {
            "name": "openbookV2MarketExternal",
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
                512
              ]
            }
          }
        ]
      }
    },
    {
      "name": "openbookV2MarketIndexReservation",
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
            "name": "lastUpdateTs",
            "type": "i64"
          },
          {
            "name": "lastUpdateSlot",
            "type": "u64"
          },
          {
            "name": "deviation",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                104
              ]
            }
          }
        ]
      }
    },
    {
      "name": "bookSide",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "roots",
            "type": {
              "array": [
                {
                  "defined": "OrderTreeRoot"
                },
                2
              ]
            }
          },
          {
            "name": "reservedRoots",
            "type": {
              "array": [
                {
                  "defined": "OrderTreeRoot"
                },
                4
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
          },
          {
            "name": "nodes",
            "type": {
              "defined": "OrderTreeNodes"
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
            "docs": [
              "Token index that settlements happen in.",
              "",
              "Currently required to be 0, USDC. In the future settlement",
              "may be allowed to happen in other tokens."
            ],
            "type": "u16"
          },
          {
            "name": "perpMarketIndex",
            "docs": [
              "Index of this perp market. Other data, like the MangoAccount's PerpPosition",
              "reference this market via this index. Unique for this group's perp markets."
            ],
            "type": "u16"
          },
          {
            "name": "blocked1",
            "docs": [
              "Field used to contain the trusted_market flag and is now unused."
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
            "name": "bump",
            "docs": [
              "PDA bump"
            ],
            "type": "u8"
          },
          {
            "name": "baseDecimals",
            "docs": [
              "Number of decimals used for the base token.",
              "",
              "Used to convert the oracle's price into a native/native price."
            ],
            "type": "u8"
          },
          {
            "name": "name",
            "docs": [
              "Name. Trailing zero bytes are ignored."
            ],
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          },
          {
            "name": "bids",
            "docs": [
              "Address of the BookSide account for bids"
            ],
            "type": "publicKey"
          },
          {
            "name": "asks",
            "docs": [
              "Address of the BookSide account for asks"
            ],
            "type": "publicKey"
          },
          {
            "name": "eventQueue",
            "docs": [
              "Address of the EventQueue account"
            ],
            "type": "publicKey"
          },
          {
            "name": "oracle",
            "docs": [
              "Oracle account address"
            ],
            "type": "publicKey"
          },
          {
            "name": "oracleConfig",
            "docs": [
              "Oracle configuration"
            ],
            "type": {
              "defined": "OracleConfig"
            }
          },
          {
            "name": "stablePriceModel",
            "docs": [
              "Maintains a stable price based on the oracle price that is less volatile."
            ],
            "type": {
              "defined": "StablePriceModel"
            }
          },
          {
            "name": "quoteLotSize",
            "docs": [
              "Number of quote native in a quote lot. Must be a power of 10.",
              "",
              "Primarily useful for increasing the tick size on the market: A lot price",
              "of 1 becomes a native price of quote_lot_size/base_lot_size becomes a",
              "ui price of quote_lot_size*base_decimals/base_lot_size/quote_decimals."
            ],
            "type": "i64"
          },
          {
            "name": "baseLotSize",
            "docs": [
              "Number of base native in a base lot. Must be a power of 10.",
              "",
              "Example: If base decimals for the underlying asset is 6, base lot size",
              "is 100 and and base position lots is 10_000 then base position native is",
              "1_000_000 and base position ui is 1."
            ],
            "type": "i64"
          },
          {
            "name": "maintBaseAssetWeight",
            "docs": [
              "These weights apply to the base position. The quote position has",
              "no explicit weight (but may be covered by the overall pnl asset weight)."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initBaseAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maintBaseLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initBaseLiabWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "openInterest",
            "docs": [
              "Number of base lots currently active in the market. Always >= 0.",
              "",
              "Since this counts positive base lots and negative base lots, the more relevant",
              "number of open base lot pairs is half this value."
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
            "name": "registrationTime",
            "docs": [
              "Timestamp in seconds that the market was registered at."
            ],
            "type": "u64"
          },
          {
            "name": "minFunding",
            "docs": [
              "Minimal funding rate per day, must be <= 0."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "maxFunding",
            "docs": [
              "Maximal funding rate per day, must be >= 0."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "impactQuantity",
            "docs": [
              "For funding, get the impact price this many base lots deep into the book."
            ],
            "type": "i64"
          },
          {
            "name": "longFunding",
            "docs": [
              "Current long funding value. Increasing it means that every long base lot",
              "needs to pay that amount of quote native in funding.",
              "",
              "PerpPosition uses and tracks it settle funding. Updated by the perp",
              "keeper instruction."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "shortFunding",
            "docs": [
              "See long_funding."
            ],
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
            "name": "baseLiquidationFee",
            "docs": [
              "Fees",
              "Fee for base position liquidation"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "makerFee",
            "docs": [
              "Fee when matching maker orders. May be negative."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "takerFee",
            "docs": [
              "Fee for taker orders, may not be negative."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "feesAccrued",
            "docs": [
              "Fees accrued in native quote currency",
              "these are increased when new fees are paid and decreased when perp_settle_fees is called"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "feesSettled",
            "docs": [
              "Fees settled in native quote currency",
              "these are increased when perp_settle_fees is called, and never decreased"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "feePenalty",
            "docs": [
              "Fee (in quote native) to charge for ioc orders"
            ],
            "type": "f32"
          },
          {
            "name": "settleFeeFlat",
            "docs": [
              "In native units of settlement token, given to each settle call above the",
              "settle_fee_amount_threshold if settling at least 1% of perp base pos value."
            ],
            "type": "f32"
          },
          {
            "name": "settleFeeAmountThreshold",
            "docs": [
              "Pnl settlement amount needed to be eligible for the flat fee."
            ],
            "type": "f32"
          },
          {
            "name": "settleFeeFractionLowHealth",
            "docs": [
              "Fraction of pnl to pay out as fee if +pnl account has low health.",
              "(limited to 2x settle_fee_flat)"
            ],
            "type": "f32"
          },
          {
            "name": "settlePnlLimitFactor",
            "docs": [
              "Controls the strictness of the settle limit.",
              "Set to a negative value to disable the limit.",
              "",
              "This factor applies to the settle limit in two ways",
              "- for the unrealized pnl settle limit, the factor is multiplied with the stable perp base value",
              "(i.e. limit_factor * base_native * stable_price)",
              "- when increasing the realized pnl settle limit (stored per PerpPosition), the factor is",
              "multiplied with the stable value of the perp pnl being realized",
              "(i.e. limit_factor * reduced_native * stable_price)",
              "",
              "See also PerpPosition::settle_pnl_limit_realized_trade"
            ],
            "type": "f32"
          },
          {
            "name": "padding3",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "settlePnlLimitWindowSizeTs",
            "docs": [
              "Window size in seconds for the perp settlement limit"
            ],
            "type": "u64"
          },
          {
            "name": "reduceOnly",
            "docs": [
              "If true, users may no longer increase their market exposure. Only actions",
              "that reduce their position are still allowed."
            ],
            "type": "u8"
          },
          {
            "name": "forceClose",
            "type": "u8"
          },
          {
            "name": "padding4",
            "type": {
              "array": [
                "u8",
                6
              ]
            }
          },
          {
            "name": "maintOverallAssetWeight",
            "docs": [
              "Weights for full perp market health, if positive"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "initOverallAssetWeight",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "positivePnlLiquidationFee",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "feesWithdrawn",
            "type": "u64"
          },
          {
            "name": "platformLiquidationFee",
            "docs": [
              "Additional to liquidation_fee, but goes to the group owner instead of the liqor"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "accruedLiquidationFees",
            "docs": [
              "Platform fees that were accrued during liquidation (in native tokens)",
              "",
              "These fees are also added to fees_accrued, this is just for bookkeeping the total",
              "liquidation fees that happened. So never decreases (different to fees_accrued)."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                1848
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
            "name": "reduceOnly",
            "type": "u8"
          },
          {
            "name": "forceClose",
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
                1
              ]
            }
          },
          {
            "name": "oraclePriceBand",
            "docs": [
              "Limit orders must be <= oracle * (1+band) and >= oracle / (1+band)",
              "",
              "Zero value is the default due to migration and disables the limit,",
              "same as f32::MAX."
            ],
            "type": "f32"
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
      "name": "FlashLoanTokenDetailV2",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tokenIndex",
            "type": "u16"
          },
          {
            "name": "changeAmount",
            "docs": [
              "The amount by which the user's token position changed at the end",
              "",
              "So if the user repaid the approved_amount in full, it'd be 0.",
              "",
              "Does NOT include the loan_origination_fee or deposit_fee, so the true",
              "change is `change_amount - loan_origination_fee - deposit_fee`."
            ],
            "type": "i128"
          },
          {
            "name": "loan",
            "docs": [
              "The amount that was a loan (<= approved_amount, depends on user's deposits)"
            ],
            "type": "i128"
          },
          {
            "name": "loanOriginationFee",
            "docs": [
              "The fee paid on the loan, not included in `loan` or `change_amount`"
            ],
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
          },
          {
            "name": "depositFee",
            "docs": [
              "Deposit fee paid for positive change_amount.",
              "",
              "Not factored into change_amount."
            ],
            "type": "i128"
          },
          {
            "name": "approvedAmount",
            "docs": [
              "The amount that was transfered out to the user"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "FlashLoanTokenDetailV3",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "tokenIndex",
            "type": "u16"
          },
          {
            "name": "changeAmount",
            "docs": [
              "The amount by which the user's token position changed at the end",
              "",
              "So if the user repaid the approved_amount in full, it'd be 0.",
              "",
              "Does NOT include the loan_origination_fee or deposit_fee, so the true",
              "change is `change_amount - loan_origination_fee - deposit_fee`."
            ],
            "type": "i128"
          },
          {
            "name": "loan",
            "docs": [
              "The amount that was a loan (<= approved_amount, depends on user's deposits)"
            ],
            "type": "i128"
          },
          {
            "name": "loanOriginationFee",
            "docs": [
              "The fee paid on the loan, not included in `loan` or `change_amount`"
            ],
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
          },
          {
            "name": "swapFee",
            "docs": [
              "Swap fee paid on the in token of a swap.",
              "",
              "Not factored into change_amount."
            ],
            "type": "i128"
          },
          {
            "name": "approvedAmount",
            "docs": [
              "The amount that was transfered out to the user"
            ],
            "type": "u64"
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
            "name": "highestPlacedBidInv",
            "docs": [
              "Track something like the highest open bid / lowest open ask, in native/native units.",
              "",
              "Tracking it exactly isn't possible since we don't see fills. So instead track",
              "the min/max of the _placed_ bids and asks.",
              "",
              "The value is reset in serum3_place_order when a new order is placed without an",
              "existing one on the book.",
              "",
              "0 is a special \"unset\" state."
            ],
            "type": "f64"
          },
          {
            "name": "lowestPlacedAsk",
            "type": "f64"
          },
          {
            "name": "potentialBaseTokens",
            "docs": [
              "An overestimate of the amount of tokens that might flow out of the open orders account.",
              "",
              "The bank still considers these amounts user deposits (see Bank::potential_serum_tokens)",
              "and that value needs to be updated in conjunction with these numbers.",
              "",
              "This estimation is based on the amount of tokens in the open orders account",
              "(see update_bank_potential_tokens() in serum3_place_order and settle)"
            ],
            "type": "u64"
          },
          {
            "name": "potentialQuoteTokens",
            "type": "u64"
          },
          {
            "name": "lowestPlacedBidInv",
            "docs": [
              "Track lowest bid/highest ask, same way as for highest bid/lowest ask.",
              "",
              "0 is a special \"unset\" state."
            ],
            "type": "f64"
          },
          {
            "name": "highestPlacedAsk",
            "type": "f64"
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
            "docs": [
              "Index of the current settle pnl limit window"
            ],
            "type": "u32"
          },
          {
            "name": "settlePnlLimitSettledInCurrentWindowNative",
            "docs": [
              "Amount of realized trade pnl and unrealized pnl that was already settled this window.",
              "",
              "Will be negative when negative pnl was settled.",
              "",
              "Note that this will be adjusted for bookkeeping reasons when the realized_trade settle",
              "limitchanges and is not useable for actually tracking how much pnl was settled",
              "on balance."
            ],
            "type": "i64"
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
              "Active position in oracle quote native. At the same time this is 1:1 a settle_token native amount.",
              "",
              "Example: Say there's a perp market on the BTC/USD price using SOL for settlement. The user buys",
              "one long contract for $20k, then base = 1, quote = -20k. The price goes to $21k. Now their",
              "unsettled pnl is (1 * 21k - 20k) __SOL__ = 1000 SOL. This is because the perp contract arbitrarily",
              "decides that each unit of price difference creates 1 SOL worth of settlement.",
              "(yes, causing 1 SOL of settlement for each $1 price change implies a lot of extra leverage; likely",
              "there should be an extra configurable scaling factor before we use this for cases like that)"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "quoteRunningNative",
            "docs": [
              "Tracks what the position is to calculate average entry & break even price"
            ],
            "type": "i64"
          },
          {
            "name": "longSettledFunding",
            "docs": [
              "Already settled long funding"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "shortSettledFunding",
            "docs": [
              "Already settled short funding"
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "bidsBaseLots",
            "docs": [
              "Base lots in open bids"
            ],
            "type": "i64"
          },
          {
            "name": "asksBaseLots",
            "docs": [
              "Base lots in open asks"
            ],
            "type": "i64"
          },
          {
            "name": "takerBaseLots",
            "docs": [
              "Amount of base lots on the EventQueue waiting to be processed"
            ],
            "type": "i64"
          },
          {
            "name": "takerQuoteLots",
            "docs": [
              "Amount of quote lots on the EventQueue waiting to be processed"
            ],
            "type": "i64"
          },
          {
            "name": "cumulativeLongFunding",
            "docs": [
              "Cumulative long funding in quote native units.",
              "If the user paid $1 in funding for a long position, this would be 1e6.",
              "Beware of the sign!",
              "",
              "(Display only)"
            ],
            "type": "f64"
          },
          {
            "name": "cumulativeShortFunding",
            "docs": [
              "Cumulative short funding in quote native units",
              "If the user paid $1 in funding for a short position, this would be -1e6.",
              "",
              "(Display only)"
            ],
            "type": "f64"
          },
          {
            "name": "makerVolume",
            "docs": [
              "Cumulative maker volume in quote native units",
              "",
              "(Display only)"
            ],
            "type": "u64"
          },
          {
            "name": "takerVolume",
            "docs": [
              "Cumulative taker volume in quote native units",
              "",
              "(Display only)"
            ],
            "type": "u64"
          },
          {
            "name": "perpSpotTransfers",
            "docs": [
              "Cumulative number of quote native units transfered from the perp position",
              "to the settle token spot position.",
              "",
              "For example, if the user settled $1 of positive pnl into their USDC spot",
              "position, this would be 1e6.",
              "",
              "(Display only)"
            ],
            "type": "i64"
          },
          {
            "name": "avgEntryPricePerBaseLot",
            "docs": [
              "The native average entry price for the base lots of the current position.",
              "Reset to 0 when the base position reaches or crosses 0."
            ],
            "type": "f64"
          },
          {
            "name": "deprecatedRealizedTradePnlNative",
            "docs": [
              "Deprecated field: Amount of pnl that was realized by bringing the base position closer to 0."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "oneshotSettlePnlAllowance",
            "docs": [
              "Amount of pnl that can be settled once.",
              "",
              "- The value is signed: a negative number means negative pnl can be settled.",
              "- A settlement in the right direction will decrease this amount.",
              "",
              "Typically added for fees, funding and liquidation."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "recurringSettlePnlAllowance",
            "docs": [
              "Amount of pnl that can be settled in each settle window.",
              "",
              "- Unsigned, the settlement can happen in both directions. Value is >= 0.",
              "- Previously stored a similar value that was signed, so in migration cases",
              "this value can be negative and should be .abs()ed.",
              "- If this value exceeds the current stable-upnl, it should be decreased,",
              "see apply_recurring_settle_pnl_allowance_constraint()",
              "",
              "When the base position is reduced, the settle limit contribution from the reduced",
              "base position is materialized into this value. When the base position increases,",
              "some of the allowance is taken away.",
              "",
              "This also gets increased when a liquidator takes over pnl."
            ],
            "type": "i64"
          },
          {
            "name": "realizedPnlForPositionNative",
            "docs": [
              "Trade pnl, fees, funding that were added over the current position's lifetime.",
              "",
              "Reset when the position changes sign or goes to zero.",
              "Not decreased by settling.",
              "",
              "This is tracked for display purposes: this value plus the difference between entry",
              "price and current price of the base position is the overall pnl."
            ],
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                88
              ]
            }
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
            "type": "u8"
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
            "name": "quantity",
            "type": "i64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                56
              ]
            }
          }
        ]
      }
    },
    {
      "name": "MangoAccountFixed",
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
            "type": "u8"
          },
          {
            "name": "inHealthRegion",
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
            "type": "i64"
          },
          {
            "name": "frozenUntil",
            "type": "u64"
          },
          {
            "name": "buybackFeesAccruedCurrent",
            "type": "u64"
          },
          {
            "name": "buybackFeesAccruedPrevious",
            "type": "u64"
          },
          {
            "name": "buybackFeesExpiryTimestamp",
            "type": "u64"
          },
          {
            "name": "nextTokenConditionalSwapId",
            "type": "u64"
          },
          {
            "name": "temporaryDelegate",
            "type": "publicKey"
          },
          {
            "name": "temporaryDelegateExpiry",
            "type": "u64"
          },
          {
            "name": "lastCollateralFeeCharge",
            "type": "u64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                152
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
            "type": "u8"
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
                72
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
            "docs": [
              "NodeTag"
            ],
            "type": "u8"
          },
          {
            "name": "ownerSlot",
            "docs": [
              "Index into the owning MangoAccount's PerpOpenOrders"
            ],
            "type": "u8"
          },
          {
            "name": "orderType",
            "docs": [
              "PostOrderType, this was added for TradingView move order"
            ],
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
            "name": "timeInForce",
            "docs": [
              "Time in seconds after `timestamp` at which the order expires.",
              "A value of 0 means no expiry."
            ],
            "type": "u16"
          },
          {
            "name": "padding2",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "key",
            "docs": [
              "The binary tree key, see new_node_key()"
            ],
            "type": "u128"
          },
          {
            "name": "owner",
            "docs": [
              "Address of the owning MangoAccount"
            ],
            "type": "publicKey"
          },
          {
            "name": "quantity",
            "docs": [
              "Number of base lots to buy or sell, always >=1"
            ],
            "type": "i64"
          },
          {
            "name": "timestamp",
            "docs": [
              "The time the order was placed"
            ],
            "type": "u64"
          },
          {
            "name": "pegLimit",
            "docs": [
              "If the effective price of an oracle pegged order exceeds this limit,",
              "it will be considered invalid and may be removed.",
              "",
              "Only applicable in the oracle_pegged OrderTree"
            ],
            "type": "i64"
          },
          {
            "name": "clientOrderId",
            "docs": [
              "User defined id for this order, used in FillEvents"
            ],
            "type": "u64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                32
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
            "type": "u8"
          },
          {
            "name": "data",
            "type": {
              "array": [
                "u8",
                119
              ]
            }
          }
        ]
      }
    },
    {
      "name": "OrderTreeRoot",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "maybeNode",
            "type": "u32"
          },
          {
            "name": "leafCount",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "OrderTreeNodes",
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
            "type": "u8"
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
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                512
              ]
            }
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
            "type": "u8"
          },
          {
            "name": "makerOut",
            "type": "u8"
          },
          {
            "name": "makerSlot",
            "type": "u8"
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
            "name": "padding2",
            "type": {
              "array": [
                "u8",
                32
              ]
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
            "name": "padding3",
            "type": {
              "array": [
                "u8",
                16
              ]
            }
          },
          {
            "name": "takerClientOrderId",
            "type": "u64"
          },
          {
            "name": "makerOrderId",
            "type": "u128"
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
            "name": "makerClientOrderId",
            "type": "u64"
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
            "type": "u8"
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
            "name": "orderId",
            "type": "u128"
          },
          {
            "name": "padding1",
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
            "name": "resetOnNonzeroPrice",
            "docs": [
              "If set to 1, the stable price will reset on the next non-zero price it sees."
            ],
            "type": "u8"
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
      "name": "TokenConditionalSwap",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "id",
            "type": "u64"
          },
          {
            "name": "maxBuy",
            "docs": [
              "maximum amount of native tokens to buy or sell"
            ],
            "type": "u64"
          },
          {
            "name": "maxSell",
            "type": "u64"
          },
          {
            "name": "bought",
            "docs": [
              "how many native tokens were already bought/sold"
            ],
            "type": "u64"
          },
          {
            "name": "sold",
            "type": "u64"
          },
          {
            "name": "expiryTimestamp",
            "docs": [
              "timestamp until which the conditional swap is valid"
            ],
            "type": "u64"
          },
          {
            "name": "priceLowerLimit",
            "docs": [
              "The lower or starting price:",
              "- For FixedPremium or PremiumAuctions, it's the lower end of the price range:",
              "the tcs can only be triggered if the oracle price exceeds this value.",
              "- For LinearAuctions it's the starting price that's offered at start_timestamp.",
              "",
              "The price is always in \"sell_token per buy_token\" units, which can be computed",
              "by dividing the buy token price by the sell token price.",
              "",
              "For FixedPremium or PremiumAuctions:",
              "",
              "The price must exceed this threshold to allow execution.",
              "",
              "This threshold is compared to the \"sell_token per buy_token\" oracle price.",
              "If that price is >= lower_limit and <= upper_limit the tcs may be executable.",
              "",
              "Example: Stop loss to get out of a SOL long: The user bought SOL at 20 USDC/SOL",
              "and wants to stop loss at 18 USDC/SOL. They'd set buy_token=USDC, sell_token=SOL",
              "so the reference price is in SOL/USDC units. Set price_lower_limit=toNative(1/18)",
              "and price_upper_limit=toNative(1/10). Also set allow_borrows=false.",
              "",
              "Example: Want to buy SOL with USDC if the price falls below 22 USDC/SOL.",
              "buy_token=SOL, sell_token=USDC, reference price is in USDC/SOL units. Set",
              "price_upper_limit=toNative(22), price_lower_limit=0."
            ],
            "type": "f64"
          },
          {
            "name": "priceUpperLimit",
            "docs": [
              "Parallel to price_lower_limit, but an upper limit / auction end price."
            ],
            "type": "f64"
          },
          {
            "name": "pricePremiumRate",
            "docs": [
              "The premium to pay over oracle price to incentivize execution."
            ],
            "type": "f64"
          },
          {
            "name": "takerFeeRate",
            "docs": [
              "The taker receives only premium_price * (1 - taker_fee_rate)"
            ],
            "type": "f32"
          },
          {
            "name": "makerFeeRate",
            "docs": [
              "The maker has to pay premium_price * (1 + maker_fee_rate)"
            ],
            "type": "f32"
          },
          {
            "name": "buyTokenIndex",
            "docs": [
              "indexes of tokens for the swap"
            ],
            "type": "u16"
          },
          {
            "name": "sellTokenIndex",
            "type": "u16"
          },
          {
            "name": "isConfigured",
            "docs": [
              "If this struct is in use. (tcs are stored in a static-length array)"
            ],
            "type": "u8"
          },
          {
            "name": "allowCreatingDeposits",
            "docs": [
              "may token purchases create deposits? (often users just want to get out of a borrow)"
            ],
            "type": "u8"
          },
          {
            "name": "allowCreatingBorrows",
            "docs": [
              "may token selling create borrows? (often users just want to get out of a long)"
            ],
            "type": "u8"
          },
          {
            "name": "displayPriceStyle",
            "docs": [
              "The stored prices are always \"sell token per buy token\", but if the user",
              "used \"buy token per sell token\" when creating the tcs order, we should continue",
              "to show them prices in that way.",
              "",
              "Stores a TokenConditionalSwapDisplayPriceStyle enum value"
            ],
            "type": "u8"
          },
          {
            "name": "intention",
            "docs": [
              "The intention the user had when placing this order, display-only",
              "",
              "Stores a TokenConditionalSwapIntention enum value"
            ],
            "type": "u8"
          },
          {
            "name": "tcsType",
            "docs": [
              "Stores a TokenConditionalSwapType enum value"
            ],
            "type": "u8"
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
            "name": "startTimestamp",
            "docs": [
              "In seconds since epoch. 0 means not-started.",
              "",
              "FixedPremium: Time of first trigger call. No other effect.",
              "PremiumAuction: Time of start or first trigger call. Can continue to trigger once started.",
              "LinearAuction: Set during creation, auction starts with price_lower_limit at this timestamp."
            ],
            "type": "u64"
          },
          {
            "name": "durationSeconds",
            "docs": [
              "Duration of the auction mechanism",
              "",
              "FixedPremium: ignored",
              "PremiumAuction: time after start that the premium needs to scale to price_premium_rate",
              "LinearAuction: time after start to go from price_lower_limit to price_upper_limit"
            ],
            "type": "u64"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                88
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
          },
          {
            "name": "SwapWithoutFee"
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
        "There are three types of health:",
        "- initial health (\"init\"): users can only open new positions if it's >= 0",
        "- maintenance health (\"maint\"): users get liquidated if it's < 0",
        "- liquidation end health: once liquidation started (see being_liquidated), it",
        "only stops once this is >= 0",
        "",
        "The ordering is",
        "init health <= liquidation end health <= maint health",
        "",
        "The different health types are realized by using different weights and prices:",
        "- init health: init weights with scaling, stable-price adjusted prices",
        "- liq end health: init weights without scaling, oracle prices",
        "- maint health: maint weights, oracle prices",
        ""
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Init"
          },
          {
            "name": "Maint"
          },
          {
            "name": "LiquidationEnd"
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
          },
          {
            "name": "TokenConditionalSwapTrigger"
          }
        ]
      }
    },
    {
      "name": "IxGate",
      "docs": [
        "Enum for lookup into ix gate",
        "note:",
        "total ix files 56,",
        "ix files included 48,",
        "ix files not included 8,",
        "- Benchmark,",
        "- ComputeAccountData,",
        "- GroupCreate",
        "- GroupEdit",
        "- IxGateSet,",
        "- PerpZeroOut,",
        "- PerpEditMarket,",
        "- TokenEdit,"
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "AccountClose"
          },
          {
            "name": "AccountCreate"
          },
          {
            "name": "AccountEdit"
          },
          {
            "name": "AccountExpand"
          },
          {
            "name": "AccountToggleFreeze"
          },
          {
            "name": "AltExtend"
          },
          {
            "name": "AltSet"
          },
          {
            "name": "FlashLoan"
          },
          {
            "name": "GroupClose"
          },
          {
            "name": "GroupCreate"
          },
          {
            "name": "HealthRegion"
          },
          {
            "name": "PerpCancelAllOrders"
          },
          {
            "name": "PerpCancelAllOrdersBySide"
          },
          {
            "name": "PerpCancelOrder"
          },
          {
            "name": "PerpCancelOrderByClientOrderId"
          },
          {
            "name": "PerpCloseMarket"
          },
          {
            "name": "PerpConsumeEvents"
          },
          {
            "name": "PerpCreateMarket"
          },
          {
            "name": "PerpDeactivatePosition"
          },
          {
            "name": "PerpLiqBaseOrPositivePnl"
          },
          {
            "name": "PerpLiqForceCancelOrders"
          },
          {
            "name": "PerpLiqNegativePnlOrBankruptcy"
          },
          {
            "name": "PerpPlaceOrder"
          },
          {
            "name": "PerpSettleFees"
          },
          {
            "name": "PerpSettlePnl"
          },
          {
            "name": "PerpUpdateFunding"
          },
          {
            "name": "Serum3CancelAllOrders"
          },
          {
            "name": "Serum3CancelOrder"
          },
          {
            "name": "Serum3CloseOpenOrders"
          },
          {
            "name": "Serum3CreateOpenOrders"
          },
          {
            "name": "Serum3DeregisterMarket"
          },
          {
            "name": "Serum3EditMarket"
          },
          {
            "name": "Serum3LiqForceCancelOrders"
          },
          {
            "name": "Serum3PlaceOrder"
          },
          {
            "name": "Serum3RegisterMarket"
          },
          {
            "name": "Serum3SettleFunds"
          },
          {
            "name": "StubOracleClose"
          },
          {
            "name": "StubOracleCreate"
          },
          {
            "name": "StubOracleSet"
          },
          {
            "name": "TokenAddBank"
          },
          {
            "name": "TokenDeposit"
          },
          {
            "name": "TokenDeregister"
          },
          {
            "name": "TokenLiqBankruptcy"
          },
          {
            "name": "TokenLiqWithToken"
          },
          {
            "name": "TokenRegister"
          },
          {
            "name": "TokenRegisterTrustless"
          },
          {
            "name": "TokenUpdateIndexAndRate"
          },
          {
            "name": "TokenWithdraw"
          },
          {
            "name": "AccountBuybackFeesWithMngo"
          },
          {
            "name": "TokenForceCloseBorrowsWithToken"
          },
          {
            "name": "PerpForceClosePosition"
          },
          {
            "name": "GroupWithdrawInsuranceFund"
          },
          {
            "name": "TokenConditionalSwapCreate"
          },
          {
            "name": "TokenConditionalSwapTrigger"
          },
          {
            "name": "TokenConditionalSwapCancel"
          },
          {
            "name": "OpenbookV2CancelOrder"
          },
          {
            "name": "OpenbookV2CloseOpenOrders"
          },
          {
            "name": "OpenbookV2CreateOpenOrders"
          },
          {
            "name": "OpenbookV2DeregisterMarket"
          },
          {
            "name": "OpenbookV2EditMarket"
          },
          {
            "name": "OpenbookV2LiqForceCancelOrders"
          },
          {
            "name": "OpenbookV2PlaceOrder"
          },
          {
            "name": "OpenbookV2PlaceTakeOrder"
          },
          {
            "name": "OpenbookV2RegisterMarket"
          },
          {
            "name": "OpenbookV2SettleFunds"
          },
          {
            "name": "AdminTokenWithdrawFees"
          },
          {
            "name": "AdminPerpWithdrawFees"
          },
          {
            "name": "AccountSizeMigration"
          },
          {
            "name": "TokenConditionalSwapStart"
          },
          {
            "name": "TokenConditionalSwapCreatePremiumAuction"
          },
          {
            "name": "TokenConditionalSwapCreateLinearAuction"
          },
          {
            "name": "Serum3PlaceOrderV2"
          },
          {
            "name": "TokenForceWithdraw"
          }
        ]
      }
    },
    {
      "name": "CheckLiquidatable",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "NotLiquidatable"
          },
          {
            "name": "Liquidatable"
          },
          {
            "name": "BecameNotLiquidatable"
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
          },
          {
            "name": "OrcaCLMM"
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
      "name": "SelfTradeBehavior",
      "docs": [
        "Self trade behavior controls how taker orders interact with resting limit orders of the same account.",
        "This setting has no influence on placing a resting or oracle pegged limit order that does not match",
        "immediately, instead it's the responsibility of the user to correctly configure his taker orders."
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
              },
              {
                "name": "max_oracle_staleness_slots",
                "type": "i32"
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
    },
    {
      "name": "TokenConditionalSwapDisplayPriceStyle",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "SellTokenPerBuyToken"
          },
          {
            "name": "BuyTokenPerSellToken"
          }
        ]
      }
    },
    {
      "name": "TokenConditionalSwapIntention",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Unknown"
          },
          {
            "name": "StopLoss"
          },
          {
            "name": "TakeProfit"
          }
        ]
      }
    },
    {
      "name": "TokenConditionalSwapType",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "FixedPremium"
          },
          {
            "name": "PremiumAuction"
          },
          {
            "name": "LinearAuction"
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
      "name": "FlashLoanLogV2",
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
              "defined": "FlashLoanTokenDetailV2"
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
      "name": "FlashLoanLogV3",
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
              "defined": "FlashLoanTokenDetailV3"
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
      "name": "FillLogV2",
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
          "name": "makerClientOrderId",
          "type": "u64",
          "index": false
        },
        {
          "name": "makerFee",
          "type": "f32",
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
          "name": "takerClientOrderId",
          "type": "u64",
          "index": false
        },
        {
          "name": "takerFee",
          "type": "f32",
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
      "name": "FillLogV3",
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
          "name": "makerClientOrderId",
          "type": "u64",
          "index": false
        },
        {
          "name": "makerFee",
          "type": "f32",
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
          "name": "takerClientOrderId",
          "type": "u64",
          "index": false
        },
        {
          "name": "takerFee",
          "type": "f32",
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
        },
        {
          "name": "makerClosedPnl",
          "type": "f64",
          "index": false
        },
        {
          "name": "takerClosedPnl",
          "type": "f64",
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
          "name": "oracleSlot",
          "type": "u64",
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
          "name": "feesSettled",
          "type": "i128",
          "index": false
        },
        {
          "name": "openInterest",
          "type": "i64",
          "index": false
        },
        {
          "name": "instantaneousFundingRate",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "PerpUpdateFundingLogV2",
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
          "name": "oracleSlot",
          "type": "u64",
          "index": false
        },
        {
          "name": "oracleConfidence",
          "type": "i128",
          "index": false
        },
        {
          "name": "oracleType",
          "type": {
            "defined": "OracleType"
          },
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
          "name": "feesSettled",
          "type": "i128",
          "index": false
        },
        {
          "name": "openInterest",
          "type": "i64",
          "index": false
        },
        {
          "name": "instantaneousFundingRate",
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
        },
        {
          "name": "borrowRate",
          "type": "i128",
          "index": false
        },
        {
          "name": "depositRate",
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
      "name": "UpdateRateLogV2",
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
          "name": "util0",
          "type": "i128",
          "index": false
        },
        {
          "name": "rate1",
          "type": "i128",
          "index": false
        },
        {
          "name": "util1",
          "type": "i128",
          "index": false
        },
        {
          "name": "maxRate",
          "type": "i128",
          "index": false
        },
        {
          "name": "curveScaling",
          "type": "f64",
          "index": false
        },
        {
          "name": "targetUtilization",
          "type": "f32",
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
      "name": "TokenLiqWithTokenLogV2",
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
          "name": "assetTransferFromLiqee",
          "type": "i128",
          "index": false
        },
        {
          "name": "assetTransferToLiqor",
          "type": "i128",
          "index": false
        },
        {
          "name": "assetLiquidationFee",
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
      "name": "Serum3OpenOrdersBalanceLogV2",
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
      "name": "WithdrawLoanLog",
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
          "name": "loanAmount",
          "type": "i128",
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
        },
        {
          "name": "price",
          "type": {
            "option": "i128"
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
        },
        {
          "name": "startingLiabDepositIndex",
          "type": "i128",
          "index": false
        },
        {
          "name": "endingLiabDepositIndex",
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
      "name": "TokenMetaDataLogV2",
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
          "name": "fallbackOracle",
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
      "name": "PerpLiqBaseOrPositivePnlLog",
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
          "name": "pnlTransfer",
          "type": "i128",
          "index": false
        },
        {
          "name": "pnlSettleLimitTransfer",
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
      "name": "PerpLiqBaseOrPositivePnlLogV2",
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
          "name": "baseTransferLiqee",
          "type": "i64",
          "index": false
        },
        {
          "name": "quoteTransferLiqee",
          "type": "i128",
          "index": false
        },
        {
          "name": "quoteTransferLiqor",
          "type": "i128",
          "index": false
        },
        {
          "name": "quotePlatformFee",
          "type": "i128",
          "index": false
        },
        {
          "name": "pnlTransfer",
          "type": "i128",
          "index": false
        },
        {
          "name": "pnlSettleLimitTransfer",
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
        },
        {
          "name": "startingLongFunding",
          "type": "i128",
          "index": false
        },
        {
          "name": "startingShortFunding",
          "type": "i128",
          "index": false
        },
        {
          "name": "endingLongFunding",
          "type": "i128",
          "index": false
        },
        {
          "name": "endingShortFunding",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "PerpLiqNegativePnlOrBankruptcyLog",
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
          "name": "settlement",
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
    },
    {
      "name": "AccountBuybackFeesWithMngoLog",
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
          "name": "buybackFees",
          "type": "i128",
          "index": false
        },
        {
          "name": "buybackMngo",
          "type": "i128",
          "index": false
        },
        {
          "name": "mngoBuybackPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "oraclePrice",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "FilledPerpOrderLog",
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
          "name": "seqNum",
          "type": "u64",
          "index": false
        }
      ]
    },
    {
      "name": "PerpTakerTradeLog",
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
          "name": "takerSide",
          "type": "u8",
          "index": false
        },
        {
          "name": "totalBaseLotsTaken",
          "type": "i64",
          "index": false
        },
        {
          "name": "totalBaseLotsDecremented",
          "type": "i64",
          "index": false
        },
        {
          "name": "totalQuoteLotsTaken",
          "type": "i64",
          "index": false
        },
        {
          "name": "totalQuoteLotsDecremented",
          "type": "i64",
          "index": false
        },
        {
          "name": "takerFeesPaid",
          "type": "i128",
          "index": false
        },
        {
          "name": "feePenalty",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "PerpForceClosePositionLog",
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
          "name": "accountA",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "accountB",
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
      "name": "TokenForceCloseBorrowsWithTokenLog",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
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
          "name": "feeFactor",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "TokenForceCloseBorrowsWithTokenLogV2",
      "fields": [
        {
          "name": "mangoGroup",
          "type": "publicKey",
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
          "name": "assetTransferFromLiqee",
          "type": "i128",
          "index": false
        },
        {
          "name": "assetTransferToLiqor",
          "type": "i128",
          "index": false
        },
        {
          "name": "assetLiquidationFee",
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
          "name": "feeFactor",
          "type": "i128",
          "index": false
        }
      ]
    },
    {
      "name": "TokenConditionalSwapCreateLog",
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
          "name": "id",
          "type": "u64",
          "index": false
        },
        {
          "name": "maxBuy",
          "type": "u64",
          "index": false
        },
        {
          "name": "maxSell",
          "type": "u64",
          "index": false
        },
        {
          "name": "expiryTimestamp",
          "type": "u64",
          "index": false
        },
        {
          "name": "priceLowerLimit",
          "type": "f64",
          "index": false
        },
        {
          "name": "priceUpperLimit",
          "type": "f64",
          "index": false
        },
        {
          "name": "pricePremiumRate",
          "type": "f64",
          "index": false
        },
        {
          "name": "takerFeeRate",
          "type": "f32",
          "index": false
        },
        {
          "name": "makerFeeRate",
          "type": "f32",
          "index": false
        },
        {
          "name": "buyTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "sellTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "allowCreatingDeposits",
          "type": "bool",
          "index": false
        },
        {
          "name": "allowCreatingBorrows",
          "type": "bool",
          "index": false
        }
      ]
    },
    {
      "name": "TokenConditionalSwapCreateLogV2",
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
          "name": "id",
          "type": "u64",
          "index": false
        },
        {
          "name": "maxBuy",
          "type": "u64",
          "index": false
        },
        {
          "name": "maxSell",
          "type": "u64",
          "index": false
        },
        {
          "name": "expiryTimestamp",
          "type": "u64",
          "index": false
        },
        {
          "name": "priceLowerLimit",
          "type": "f64",
          "index": false
        },
        {
          "name": "priceUpperLimit",
          "type": "f64",
          "index": false
        },
        {
          "name": "pricePremiumRate",
          "type": "f64",
          "index": false
        },
        {
          "name": "takerFeeRate",
          "type": "f32",
          "index": false
        },
        {
          "name": "makerFeeRate",
          "type": "f32",
          "index": false
        },
        {
          "name": "buyTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "sellTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "allowCreatingDeposits",
          "type": "bool",
          "index": false
        },
        {
          "name": "allowCreatingBorrows",
          "type": "bool",
          "index": false
        },
        {
          "name": "displayPriceStyle",
          "type": "u8",
          "index": false
        },
        {
          "name": "intention",
          "type": "u8",
          "index": false
        }
      ]
    },
    {
      "name": "TokenConditionalSwapCreateLogV3",
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
          "name": "id",
          "type": "u64",
          "index": false
        },
        {
          "name": "maxBuy",
          "type": "u64",
          "index": false
        },
        {
          "name": "maxSell",
          "type": "u64",
          "index": false
        },
        {
          "name": "expiryTimestamp",
          "type": "u64",
          "index": false
        },
        {
          "name": "priceLowerLimit",
          "type": "f64",
          "index": false
        },
        {
          "name": "priceUpperLimit",
          "type": "f64",
          "index": false
        },
        {
          "name": "pricePremiumRate",
          "type": "f64",
          "index": false
        },
        {
          "name": "takerFeeRate",
          "type": "f32",
          "index": false
        },
        {
          "name": "makerFeeRate",
          "type": "f32",
          "index": false
        },
        {
          "name": "buyTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "sellTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "allowCreatingDeposits",
          "type": "bool",
          "index": false
        },
        {
          "name": "allowCreatingBorrows",
          "type": "bool",
          "index": false
        },
        {
          "name": "displayPriceStyle",
          "type": "u8",
          "index": false
        },
        {
          "name": "intention",
          "type": "u8",
          "index": false
        },
        {
          "name": "tcsType",
          "type": "u8",
          "index": false
        },
        {
          "name": "startTimestamp",
          "type": "u64",
          "index": false
        },
        {
          "name": "durationSeconds",
          "type": "u64",
          "index": false
        }
      ]
    },
    {
      "name": "TokenConditionalSwapTriggerLog",
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
          "name": "tokenConditionalSwapId",
          "type": "u64",
          "index": false
        },
        {
          "name": "buyTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "sellTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "buyAmount",
          "type": "u64",
          "index": false
        },
        {
          "name": "sellAmount",
          "type": "u64",
          "index": false
        },
        {
          "name": "makerFee",
          "type": "u64",
          "index": false
        },
        {
          "name": "takerFee",
          "type": "u64",
          "index": false
        },
        {
          "name": "buyTokenPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "sellTokenPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "closed",
          "type": "bool",
          "index": false
        }
      ]
    },
    {
      "name": "TokenConditionalSwapTriggerLogV2",
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
          "name": "tokenConditionalSwapId",
          "type": "u64",
          "index": false
        },
        {
          "name": "buyTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "sellTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "buyAmount",
          "type": "u64",
          "index": false
        },
        {
          "name": "sellAmount",
          "type": "u64",
          "index": false
        },
        {
          "name": "makerFee",
          "type": "u64",
          "index": false
        },
        {
          "name": "takerFee",
          "type": "u64",
          "index": false
        },
        {
          "name": "buyTokenPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "sellTokenPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "closed",
          "type": "bool",
          "index": false
        },
        {
          "name": "displayPriceStyle",
          "type": "u8",
          "index": false
        },
        {
          "name": "intention",
          "type": "u8",
          "index": false
        }
      ]
    },
    {
      "name": "TokenConditionalSwapTriggerLogV3",
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
          "name": "tokenConditionalSwapId",
          "type": "u64",
          "index": false
        },
        {
          "name": "buyTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "sellTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "buyAmount",
          "type": "u64",
          "index": false
        },
        {
          "name": "sellAmount",
          "type": "u64",
          "index": false
        },
        {
          "name": "makerFee",
          "type": "u64",
          "index": false
        },
        {
          "name": "takerFee",
          "type": "u64",
          "index": false
        },
        {
          "name": "buyTokenPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "sellTokenPrice",
          "type": "i128",
          "index": false
        },
        {
          "name": "closed",
          "type": "bool",
          "index": false
        },
        {
          "name": "displayPriceStyle",
          "type": "u8",
          "index": false
        },
        {
          "name": "intention",
          "type": "u8",
          "index": false
        },
        {
          "name": "tcsType",
          "type": "u8",
          "index": false
        },
        {
          "name": "startTimestamp",
          "type": "u64",
          "index": false
        }
      ]
    },
    {
      "name": "TokenConditionalSwapCancelLog",
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
          "name": "id",
          "type": "u64",
          "index": false
        }
      ]
    },
    {
      "name": "TokenConditionalSwapStartLog",
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
          "name": "caller",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "tokenConditionalSwapId",
          "type": "u64",
          "index": false
        },
        {
          "name": "incentiveTokenIndex",
          "type": "u16",
          "index": false
        },
        {
          "name": "incentiveAmount",
          "type": "u64",
          "index": false
        }
      ]
    },
    {
      "name": "TokenCollateralFeeLog",
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
          "name": "assetUsageFraction",
          "type": "i128",
          "index": false
        },
        {
          "name": "fee",
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
      "msg": "health must be positive or not decrease"
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
    },
    {
      "code": 6028,
      "name": "TokenPositionDoesNotExist",
      "msg": "token position does not exist"
    },
    {
      "code": 6029,
      "name": "DepositsIntoLiquidatingMustRecover",
      "msg": "token deposits into accounts that are being liquidated must bring their health above the init threshold"
    },
    {
      "code": 6030,
      "name": "TokenInReduceOnlyMode",
      "msg": "token is in reduce only mode"
    },
    {
      "code": 6031,
      "name": "MarketInReduceOnlyMode",
      "msg": "market is in reduce only mode"
    },
    {
      "code": 6032,
      "name": "GroupIsHalted",
      "msg": "group is halted"
    },
    {
      "code": 6033,
      "name": "PerpHasBaseLots",
      "msg": "the perp position has non-zero base lots"
    },
    {
      "code": 6034,
      "name": "HasOpenOrUnsettledSerum3Orders",
      "msg": "there are open or unsettled serum3 orders"
    },
    {
      "code": 6035,
      "name": "HasLiquidatableTokenPosition",
      "msg": "has liquidatable token position"
    },
    {
      "code": 6036,
      "name": "HasLiquidatablePerpBasePosition",
      "msg": "has liquidatable perp base position"
    },
    {
      "code": 6037,
      "name": "HasLiquidatablePositivePerpPnl",
      "msg": "has liquidatable positive perp pnl"
    },
    {
      "code": 6038,
      "name": "AccountIsFrozen",
      "msg": "account is frozen"
    },
    {
      "code": 6039,
      "name": "InitAssetWeightCantBeNegative",
      "msg": "Init Asset Weight can't be negative"
    },
    {
      "code": 6040,
      "name": "HasOpenPerpTakerFills",
      "msg": "has open perp taker fills"
    },
    {
      "code": 6041,
      "name": "DepositLimit",
      "msg": "deposit crosses the current group deposit limit"
    },
    {
      "code": 6042,
      "name": "IxIsDisabled",
      "msg": "instruction is disabled"
    },
    {
      "code": 6043,
      "name": "NoLiquidatablePerpBasePosition",
      "msg": "no liquidatable perp base position"
    },
    {
      "code": 6044,
      "name": "PerpOrderIdNotFound",
      "msg": "perp order id not found on the orderbook"
    },
    {
      "code": 6045,
      "name": "HealthRegionBadInnerInstruction",
      "msg": "HealthRegions allow only specific instructions between Begin and End"
    },
    {
      "code": 6046,
      "name": "TokenInForceClose",
      "msg": "token is in force close"
    },
    {
      "code": 6047,
      "name": "InvalidHealthAccountCount",
      "msg": "incorrect number of health accounts"
    },
    {
      "code": 6048,
      "name": "WouldSelfTrade",
      "msg": "would self trade"
    },
    {
      "code": 6049,
      "name": "TokenConditionalSwapPriceNotInRange",
      "msg": "token conditional swap oracle price is not in execution range"
    },
    {
      "code": 6050,
      "name": "TokenConditionalSwapExpired",
      "msg": "token conditional swap is expired"
    },
    {
      "code": 6051,
      "name": "TokenConditionalSwapNotStarted",
      "msg": "token conditional swap is not available yet"
    },
    {
      "code": 6052,
      "name": "TokenConditionalSwapAlreadyStarted",
      "msg": "token conditional swap was already started"
    },
    {
      "code": 6053,
      "name": "TokenConditionalSwapNotSet",
      "msg": "token conditional swap it not set"
    },
    {
      "code": 6054,
      "name": "TokenConditionalSwapMinBuyTokenNotReached",
      "msg": "token conditional swap trigger did not reach min_buy_token"
    },
    {
      "code": 6055,
      "name": "TokenConditionalSwapCantPayIncentive",
      "msg": "token conditional swap cannot pay incentive"
    },
    {
      "code": 6056,
      "name": "TokenConditionalSwapTakerPriceTooLow",
      "msg": "token conditional swap taker price is too low"
    },
    {
      "code": 6057,
      "name": "TokenConditionalSwapIndexIdMismatch",
      "msg": "token conditional swap index and id don't match"
    },
    {
      "code": 6058,
      "name": "TokenConditionalSwapTooSmallForStartIncentive",
      "msg": "token conditional swap volume is too small compared to the cost of starting it"
    },
    {
      "code": 6059,
      "name": "TokenConditionalSwapTypeNotStartable",
      "msg": "token conditional swap type cannot be started"
    },
    {
      "code": 6060,
      "name": "HealthAccountBankNotWritable",
      "msg": "a bank in the health account list should be writable but is not"
    },
    {
      "code": 6061,
      "name": "Serum3PriceBandExceeded",
      "msg": "the market does not allow limit orders too far from the current oracle value"
    },
    {
      "code": 6062,
      "name": "BankDepositLimit",
      "msg": "deposit crosses the token's deposit limit"
    },
    {
      "code": 6063,
      "name": "DelegateWithdrawOnlyToOwnerAta",
      "msg": "delegates can only withdraw to the owner's associated token account"
    },
    {
      "code": 6064,
      "name": "DelegateWithdrawMustClosePosition",
      "msg": "delegates can only withdraw if they close the token position"
    },
    {
      "code": 6065,
      "name": "DelegateWithdrawSmall",
      "msg": "delegates can only withdraw small amounts"
    },
    {
      "code": 6066,
      "name": "InvalidCLMMOracle",
      "msg": "The provided CLMM oracle is not valid"
    },
    {
      "code": 6067,
      "name": "InvalidFeedForCLMMOracle",
      "msg": "invalid usdc/usd feed provided for the CLMM oracle"
    },
    {
      "code": 6068,
      "name": "MissingFeedForCLMMOracle",
      "msg": "Pyth USDC/USD or SOL/USD feed not found (required by CLMM oracle)"
    },
    {
      "code": 6069,
      "name": "TokenAssetLiquidationDisabled",
      "msg": "the asset does not allow liquidation"
    }
  ]
};
