pub struct ReferrerMemory {
    pub meta_data: MetaData,
    pub referrer_mango_account: Pubkey,
}

pub struct ReferrerIdRecord {
    pub meta_data: MetaData,
    pub referrer_mango_account: Pubkey,
    pub id: [u8; INFO_LEN], // this id is one of the seeds
}
