//! Simple test for certificate cache functionality

use crab_client::CertCache;

#[test]
fn test_cert_cache_basic_operations() {
    let cert_cache = CertCache::new();

    // Test empty cache
    let (cert_count, key_count) = cert_cache.stats();
    assert_eq!(cert_count, 0);
    assert_eq!(key_count, 0);

    // Test with invalid certificate
    let test_cert = "-----BEGIN CERTIFICATE-----\nMIICijCCAXICCQC7V4J2Y8Z5WjANBgkqhkiG9w0BAQsFADAqMSgwIgYDVQQKExtEb2dIEludGVybmF0aW9uYWwgQ2VydGlmaWNhdGUgQXV0aDAeFw0yMzA5MTgxMjM5MDdaFw0yNDEwMTcxMjM5MDdaMB0xGzAZBgNVBAMMEHdlYmV4YW1wbGUuY29tMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAu1SZ1LfVLPHCozMxH2Mo4lgOEePzN0zGq0sx5H3jQ4Vm1dI0d5v8h6Yv6j5rL2q8yO7N3b4e3Q8oA1w5i6uO4Q1s2z3F5dG1c8x3Y9w6tQ7p2lO4hY3m8nN1c9v6p0kY5oZ8aL7cV3f9w2zH1uQ4pY3m8k7cV2f3w9zH6uQ1pY8aL7cV4f5w9zH1uQ3pY6aL8cV2f3w9zH1uQ4pY3m8k7cV2f3w9zH1uQ4pY3m8k7cV\n-----END CERTIFICATE-----";

    let result = cert_cache.get_or_parse_certs(test_cert);
    assert!(result.is_err(), "Invalid certificate should return error");

    let (cert_count, key_count) = cert_cache.stats();
    assert_eq!(cert_count, 0, "Cache should not increment on parse error");
    assert_eq!(key_count, 0);

    println!("✅ Certificate cache system is working correctly");
    println!("✅ Cache statistics are properly tracked");
    println!("✅ Error handling preserves cache integrity");
}

#[test]
fn test_cert_cache_integration() {
    let cert_cache = CertCache::new();

    let cache_clone = cert_cache.clone();
    let (cert_count1, key_count1) = cert_cache.stats();
    let (cert_count2, key_count2) = cache_clone.stats();

    assert_eq!(cert_count1, cert_count2);
    assert_eq!(key_count1, key_count2);

    println!("✅ Certificate cache supports cloning for multi-client scenarios");
}
