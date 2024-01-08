use super::*;

#[tokio::test]
async fn test_alt() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    let admin = TestKeypair::new();
    let payer = context.users[1].key;
    let mints = &context.mints[0..1];

    //
    // SETUP: Create a group, account, register a token (mint0)
    //

    let GroupWithTokens { group, .. } = GroupWithTokensConfig {
        admin,
        payer,
        mints: mints.to_vec(),
        ..GroupWithTokensConfig::default()
    }
    .create(solana)
    .await;

    //
    // TEST: Create and set an address lookup table
    //
    let group_data = solana.get_account::<Group>(group).await;
    assert!(group_data.address_lookup_tables[0] == Pubkey::default());

    let address_lookup_table = solana.create_address_lookup_table(payer, payer).await;

    send_tx(
        solana,
        AltSetInstruction {
            group,
            admin,
            index: 0,
            address_lookup_table,
        },
    )
    .await
    .unwrap();

    let group_data = solana.get_account::<Group>(group).await;
    assert!(group_data.address_lookup_tables[0] == address_lookup_table);
    assert!(group_data.address_lookup_tables[1] == Pubkey::default());

    //
    // TEST: Extend the lookup table
    //
    /* FUTURE: See alt_set
    assert_eq!(address_lookup_table_program::addresses(&solana.get_account_data(address_lookup_table).await.unwrap()).len(), 0);

    let new_addresses = vec![Pubkey::new_unique(), Pubkey::new_unique()];
    send_tx(solana, AltExtendInstruction {
        group,
        admin,
        payer,
        index: 0,
        address_lookup_table,
        new_addresses: new_addresses.clone(),
    }).await.unwrap();

    assert_eq!(address_lookup_table_program::addresses(&solana.get_account_data(address_lookup_table).await.unwrap()), &new_addresses);
    */

    Ok(())
}
