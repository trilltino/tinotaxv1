//! Network-gated verification of address auto-detection against live
//! explorers. Ignored by default (hits public Blockscout instances):
//!   cargo test -p tinotax-app --test create_project_live -- --ignored

use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

async fn detect(address: &str, name: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let tmp = tempfile::tempdir()?;
    let parent = tmp.path().join("TinoTax").to_string_lossy().to_string();
    let result =
        tinotax_app::desktop_create_project_from_address(&parent, address, Some(name)).await?;
    assert!(std::path::Path::new(&result.config_path).exists());
    assert!(result.config_path.ends_with(".toml"));
    Ok(result
        .detected
        .iter()
        .map(|chain| chain.chain.clone())
        .collect())
}

#[test]
#[ignore = "hits live block explorers"]
fn detects_evm_chains_for_a_real_address() -> TestResult {
    let rt = tokio::runtime::Runtime::new()?;
    let chains = rt.block_on(detect("0x1b4399A7c97ae092fB4CCDc1598b2767ECB79652", "live detect"))?;
    // This address is active on both Lisk EVM and IOTA EVM.
    assert!(chains.iter().any(|c| c == "lisk-evm"), "expected lisk-evm, got {chains:?}");
    assert!(chains.iter().any(|c| c == "iota-evm"), "expected iota-evm, got {chains:?}");
    Ok(())
}

#[test]
#[ignore = "hits live block explorers"]
fn detects_added_evm_chains_for_an_eth_active_address() -> TestResult {
    let rt = tokio::runtime::Runtime::new()?;
    // vitalik.eth — active across Ethereum and several L2s.
    let chains = rt.block_on(detect("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045", "eth detect"))?;
    assert!(
        chains.iter().any(|c| c == "ethereum-evm"),
        "expected ethereum-evm among {chains:?}"
    );
    Ok(())
}
