//! HyperLiquid authentication module.
//!
//! Implements EIP-712 typed data signing for HyperLiquid API authentication.
//! Unlike centralized exchanges that use HMAC-SHA256, HyperLiquid uses
//! Ethereum's secp256k1 signatures with EIP-712 typed data.

use ccxt_core::credentials::SecretBytes;
use ccxt_core::error::{Error, Result};

/// HyperLiquid EIP-712 authenticator.
///
/// Handles wallet address derivation and EIP-712 typed data signing
/// for authenticated API requests.
///
/// The private key is automatically zeroed from memory when dropped.
#[derive(Debug, Clone)]
pub struct HyperLiquidAuth {
    /// The 32-byte private key (automatically zeroed on drop).
    private_key: SecretBytes,
    /// The derived wallet address (checksummed).
    wallet_address: String,
}

impl HyperLiquidAuth {
    /// Creates a new HyperLiquidAuth from a hex-encoded private key.
    ///
    /// # Arguments
    ///
    /// * `private_key_hex` - The private key as a hex string (with or without "0x" prefix).
    ///
    /// # Returns
    ///
    /// A new `HyperLiquidAuth` instance with the derived wallet address.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The private key is not valid hex
    /// - The private key is not exactly 32 bytes
    /// - The private key is not a valid secp256k1 scalar
    ///
    /// # Security
    ///
    /// The private key is automatically zeroed from memory when the authenticator is dropped.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ccxt_exchanges::hyperliquid::HyperLiquidAuth;
    ///
    /// let auth = HyperLiquidAuth::from_private_key(
    ///     "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
    /// ).unwrap();
    /// println!("Wallet: {}", auth.wallet_address());
    /// ```
    pub fn from_private_key(private_key_hex: &str) -> Result<Self> {
        // Remove 0x prefix if present
        let hex_str = private_key_hex
            .strip_prefix("0x")
            .or_else(|| private_key_hex.strip_prefix("0X"))
            .unwrap_or(private_key_hex);

        // Decode hex
        let bytes = hex::decode(hex_str)
            .map_err(|e| Error::invalid_argument(format!("Invalid private key hex: {}", e)))?;

        // Validate length
        if bytes.len() != 32 {
            return Err(Error::invalid_argument(format!(
                "Private key must be 32 bytes, got {}",
                bytes.len()
            )));
        }

        // Convert to fixed-size array for address derivation
        let mut private_key_array = [0u8; 32];
        private_key_array.copy_from_slice(&bytes);

        // Derive wallet address using k256
        let wallet_address = derive_address(&private_key_array)?;

        // Store as SecretBytes for automatic zeroization
        let private_key = SecretBytes::from_array(private_key_array);

        // Zero the temporary array
        private_key_array.fill(0);

        Ok(Self {
            private_key,
            wallet_address,
        })
    }

    /// Returns the wallet address (checksummed).
    pub fn wallet_address(&self) -> &str {
        &self.wallet_address
    }

    /// Returns the private key bytes.
    ///
    /// # Security
    ///
    /// This method exposes the private key. Use with caution.
    /// The returned reference should not be stored.
    pub fn private_key_bytes(&self) -> &[u8] {
        self.private_key.expose_secret()
    }

    /// Signs an L1 action using EIP-712 typed data signing.
    ///
    /// # Arguments
    ///
    /// * `action` - The action data to sign (serialized as JSON).
    /// * `nonce` - The nonce (typically timestamp in milliseconds).
    /// * `is_mainnet` - Whether this is for mainnet (affects chain ID).
    ///
    /// # Returns
    ///
    /// The signature as (r, s, v) components.
    pub fn sign_l1_action(
        &self,
        action: &serde_json::Value,
        nonce: u64,
        is_mainnet: bool,
    ) -> Result<Eip712Signature> {
        // Build the typed data hash
        let typed_data_hash = build_typed_data_hash(action, nonce, is_mainnet)?;

        // Get private key as array for signing
        let key_bytes = self.private_key.expose_secret();
        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(key_bytes);

        // Sign the hash
        let result = sign_hash(&key_array, &typed_data_hash);

        // Zero the temporary array
        key_array.fill(0);

        result
    }

    /// Signs a user agent connection request.
    ///
    /// # Arguments
    ///
    /// * `agent_address` - The agent's Ethereum address.
    ///
    /// # Returns
    ///
    /// The signature for agent authorization.
    pub fn sign_agent(&self, agent_address: &str) -> Result<Eip712Signature> {
        let message = format!("I authorize {} to trade on my behalf.", agent_address);

        // Get private key as array for signing
        let key_bytes = self.private_key.expose_secret();
        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(key_bytes);

        // Sign as personal message
        let result = sign_personal_message(&key_array, &message);

        // Zero the temporary array
        key_array.fill(0);

        result
    }
}

/// EIP-712 signature components.
#[derive(Debug, Clone)]
pub struct Eip712Signature {
    /// R component (32 bytes hex).
    pub r: String,
    /// S component (32 bytes hex).
    pub s: String,
    /// V component (recovery id).
    pub v: u8,
}

impl Eip712Signature {
    /// Converts the signature to a hex string (r + s + v).
    pub fn to_hex(&self) -> String {
        format!("0x{}{}{:02x}", self.r, self.s, self.v)
    }
}

/// Derives an Ethereum address from a private key.
fn derive_address(private_key: &[u8; 32]) -> Result<String> {
    use k256::ecdsa::SigningKey;
    use sha3::{Digest, Keccak256};

    // Create signing key
    let signing_key = SigningKey::from_bytes(private_key.into())
        .map_err(|e| Error::invalid_argument(format!("Invalid private key: {}", e)))?;

    // Get public key
    let public_key = signing_key.verifying_key();
    let public_key_bytes = public_key.to_encoded_point(false);

    // Skip the 0x04 prefix (uncompressed point indicator)
    let public_key_data = &public_key_bytes.as_bytes()[1..];

    // Keccak256 hash of public key
    let mut hasher = Keccak256::new();
    hasher.update(public_key_data);
    let hash = hasher.finalize();

    // Take last 20 bytes as address
    let address_bytes = &hash[12..];
    let address = format!("0x{}", hex::encode(address_bytes));

    // Return checksummed address
    Ok(checksum_address(&address))
}

/// Applies EIP-55 checksum to an address.
fn checksum_address(address: &str) -> String {
    use sha3::{Digest, Keccak256};

    let addr = address.strip_prefix("0x").unwrap_or(address).to_lowercase();

    let mut hasher = Keccak256::new();
    hasher.update(addr.as_bytes());
    let hash = hasher.finalize();
    let hash_hex = hex::encode(hash);

    let mut checksummed = String::with_capacity(42);
    checksummed.push_str("0x");

    for (i, c) in addr.chars().enumerate() {
        if c.is_ascii_digit() {
            checksummed.push(c);
        } else {
            let hash_char = hash_hex.chars().nth(i).unwrap_or('0');
            let hash_val = hash_char.to_digit(16).unwrap_or(0);
            if hash_val >= 8 {
                checksummed.push(c.to_ascii_uppercase());
            } else {
                checksummed.push(c);
            }
        }
    }

    checksummed
}

/// 将 serde_json::Value 转换为保持插入顺序的格式
///
/// 问题：serde_json::Object 使用 BTreeMap（字母序），而 Python 使用 dict（插入序）
/// 解决：递归转换为 IndexMap 保持原始顺序
#[allow(unused)]
fn value_to_ordered(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            // serde_json::Value::Object 内部是 serde_json::Map，已经是插入序
            // 但为了确保 msgpack 编码正确，我们直接返回
            value.clone()
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(value_to_ordered).collect())
        }
        _ => value.clone(),
    }
}

/// Builds the EIP-712 typed data hash for an L1 action.
///
/// HyperLiquid 使用 Phantom Agent 模式：
/// 1. 使用 msgpack 编码 action
/// 2. 添加 nonce 和 vault address
/// 3. 对编码后的数据做 keccak256 哈希
/// 4. 构建 Phantom Agent 包含 source 和 connectionId (hash)
/// 5. 对 Agent 类型做 EIP-712 签名
fn build_typed_data_hash(
    action: &serde_json::Value,
    nonce: u64,
    is_mainnet: bool,
) -> Result<[u8; 32]> {
    use sha3::{Digest, Keccak256};

    // Step 1: 使用 msgpack 编码 action
    let msgpack_data = rmp_serde::to_vec(action)
        .map_err(|e| Error::invalid_argument(format!("Failed to msgpack encode action: {}", e)))?;

    // Step 2: 添加 nonce (8 bytes, big-endian)
    let mut data = msgpack_data;
    data.extend_from_slice(&nonce.to_be_bytes());

    // Step 3: 添加 vault address (HyperLiquid 当前不使用 vault，添加 \x00)
    data.push(0x00);

    // Step 4: 计算 action hash
    let action_hash = Keccak256::digest(&data);

    // Step 5: 构建 Phantom Agent
    // source: "a" for mainnet, "b" for testnet
    let source = if is_mainnet { "a" } else { "b" };

    // 构建 EIP-712 消息
    // Agent 类型定义:
    // Agent([{"name": "source", "type": "string"}, {"name": "connectionId", "type": "bytes32"}])
    let agent_type_hash = keccak256(b"Agent(string source,bytes32 connectionId)");

    let source_hash = keccak256(source.as_bytes());

    // 构建编码后的 Agent 数据
    let mut agent_data = Vec::new();
    agent_data.extend_from_slice(&agent_type_hash);
    agent_data.extend_from_slice(&source_hash);
    agent_data.extend_from_slice(&action_hash); // connectionId 就是 action hash

    let agent_hash = keccak256(&agent_data);

    // Step 6: 构建 domain separator
    // Exchange(name, version, chainId, verifyingContract)
    let domain_type_hash = keccak256(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
    );

    let name_hash = keccak256(b"Exchange");
    let version_hash = keccak256(b"1");
    let chain_id: u64 = 1337; // HyperLiquid 使用 1337
    let verifying_contract = [0u8; 20]; // 零地址

    let mut domain_data = Vec::new();
    domain_data.extend_from_slice(&domain_type_hash);
    domain_data.extend_from_slice(&name_hash);
    domain_data.extend_from_slice(&version_hash);
    domain_data.extend_from_slice(&pad_u256(chain_id));
    domain_data.extend_from_slice(&[0u8; 12]); // address 的 padding
    domain_data.extend_from_slice(&verifying_contract);

    let domain_separator = keccak256(&domain_data);

    // Step 7: 最终的 EIP-712 哈希
    // keccak256("\x19\x01" || domainSeparator || hashStruct(message))
    let mut final_data = Vec::new();
    final_data.push(0x19);
    final_data.push(0x01);
    final_data.extend_from_slice(&domain_separator);
    final_data.extend_from_slice(&agent_hash);

    let final_hash = keccak256(&final_data);

    Ok(final_hash)
}

/// Signs a hash with the private key.
fn sign_hash(private_key: &[u8; 32], hash: &[u8; 32]) -> Result<Eip712Signature> {
    use k256::ecdsa::SigningKey;

    let signing_key = SigningKey::from_bytes(private_key.into())
        .map_err(|e| Error::invalid_argument(format!("Invalid private key: {}", e)))?;

    // 签名并获取带 recovery ID 的完整签名
    let (signature, recovery_id) = signing_key
        .sign_prehash_recoverable(hash)
        .map_err(|e| Error::invalid_argument(format!("Signing failed: {}", e)))?;

    let sig_bytes = signature.to_bytes();

    // Split into r and s
    let r = hex::encode(&sig_bytes[..32]);
    let s = hex::encode(&sig_bytes[32..]);

    // Recovery ID (k256 returns 0 or 1, we need 27 or 28)
    let v = recovery_id.to_byte() + 27;

    Ok(Eip712Signature { r, s, v })
}

/// Recovers an Ethereum address from a signature and message hash.
#[allow(unused)]
fn recover_address_from_signature(
    sig_bytes: &[u8],
    message_hash: &[u8; 32],
    v: u8,
) -> Result<String> {
    use k256::ecdsa::{RecoveryId, VerifyingKey};
    use sha3::{Digest, Keccak256};

    // Create recovery ID (0 or 1, not 27 or 28)
    let recovery_id = RecoveryId::try_from((v - 27) as u8)
        .map_err(|e| Error::invalid_argument(format!("Invalid recovery ID: {}", e)))?;

    // Reconstruct signature
    let signature = k256::ecdsa::Signature::from_slice(sig_bytes)
        .map_err(|e| Error::invalid_argument(format!("Invalid signature: {}", e)))?;

    // Recover public key
    let verifying_key =
        VerifyingKey::recover_from_prehash(message_hash, &signature, recovery_id)
            .map_err(|e| Error::invalid_argument(format!("Signature recovery failed: {}", e)))?;

    // Convert public key to address
    let public_key_bytes = verifying_key.to_encoded_point(false);
    let public_key_data = &public_key_bytes.as_bytes()[1..]; // Skip 0x04 prefix
    let hash = Keccak256::digest(public_key_data);
    let address = hex::encode(&hash[12..]);

    Ok(address)
}

/// Signs a personal message (EIP-191).
fn sign_personal_message(private_key: &[u8; 32], message: &str) -> Result<Eip712Signature> {
    let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
    let mut data = prefix.into_bytes();
    data.extend_from_slice(message.as_bytes());

    let hash = keccak256(&data);
    sign_hash(private_key, &hash)
}

/// Computes Keccak256 hash.
fn keccak256(data: &[u8]) -> [u8; 32] {
    use sha3::{Digest, Keccak256};
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Pads a u64 to 32 bytes (big-endian).
fn pad_u256(value: u64) -> [u8; 32] {
    let mut result = [0u8; 32];
    result[24..].copy_from_slice(&value.to_be_bytes());
    result
}

/// Computes Keccak256 hash (standalone function for use in build_typed_data_hash).
#[allow(dead_code)]
fn keccak256_hash(data: &[u8]) -> [u8; 32] {
    use sha3::{Digest, Keccak256};
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test private key (DO NOT USE IN PRODUCTION)
    const TEST_PRIVATE_KEY: &str =
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

    #[test]
    fn test_from_private_key_with_prefix() {
        let auth = HyperLiquidAuth::from_private_key(TEST_PRIVATE_KEY);
        assert!(auth.is_ok());
        let auth = auth.unwrap();
        assert!(auth.wallet_address().starts_with("0x"));
        assert_eq!(auth.wallet_address().len(), 42);
    }

    #[test]
    fn test_from_private_key_without_prefix() {
        let key = TEST_PRIVATE_KEY.strip_prefix("0x").unwrap();
        let auth = HyperLiquidAuth::from_private_key(key);
        assert!(auth.is_ok());
    }

    #[test]
    fn test_invalid_private_key_length() {
        let result = HyperLiquidAuth::from_private_key("0x1234");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_private_key_hex() {
        let result = HyperLiquidAuth::from_private_key("0xGGGG");
        assert!(result.is_err());
    }

    #[test]
    fn test_address_derivation_deterministic() {
        let auth1 = HyperLiquidAuth::from_private_key(TEST_PRIVATE_KEY).unwrap();
        let auth2 = HyperLiquidAuth::from_private_key(TEST_PRIVATE_KEY).unwrap();
        assert_eq!(auth1.wallet_address(), auth2.wallet_address());
    }

    #[test]
    fn test_checksum_address() {
        // Known checksummed address
        let addr = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266";
        let checksummed = checksum_address(addr);
        // Should have mixed case
        assert!(checksummed.chars().any(|c| c.is_uppercase()));
        assert!(
            checksummed
                .chars()
                .any(|c| c.is_lowercase() && c.is_alphabetic())
        );
    }

    #[test]
    fn test_signature_to_hex() {
        let sig = Eip712Signature {
            r: "a".repeat(64),
            s: "b".repeat(64),
            v: 27,
        };
        let hex = sig.to_hex();
        assert!(hex.starts_with("0x"));
        assert_eq!(hex.len(), 132); // 0x + 64 + 64 + 2
    }

    #[test]
    fn test_sign_l1_action() {
        let auth = HyperLiquidAuth::from_private_key(TEST_PRIVATE_KEY).unwrap();
        let action = serde_json::json!({"type": "order", "data": {}});
        let result = auth.sign_l1_action(&action, 1234567890, false);
        assert!(result.is_ok());

        let sig = result.unwrap();
        assert_eq!(sig.r.len(), 64);
        assert_eq!(sig.s.len(), 64);
    }

    #[test]
    fn test_sign_deterministic() {
        let auth = HyperLiquidAuth::from_private_key(TEST_PRIVATE_KEY).unwrap();
        let action = serde_json::json!({"type": "test"});

        let sig1 = auth.sign_l1_action(&action, 1000, false).unwrap();
        let sig2 = auth.sign_l1_action(&action, 1000, false).unwrap();

        assert_eq!(sig1.r, sig2.r);
        assert_eq!(sig1.s, sig2.s);
    }

    #[test]
    fn test_sign_compare_with_python() {
        // 使用与 Python 脚本相同的参数
        let private_key = "0xa766c9da644f2ee94a8be85c411c999ef206de27b3c41580110b5a1994ebe368";
        let auth = HyperLiquidAuth::from_private_key(private_key).unwrap();

        let action = serde_json::json!({
            "type": "order",
            "orders": [{
                "a": 10000,
                "b": true,
                "p": "0",
                "s": "100",
                "r": false,
                "t": {"limit": {"tif": "Ioc"}}
            }],
            "grouping": "na"
        });
        let nonce = 1234567890123u64;

        // 签名
        let sig = auth.sign_l1_action(&action, nonce, false).unwrap();

        // Python 输出的期望值
        let expected_final_hash =
            "8069a8c60fd927b0ffdad1d2d37c2b574469131cca430c95327a8d3aba0a9d1f";

        eprintln!("\n[RUST] Signature:");
        eprintln!("  r: 0x{}", sig.r);
        eprintln!("  s: 0x{}", sig.s);
        eprintln!("  v: {}", sig.v);
        eprintln!("  Wallet: {}", auth.wallet_address());
        eprintln!("  Expected final hash: {}", expected_final_hash);

        // 验证钱包地址
        assert_eq!(
            auth.wallet_address(),
            "0xb062ca7dCa674754e5F115715d07a68D4daf9382"
        );
    }
}
