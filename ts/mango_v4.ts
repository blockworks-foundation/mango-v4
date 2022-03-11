export type MangoV4 = {
  "version": "0.1.0",
  "name": "mango_v4",
  "instructions": [
    {
      "name": "createGroup",
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
                "value": "group"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "admin"
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
      "name": "registerToken",
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
                "value": "TokenBank"
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
                "value": "TokenVault"
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
          "name": "decimals",
          "type": "u8"
        }
      ]
    },
    {
      "name": "createAccount",
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
                "value": "account"
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
          "type": "u8"
        }
      ]
    },
    {
      "name": "deposit",
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
                "value": "TokenBank"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "account": "TokenAccount",
                "path": "deposit_token.mint"
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
                "value": "TokenVault"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "account": "TokenAccount",
                "path": "deposit_token.mint"
              }
            ]
          }
        },
        {
          "name": "depositToken",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "depositAuthority",
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
    }
  ],
  "accounts": [
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
            "name": "delegate",
            "type": "publicKey"
          },
          {
            "name": "indexedPositions",
            "type": {
              "defined": "IndexedPositions"
            }
          },
          {
            "name": "beingLiquidated",
            "type": "bool"
          },
          {
            "name": "isBankrupt",
            "type": "bool"
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
      "name": "mangoGroup",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "admin",
            "type": "publicKey"
          },
          {
            "name": "tokens",
            "type": {
              "defined": "Tokens"
            }
          }
        ]
      }
    },
    {
      "name": "TokenBank",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "depositIndex",
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
            "name": "indexedTotalDeposits",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "indexedTotalBorrows",
            "type": {
              "defined": "I80F48"
            }
          }
        ]
      }
    }
  ],
  "types": [
    {
      "name": "IndexedPosition",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "indexedValue",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "tokenIndex",
            "type": {
              "defined": "TokenIndex"
            }
          }
        ]
      }
    },
    {
      "name": "IndexedPositions",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "values",
            "type": {
              "array": [
                {
                  "defined": "IndexedPosition"
                },
                32
              ]
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
            "name": "mint",
            "type": "publicKey"
          },
          {
            "name": "decimals",
            "type": "u8"
          },
          {
            "name": "bankBump",
            "type": "u8"
          },
          {
            "name": "vaultBump",
            "type": "u8"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                30
              ]
            }
          }
        ]
      }
    },
    {
      "name": "Tokens",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "infos",
            "type": {
              "array": [
                {
                  "defined": "TokenInfo"
                },
                100
              ]
            }
          }
        ]
      }
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "SomeError",
      "msg": ""
    }
  ]
};

export const IDL: MangoV4 = {
  "version": "0.1.0",
  "name": "mango_v4",
  "instructions": [
    {
      "name": "createGroup",
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
                "value": "group"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "admin"
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
      "name": "registerToken",
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
                "value": "TokenBank"
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
                "value": "TokenVault"
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
          "name": "decimals",
          "type": "u8"
        }
      ]
    },
    {
      "name": "createAccount",
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
                "value": "account"
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
          "type": "u8"
        }
      ]
    },
    {
      "name": "deposit",
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
                "value": "TokenBank"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "account": "TokenAccount",
                "path": "deposit_token.mint"
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
                "value": "TokenVault"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "account": "TokenAccount",
                "path": "deposit_token.mint"
              }
            ]
          }
        },
        {
          "name": "depositToken",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "depositAuthority",
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
    }
  ],
  "accounts": [
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
            "name": "delegate",
            "type": "publicKey"
          },
          {
            "name": "indexedPositions",
            "type": {
              "defined": "IndexedPositions"
            }
          },
          {
            "name": "beingLiquidated",
            "type": "bool"
          },
          {
            "name": "isBankrupt",
            "type": "bool"
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
      "name": "mangoGroup",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "admin",
            "type": "publicKey"
          },
          {
            "name": "tokens",
            "type": {
              "defined": "Tokens"
            }
          }
        ]
      }
    },
    {
      "name": "TokenBank",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "depositIndex",
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
            "name": "indexedTotalDeposits",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "indexedTotalBorrows",
            "type": {
              "defined": "I80F48"
            }
          }
        ]
      }
    }
  ],
  "types": [
    {
      "name": "IndexedPosition",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "indexedValue",
            "type": {
              "defined": "I80F48"
            }
          },
          {
            "name": "tokenIndex",
            "type": {
              "defined": "TokenIndex"
            }
          }
        ]
      }
    },
    {
      "name": "IndexedPositions",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "values",
            "type": {
              "array": [
                {
                  "defined": "IndexedPosition"
                },
                32
              ]
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
            "name": "mint",
            "type": "publicKey"
          },
          {
            "name": "decimals",
            "type": "u8"
          },
          {
            "name": "bankBump",
            "type": "u8"
          },
          {
            "name": "vaultBump",
            "type": "u8"
          },
          {
            "name": "reserved",
            "type": {
              "array": [
                "u8",
                30
              ]
            }
          }
        ]
      }
    },
    {
      "name": "Tokens",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "infos",
            "type": {
              "array": [
                {
                  "defined": "TokenInfo"
                },
                100
              ]
            }
          }
        ]
      }
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "SomeError",
      "msg": ""
    }
  ]
};
