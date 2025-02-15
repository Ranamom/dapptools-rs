use ethers::{
    prelude::BlockNumber,
    providers::Middleware,
    types::{Address, Block, BlockId, Bytes, TxHash, H256, U256, U64},
};
use tokio::runtime::{Handle, Runtime};

#[derive(Debug)]
/// Blocking wrapper around an Ethers middleware, for use in synchronous contexts
/// (powered by a tokio runtime)
pub struct BlockingProvider<M> {
    provider: M,
    runtime: Option<Runtime>,
}

impl<M: Clone> Clone for BlockingProvider<M> {
    fn clone(&self) -> Self {
        Self {
            provider: self.provider.clone(),
            runtime: self.runtime.as_ref().map(|_| Runtime::new().unwrap()),
        }
    }
}

impl<M: Middleware> BlockingProvider<M>
where
    M::Error: 'static,
{
    pub fn new(provider: M) -> Self {
        let runtime = Handle::try_current().is_err().then(|| Runtime::new().unwrap());
        Self { provider, runtime }
    }

    fn block_on<F: std::future::Future>(&self, f: F) -> F::Output {
        match &self.runtime {
            Some(runtime) => runtime.block_on(f),
            None => futures::executor::block_on(f),
        }
    }

    pub fn block_and_chainid(
        &self,
        block_id: Option<impl Into<BlockId>>,
    ) -> eyre::Result<(Block<TxHash>, U256)> {
        let block_id = block_id.map(Into::into).unwrap_or(BlockId::Number(BlockNumber::Latest));
        let f = async {
            let block = self.provider.get_block(block_id);
            let chain_id = self.provider.get_chainid();
            tokio::try_join!(block, chain_id)
        };
        let (block, chain_id) = self.block_on(f)?;
        Ok((block.ok_or_else(|| eyre::eyre!("block {:?} not found", block_id))?, chain_id))
    }

    pub fn get_account(
        &self,
        address: Address,
        block_id: Option<BlockId>,
    ) -> eyre::Result<(U256, U256, Bytes)> {
        let f = async {
            let balance = self.provider.get_balance(address, block_id);
            let nonce = self.provider.get_transaction_count(address, block_id);
            let code = self.provider.get_code(address, block_id);
            tokio::try_join!(balance, nonce, code)
        };
        let (balance, nonce, code) = self.block_on(f)?;

        Ok((nonce, balance, code))
    }

    pub fn get_block_number(&self) -> Result<U64, M::Error> {
        self.block_on(self.provider.get_block_number())
    }

    pub fn get_balance(&self, address: Address, block: Option<BlockId>) -> Result<U256, M::Error> {
        self.block_on(self.provider.get_balance(address, block))
    }

    pub fn get_transaction_count(
        &self,
        address: Address,
        block: Option<BlockId>,
    ) -> Result<U256, M::Error> {
        self.block_on(self.provider.get_transaction_count(address, block))
    }

    pub fn get_code(&self, address: Address, block: Option<BlockId>) -> Result<Bytes, M::Error> {
        self.block_on(self.provider.get_code(address, block))
    }

    pub fn get_storage_at(
        &self,
        address: Address,
        slot: H256,
        block: Option<BlockId>,
    ) -> Result<H256, M::Error> {
        self.block_on(self.provider.get_storage_at(address, slot, block))
    }
}
