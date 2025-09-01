use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let base_url = "http://localhost:8545";

    println!("RelayX Test Client");
    println!("==================");

    // Test 1: Submit a new request
    println!("\n1. Submitting a new relayer request...");

    let submit_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "submit_request",
        "params": {
            "from_address": "0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6",
            "to_address": "0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6",
            "amount": "1000000000000000000",
            "gas_limit": 21000,
            "gas_price": "20000000000",
            "data": "0x",
            "nonce": 0,
            "chain_id": 1
        }
    });

    let response = client.post(base_url).json(&submit_request).send().await?;

    let response_json: Value = response.json().await?;
    println!(
        "Response: {}",
        serde_json::to_string_pretty(&response_json)?
    );

    // Extract request ID for the next test
    let request_id = if let Some(result) = response_json.get("result") {
        result.as_str().unwrap_or("")
    } else {
        println!("Failed to get request ID from response");
        return Ok(());
    };

    if !request_id.is_empty() {
        // Test 2: Get request status
        println!("\n2. Getting request status for ID: {}", request_id);

        let get_status = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "get_request_status",
            "params": request_id
        });

        let status_response = client.post(base_url).json(&get_status).send().await?;

        let status_json: Value = status_response.json().await?;
        println!(
            "Status Response: {}",
            serde_json::to_string_pretty(&status_json)?
        );
    }

    // Test 3: Health check
    println!("\n3. Performing health check...");

    let health_check = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "health_check",
        "params": null
    });

    let health_response = client.post(base_url).json(&health_check).send().await?;

    let health_json: Value = health_response.json().await?;
    println!(
        "Health Response: {}",
        serde_json::to_string_pretty(&health_json)?
    );

    println!("\nTest completed successfully!");
    Ok(())
}
