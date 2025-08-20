# Boundless Load Test Configuration

This document provides an overview of how to run and configure the Boundless load test.

## Running the Test

To execute the load test, run the following command from this directory:

```bash
RUST_LOG=info cargo test test_prove_get_proof_data_boundless_load_test -- --nocapture
```

## Configurable Parameters

You can customize the test by modifying parameters in the following files.

### Test Execution Parameters

These parameters control the load test's behavior, such as the number of users, chains, and test duration.

**File:** `methods/tests/integration_tests.rs`
**Test Function:** `test_prove_get_proof_data_boundless_load_test`

-   **`num_iterations`**: The number of times the test scenario is repeated.
    -   *Example:* `let num_iterations = 10;`

-   **`users`**: Defines the number of users for each chain in the test. This is a nested vector where each inner vector represents a chain.
    -   *Example:* `test_users[..2].to_vec()` sets up 2 users.

-   **`assets`**: Specifies the market assets to be used for each user on each chain.
    -   *Example:* `vec![WETH_MARKET; 2]` assigns `WETH_MARKET` to 2 users.

-   **`dst_chain_ids`**: The destination chain IDs for the cross-chain operations.
    -   *Example:* `vec![OPTIMISM_CHAIN_ID; 2]` sets the destination to Optimism for 2 users.

-   **`chain_ids`**: The source chain IDs where the test transactions originate.
    -   *Example:* `vec![LINEA_CHAIN_ID, ETHEREUM_CHAIN_ID]` runs the test on Linea and Ethereum.

-   **Time Delay**: A delay between each test iteration.
    -   *Example:* `sleep(Duration::from_secs(15)).await;` sets a 15-second delay.

### Boundless Parameters

These parameters configure the interaction with the Boundless market, including pricing and timeouts for proof generation.

**File:** `malda_rs/src/viewcalls.rs`
**Function:** `get_proof_data_prove_boundless`

#### Client Builder Configuration

Inside the `BoundlessClient::builder()`:

-   **`max_price_per_cycle`**: Maximum price per cycle for automatic pricing.
-   **`min_price_per_cycle`**: Minimum price per cycle for automatic pricing.
-   **`ramp_up_period`**: (Optional) The duration over which the price ramps up in an auction.
-   **`lock_timeout`**: (Optional) Timeout for a prover to fulfill a locked request.
-   **`timeout`**: (Optional) Overall request timeout.

#### Request Builder Configuration

Inside the `.with_offer(OfferParams::builder()...)`:

-   **`min_price` / `max_price`**: (Optional) Sets a fixed price range for the proof request auction.
-   **`bidding_start`**: The Unix timestamp when bidding for the request should start.
-   **`timeout`**: (Optional) Maximum time the request can remain unfulfilled before expiring.
-   **`lock_timeout`**: (Optional) Time a prover has to fulfill the request after locking it.
-   **`ramp_up_period`**: (Optional) Duration for the price to ramp up in the auction.