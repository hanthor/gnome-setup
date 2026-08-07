//! API for enrolling credentials into LUKS2 volumes via systemd-cryptenroll.

use serde::{Deserialize, Serialize};
use zlink::futures_util::Stream;
use zlink::{proxy, ReplyError};

/// Proxy trait for calling methods on the interface.
#[proxy("io.systemd.CryptEnroll")]
pub trait CryptEnroll {
    /// Enroll a recovery key and wipe key slots in LUKS2 volume.
    async fn enroll(
        &mut self,
        node: &str,
        mechanism: &EnrollMechanism,
        #[zlink(rename = "unlockPassword")] unlock_password: Option<&str>,
        #[zlink(rename = "wipeSlots")] wipe_slots: Option<&[i64]>,
    ) -> zlink::Result<Result<EnrollOutput<'_>, CryptEnrollError>>;

    /// Enumerate the keyslots currently enrolled in a LUKS2 volume, one per reply. Must be called with the 'more' flag.
    #[zlink(more)]
    async fn list_slots(
        &mut self,
        node: &str,
    ) -> zlink::Result<impl Stream<Item = zlink::Result<Result<ListSlotsOutput, CryptEnrollError>>>>;
}

/// Output parameters for the Enroll method.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnrollOutput<'a> {
    pub slot: Option<i64>,
    #[serde(rename = "wipedSlots")]
    pub wiped_slots: Option<Vec<i64>>,
    #[serde(borrow)]
    #[serde(rename = "recoveryKey")]
    pub recovery_key: Option<&'a str>,
    #[serde(borrow)]
    pub state: Option<&'a str>,
}

/// Output parameters for the ListSlots method.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListSlotsOutput {
    pub slot: Option<i64>,
    pub r#type: Option<EnrollMechanism>,
}

/// The credential types this interface knows about, used both as the enrollment mechanism and as the slot type reported by ListSlots.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnrollMechanism {
    /// A regular passphrase
    Password,
    /// A randomly generated recovery key
    Recovery,
    /// A PKCS#11 security token (not enrollable via this interface)
    Pkcs11,
    /// A FIDO2 security token
    Fido2,
    /// A TPM2 device (not enrollable via this interface)
    Tpm2,
}

/// Errors that can occur in this interface.
#[derive(Debug, Clone, PartialEq, ReplyError)]
#[zlink(interface = "io.systemd.CryptEnroll")]
pub enum CryptEnrollError {
    /// The volume is managed by another subsystem (e.g. systemd-homed) and may not be enrolled into directly.
    VolumeUnderForeignManagement,
    /// A password is required to unlock the volume, but none was provided.
    PasswordRequired,
    /// The provided password did not unlock the volume.
    PasswordIncorrect,
    /// No matching FIDO2 device was found.
    FidoDeviceNotFound,
    /// The FIDO2 token was not interacted with in time.
    FidoActionTimeout,
}
