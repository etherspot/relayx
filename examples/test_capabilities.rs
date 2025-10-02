/// Example client to test the relayer_getCapabilities endpoint
/// This demonstrates how to call the new endpoint and parse the response
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    
    // JSON-RPC request payload for relayer_getCapabilities
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "relayer_getCapabilities",
        "params": [],
        "id": 1
    });

    println!("Testing relayer_getCapabilities endpoint...");
    println!("Request: {}", serde_json::to_string_pretty(&request_body)?);

    // Make the request to the relayer server
    let response = client
        .post("http://127.0.0.1:4937")
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&request_body)?)
        .send()?;

    let status = response.status();
    println!("Response status: {}", status);

    let response_text = response.text()?;
    println!("Response body: {}", response_text);

    // Parse the response to demonstrate the structure
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&response_text) {
        if let Some(result) = parsed.get("result") {
            println!("\nParsed capabilities:");
            if let Some(capabilities) = result.get("capabilities") {
                if let Some(payments) = capabilities.get("payment") {
                    if let Some(payment_array) = payments.as_array() {
                        println!("Found {} payment options:", payment_array.len());
                        for (i, payment) in payment_array.iter().enumerate() {
                            if let Some(payment_type) = payment.get("type") {
                                print!("  {}: {:?}", i + 1, payment_type);
                                if let Some(token) = payment.get("token") {
                                    println!(" - Token: {}", token);
                                } else {
                                    println!();
                                }
                            }
                        }
                    }
                }
            }
        } else if let Some(error) = parsed.get("error") {
            println!("Error: {:?}", error);
        }
    }

    Ok(())
}
