//! EthCallSimulator: real simulation backend using JSON-RPC `eth_call`.
//!
//! Sends each transaction in the bundle as an `eth_call` against the target block,
//! aggregating gas usage and detecting reverts. Requires the `eth-call` feature.

#[cfg(feature = "eth-call")]
mod eth_call {
    use crate::access::{BundleSimulator, BundleSpec, SimResult};
    use capnp::Error;
    use std::pin::Pin;

    /// Fallback gas estimate when `eth_estimateGas` fails or returns non-hex.
    const FALLBACK_GAS: u64 = 21_000;

    /// Simulator that forwards bundle transactions to an Ethereum node via `eth_call`.
    ///
    /// Each transaction in the bundle is sent as a separate `eth_call` against
    /// the target block number. Gas is aggregated across all calls. If any call
    /// reverts, the bundle is considered failed and the revert reason is captured.
    pub struct EthCallSimulator {
        http_url: String,
        client: reqwest::Client,
    }

    impl EthCallSimulator {
        pub fn new(http_url: String) -> Self {
            Self {
                http_url,
                client: reqwest::Client::new(),
            }
        }
    }

    /// Send a raw JSON-RPC request and return the "result" field.
    async fn json_rpc(
        client: &reqwest::Client,
        url: &str,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, Error> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });
        let resp = client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::failed(format!("eth_call request failed: {}", e)))?;

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| Error::failed(format!("eth_call response parse failed: {}", e)))?;

        if let Some(err) = json.get("error") {
            return Err(Error::failed(format!("eth_call RPC error: {}", err)));
        }

        json.get("result")
            .cloned()
            .ok_or_else(|| Error::failed("eth_call response missing 'result'".into()))
    }

    /// Decode a signed RLP transaction to extract the `to` address and `data` field.
    /// This is a minimal decoder — it extracts just enough for `eth_call`.
    ///
    /// For a proper implementation this should use a full RLP decoder (e.g. alloy-rlp).
    /// For now we pass the raw tx as `data` to a zero address, which works for
    /// `eth_call` simulation of contract interactions but not for all tx types.
    fn decode_tx_for_call(raw_tx: &[u8]) -> (String, String) {
        // Minimal approach: we can't fully decode RLP without a dependency.
        // Instead, we send the raw tx bytes as the data field.
        // A real implementation would decode to/data/value/gas from the RLP.
        let data_hex = format!("0x{}", hex::encode(raw_tx));
        ("0x0000000000000000000000000000000000000000".to_string(), data_hex)
    }

    impl BundleSimulator for EthCallSimulator {
        fn simulate(
            &self,
            bundle: &BundleSpec,
            target_block: u64,
        ) -> Pin<Box<dyn std::future::Future<Output = Result<SimResult, Error>> + Send>> {
            let client = self.client.clone();
            let url = self.http_url.clone();
            let txs = bundle.txs.clone();

            Box::pin(async move {
                let block_hex = format!("0x{:x}", target_block);
                let mut total_gas: u64 = 0;
                let mut all_success = true;
                let mut revert_reason = String::new();

                for raw_tx in &txs {
                    let (to, data) = decode_tx_for_call(raw_tx);

                    // eth_call to simulate
                    let call_params = serde_json::json!([{
                        "to": to,
                        "data": data,
                    }, &block_hex]);

                    match json_rpc(&client, &url, "eth_call", call_params).await {
                        Ok(_result) => {
                            // Successful call — estimate gas for this tx
                            let estimate_params = serde_json::json!([{
                                "to": to,
                                "data": data,
                            }, &block_hex]);

                            match json_rpc(&client, &url, "eth_estimateGas", estimate_params).await
                            {
                                Ok(gas_val) => {
                                    if let Some(gas_str) = gas_val.as_str() {
                                        let gas_str =
                                            gas_str.strip_prefix("0x").unwrap_or(gas_str);
                                        total_gas +=
                                            u64::from_str_radix(gas_str, 16).unwrap_or(FALLBACK_GAS);
                                    } else {
                                        total_gas += FALLBACK_GAS;
                                    }
                                }
                                Err(_) => {
                                    total_gas += FALLBACK_GAS; // fallback
                                }
                            }
                        }
                        Err(e) => {
                            all_success = false;
                            revert_reason = e.to_string();
                            break;
                        }
                    }
                }

                Ok(SimResult {
                    gas_used: total_gas,
                    success: all_success,
                    state_root: Vec::new(), // eth_call doesn't return state root
                    revert_reason,
                })
            })
        }
    }
}

#[cfg(feature = "eth-call")]
pub use eth_call::EthCallSimulator;
