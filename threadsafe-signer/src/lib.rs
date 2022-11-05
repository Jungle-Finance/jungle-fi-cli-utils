/// The struct below wraps the often-acquired [Box<dyn<Signer>>] to:
/// 1. Satisfy [T: Signer].
/// 2. Is threadsafe.
/// 3. Serve as a signer with the [Clone] trait.
use std::sync::{Arc, Mutex};
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::{Signature, Signer, SignerError};

/// Basic struct that [impl Signer + Clone + Send + Sync].
pub struct ThreadsafeSigner {
    pub inner: Arc<Mutex<Box<dyn Signer>>>,
}

impl Clone for ThreadsafeSigner {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Signer for ThreadsafeSigner {
    fn try_pubkey(&self) -> Result<Pubkey, SignerError> {
        Ok(self.inner.lock().unwrap().pubkey())
    }

    fn try_sign_message(&self, message: &[u8]) -> Result<Signature, SignerError> {
        self.inner.lock().unwrap().try_sign_message(message)
    }

    fn is_interactive(&self) -> bool {
        self.inner.lock().unwrap().is_interactive()
    }
}
