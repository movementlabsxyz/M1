// todo: reduce import depth
use crate::transaction::transaction::Transaction;

pub struct Block(Vec<Transaction>);