pub enum Transaction {
    Aptos(aptos_types::Transaction),
    Sui(sui_types::Transaction)
}