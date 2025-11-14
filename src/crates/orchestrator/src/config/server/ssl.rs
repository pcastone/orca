//! SSL/TLS configuration and certificate management
//!
//! Handles SSL/TLS setup, X509 certificate generation, and PEM file management.

use crate::config::{SslConfig, SslMode, X509Config};
use rcgen::{Certificate, CertificateParams, DistinguishedName, DnType, KeyPair};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SslError {
    #[error("Failed to generate certificate: {0}")]
    CertificateGeneration(rcgen::RcgenError),
    #[error("Failed to write certificate file: {0}")]
    WriteError(std::io::Error),
    #[error("Failed to read certificate file: {0}")]
    ReadError(std::io::Error),
    #[error("Invalid SSL configuration: {0}")]
    InvalidConfig(String),
}

/// SSL certificate paths
#[derive(Debug, Clone)]
pub struct SslCertPaths {
    /// Path to certificate file (.crt or .pem)
    pub cert: PathBuf,
    /// Path to private key file (.key or .pem)
    pub key: PathBuf,
}

impl SslCertPaths {
    /// Default certificate paths in config directory
    pub fn default() -> Self {
        Self {
            cert: PathBuf::from("config/orchestrator-server.crt"),
            key: PathBuf::from("config/orchestrator-server.key"),
        }
    }
}

/// Generate X509 certificate and private key
pub fn generate_certificate(x509: &X509Config) -> Result<(String, String), SslError> {
    let mut params = CertificateParams::new(vec![]);
    
    // Set certificate validity
    params.not_before = time::OffsetDateTime::now_utc();
    params.not_after = params.not_before
        + time::Duration::days(x509.validity_days as i64);

    // Build Distinguished Name
    let mut distinguished_name = DistinguishedName::new();
    distinguished_name.push(DnType::CountryName, &x509.country);
    distinguished_name.push(DnType::StateOrProvinceName, &x509.state);
    distinguished_name.push(DnType::LocalityName, &x509.locality);
    distinguished_name.push(DnType::OrganizationName, &x509.organization);
    distinguished_name.push(DnType::OrganizationalUnitName, &x509.organizational_unit);
    distinguished_name.push(DnType::CommonName, &x509.common_name);
    
    params.distinguished_name = distinguished_name;

    // Generate certificate
    let cert = Certificate::from_params(params).map_err(SslError::CertificateGeneration)?;
    
    // Get PEM strings
    let cert_pem = cert.serialize_pem().map_err(SslError::CertificateGeneration)?;
    let key_pem = cert.serialize_private_key_pem();

    Ok((cert_pem, key_pem))
}

/// Setup SSL certificates based on configuration
pub fn setup_ssl_certificates(
    ssl_config: &SslConfig,
    cert_paths: Option<SslCertPaths>,
) -> Result<SslCertPaths, SslError> {
    let paths = cert_paths.unwrap_or_else(SslCertPaths::default);

    match ssl_config.mode {
        SslMode::Auto => {
            tracing::info!("Auto-generating SSL certificates...");
            
            // Generate certificate
            let (cert_pem, key_pem) = generate_certificate(&ssl_config.x509)?;
            
            // Ensure config directory exists
            if let Some(parent) = paths.cert.parent() {
                fs::create_dir_all(parent).map_err(SslError::WriteError)?;
            }
            
            // Write certificate
            fs::write(&paths.cert, cert_pem).map_err(SslError::WriteError)?;
            tracing::info!("Certificate written to: {:?}", paths.cert);
            
            // Write private key
            fs::write(&paths.key, key_pem).map_err(SslError::WriteError)?;
            tracing::info!("Private key written to: {:?}", paths.key);
            
            Ok(paths)
        }
        SslMode::Pem => {
            // Verify PEM files exist
            if !paths.cert.exists() {
                return Err(SslError::InvalidConfig(format!(
                    "Certificate file not found: {:?}",
                    paths.cert
                )));
            }
            if !paths.key.exists() {
                return Err(SslError::InvalidConfig(format!(
                    "Private key file not found: {:?}",
                    paths.key
                )));
            }
            
            tracing::info!("Using predefined PEM files:");
            tracing::info!("  Certificate: {:?}", paths.cert);
            tracing::info!("  Private key: {:?}", paths.key);
            
            Ok(paths)
        }
    }
}

/// Load certificate and key from files
pub fn load_certificates(paths: &SslCertPaths) -> Result<(Vec<u8>, Vec<u8>), SslError> {
    let cert = fs::read(&paths.cert).map_err(SslError::ReadError)?;
    let key = fs::read(&paths.key).map_err(SslError::ReadError)?;
    Ok((cert, key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_certificate_generation() {
        let x509 = X509Config {
            country: "US".to_string(),
            state: "CA".to_string(),
            locality: "SF".to_string(),
            organization: "Test".to_string(),
            organizational_unit: "Test".to_string(),
            common_name: "test.example.com".to_string(),
            validity_days: 365,
        };

        let (cert, key) = generate_certificate(&x509).unwrap();
        assert!(cert.contains("BEGIN CERTIFICATE"));
        assert!(key.contains("BEGIN PRIVATE KEY"));
    }

    #[test]
    fn test_auto_certificate_setup() {
        let temp_dir = TempDir::new().unwrap();
        let cert_path = temp_dir.path().join("test.crt");
        let key_path = temp_dir.path().join("test.key");

        let ssl_config = SslConfig {
            cipher: "ECDHE-RSA-AES256-GCM-SHA384".to_string(),
            version: "1.3".to_string(),
            mac: "SHA256".to_string(),
            mode: SslMode::Auto,
            x509: X509Config::default(),
        };

        let paths = SslCertPaths {
            cert: cert_path.clone(),
            key: key_path.clone(),
        };

        let result = setup_ssl_certificates(&ssl_config, Some(paths.clone()));
        assert!(result.is_ok());
        assert!(cert_path.exists());
        assert!(key_path.exists());
    }
}

