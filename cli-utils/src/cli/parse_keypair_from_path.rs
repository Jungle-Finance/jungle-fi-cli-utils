/// Mostly copy-pasted CLI code from Solana SDK, and narrowed to a [Keypair]
/// instead of a [Box<dyn<Signer>>].
/// While this necessitates the removal of the `usb` and `pubkey` input prefixes,
/// It returns a concrete type, and also allows for flexibility
/// when designing a system to safely pass a keypair to an automated signing server at startup.
use anyhow::anyhow;
use solana_clap_v3_utils::keypair::keypair_from_seed_phrase;
use anchor_client::solana_sdk::derivation_path::{DerivationPath, DerivationPathError};
use anchor_client::solana_sdk::signature::{read_keypair, read_keypair_file, Keypair};
use thiserror::Error;

const STDOUT_OUTFILE_TOKEN: &str = "-";

struct SignerSource {
    pub kind: SignerSourceKind,
    pub derivation_path: Option<DerivationPath>,
    pub legacy: bool,
}

impl SignerSource {
    fn new(kind: SignerSourceKind) -> Self {
        Self {
            kind,
            derivation_path: None,
            legacy: false,
        }
    }
}

const SIGNER_SOURCE_PROMPT: &str = "prompt";
const SIGNER_SOURCE_FILEPATH: &str = "file";
const SIGNER_SOURCE_STDIN: &str = "stdin";

enum SignerSourceKind {
    Prompt,
    Filepath(String),
    Stdin,
}

impl AsRef<str> for SignerSourceKind {
    fn as_ref(&self) -> &str {
        match self {
            Self::Prompt => SIGNER_SOURCE_PROMPT,
            Self::Filepath(_) => SIGNER_SOURCE_FILEPATH,
            Self::Stdin => SIGNER_SOURCE_STDIN,
        }
    }
}

impl std::fmt::Debug for SignerSourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s: &str = self.as_ref();
        write!(f, "{}", s)
    }
}

#[derive(Debug, Error)]
enum SignerSourceError {
    #[error("unrecognized signer source")]
    UnrecognizedSource,
    #[error(transparent)]
    DerivationPathError(#[from] DerivationPathError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

fn parse_signer_source<S: AsRef<str>>(source: S) -> Result<SignerSource, SignerSourceError> {
    let source = source.as_ref().to_string();
    match uriparse::URIReference::try_from(source.as_str()) {
        Err(_) => Err(SignerSourceError::UnrecognizedSource),
        Ok(uri) => {
            if let Some(scheme) = uri.scheme() {
                let scheme = scheme.as_str().to_ascii_lowercase();
                match scheme.as_str() {
                    SIGNER_SOURCE_PROMPT => Ok(SignerSource {
                        kind: SignerSourceKind::Prompt,
                        derivation_path: DerivationPath::from_uri_any_query(&uri)?,
                        legacy: false,
                    }),
                    SIGNER_SOURCE_FILEPATH => Ok(SignerSource::new(SignerSourceKind::Filepath(
                        uri.path().to_string(),
                    ))),
                    SIGNER_SOURCE_STDIN => Ok(SignerSource::new(SignerSourceKind::Stdin)),
                    _ => {
                        Err(SignerSourceError::UnrecognizedSource)
                    }
                }
            } else {
                match source.as_str() {
                    STDOUT_OUTFILE_TOKEN => Ok(SignerSource::new(SignerSourceKind::Stdin)),
                    _ => std::fs::metadata(source.as_str())
                        .map(|_| SignerSource::new(SignerSourceKind::Filepath(source)))
                        .map_err(|err| err.into()),
                }
            }
        }
    }
}

/// Switches over only the allowed variants if what we need is a keypair,
/// including: `file`, `prompt`, `stdin`.
pub fn keypair_from_path(keypair_path: &str) -> anyhow::Result<Box<Keypair>> {
    let SignerSource {
        kind,
        derivation_path,
        legacy,
    } = parse_signer_source(keypair_path)?;
    match kind {
        SignerSourceKind::Prompt => Ok(Box::new(
            keypair_from_seed_phrase("keypair", false, false, derivation_path, legacy)
                .map_err(|e| anyhow!("Failed to read keypair from prompt: {:?}", e))?,
        )),
        SignerSourceKind::Filepath(path) => match read_keypair_file(path) {
            Err(e) => Err(anyhow!("Failed to read keypair from filepath: {:?}", e)),
            Ok(file) => Ok(Box::new(file)),
        },
        SignerSourceKind::Stdin => {
            let mut stdin = std::io::stdin();
            Ok(Box::new(read_keypair(&mut stdin).map_err(|e| {
                anyhow!("Failed to read keypair from stdin: {:?}", e)
            })?))
        }
    }
}