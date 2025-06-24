pub use redbit::*;

#[root_key] pub struct BlockHeight(pub u32);

#[pointer_key(u16)] pub struct TxPointer(BlockHeight);
#[pointer_key(u16)] pub struct UtxoPointer(TxPointer);
#[pointer_key(u16)] pub struct InputPointer(TxPointer);
#[pointer_key(u8)] pub struct AssetPointer(UtxoPointer);

#[index] pub struct Hash(pub String);
#[index] pub struct BlockHash(pub [u8; 32]);
#[index] pub struct Tree(pub Vec<u8>);
#[index] pub struct TreeT8(pub Vec<u8>);
#[index] pub struct BoxId(pub Vec<u8>);
#[index] pub struct TxHash(pub [u8; 32]);
#[index] pub struct Address(pub Vec<u8>);
#[index] pub struct PolicyId(pub Vec<u8>);
#[index] pub struct Datum(pub Vec<u8>);
#[index] pub struct AssetName(pub Vec<u8>);
#[index] pub struct AssetAction(pub u8);

#[index]
#[derive(Copy, Hash)]
pub struct BlockTimestamp(pub u32);
/*impl fmt::Display for BlockTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let datetime = DateTime::from_timestamp(self.0 as i64, 0).unwrap();
        let readable_date = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
        write!(f, "{}", readable_date)
    }
}
*/

#[entity]
pub struct Block {
    #[pk(range)]
    pub id: BlockHeight,
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    #[column]
    pub weight: u16
}

#[entity]
pub struct BlockHeader {
    #[fk(one2one, range)]
    pub id: BlockHeight,
    #[column(index)]
    pub hash: BlockHash,
    #[column(index)]
    pub prev_hash: BlockHash,
    #[column(index, range)]
    pub timestamp: BlockTimestamp,
}

#[entity]
pub struct Transaction {
    #[fk(one2many, range)]
    pub id: TxPointer,
    #[column(index)]
    pub hash: TxHash,
    pub utxos: Vec<Utxo>,
    pub inputs: Vec<InputRef>,
}

#[entity]
pub struct Utxo {
    #[fk(one2many, range)]
    pub id: UtxoPointer,
    #[column]
    pub amount: u64,
    #[column(index, dictionary)]
    pub address: Address,
    pub assets: Vec<Asset>,
    pub ergo_box: Option<Box>,
}

#[entity]
pub struct Box {
    #[fk(one2opt, range)]
    pub id: UtxoPointer,
    #[column(index)]
    pub box_id: BoxId,
    #[column(index)]
    pub tree: Tree,
    #[column(index)]
    pub tree_t8: TreeT8,
}

#[entity]
pub struct InputRef {
    #[fk(one2many, range)]
    pub id: InputPointer,
}

#[entity]
pub struct Asset {
    #[fk(one2many, range)]
    pub id: AssetPointer,
    #[column]
    pub amount: u64,
    #[column(index, dictionary)]
    pub name: AssetName,
    #[column(index, dictionary)]
    pub policy_id: PolicyId,
    #[column(index)]
    pub asset_action: AssetAction,
}
