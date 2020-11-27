# OysterPack NEAR STAKE Token
The OysterPack STAKE token is backed by staked NEAR tokens. 
It enables you to trade your staked NEAR, i.e., you can stake your NEAR and use it as money.

When you stake your NEAR, it will get locked up within the staking pool contract.
OysterPack will issue STAKE token for staked NEAR, which enables the user to use the STAKE as a currency.

STAKE token value is pegged to NEAR token value. As staking rewards are earned, the STAKE token value increases.
In other words, STAKE tokens appreciate in NEAR token value over time.

# How staking works
The customer deposits NEAR and specifies which staking pool to delegate staking to. In exchange, the customer receives
STAKE token.

## Account Storage Fees
Any applicable account storage fees are deducted from the deposit and escrowed. Storage fee deposits will be refunded 
when storage use decreases, e.g., when all NEAR is unstaked and sithdrawn. Unstaking and withdrawing all NEAR effectively 
closes the customer account, i.e. all persistent account state will be deleted from storage.

# How is the STAKE token valued

# How to redeem STAKE tokens for NEAR tokens
There is a limitation in the staking pool contracts that needs to be worked around. Unstaked NEAR is not available for
withdrawal for 4 epochs. However, if another unstaking transaction is submitted, then the total unstaked NEAR balance
is locked for another 4 epochs. For example, 50 NEAR are unstaked at epoch 100, which means the 50 NEAR is available
for withdrawal at epoch 104. However, if a user submits a transaction to unstake another 50 NEAR at epoch 103, then
the entire 100 unstaked NEAR will be available to be withdrawn at epoch 107. In this example, in order to be able to 
withdraw the 50 NEAR at epoch 104, the 2nd unstaking request must be submitted after the NEAR is withdrawn.
