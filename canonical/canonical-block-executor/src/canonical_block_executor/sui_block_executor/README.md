# `sui-block-executor`
**Note**: This may be worthy of its own crate--even outside of the canonical development altogether--as we plan to reuse it in the future.

Sui does not have blocks by default. However, it is possible to use Sui's notion of checkpoints to create an execution model that is similar to blocks. Furthermore, we can disentangle this execution from consensus by using the checkpoint runtime by itself. This is the approach currently adopted in this module.