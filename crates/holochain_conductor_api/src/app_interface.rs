use crate::{signal_subscription::SignalSubscription, ExternalApiWireError};
use holo_hash::AgentPubKey;
use holochain_keystore::LairResult;
use holochain_keystore::MetaLairClient;
use holochain_types::prelude::*;

/// Represents the available conductor functions to call over an app interface
/// and will result in a corresponding [`AppResponse`] message being sent back over the
/// interface connection.
///
/// # Errors
///
/// Returns an [`AppResponse::Error`] with a reason why the request failed.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, SerializedBytes)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum AppRequest {
    /// Get info about the app identified by the given `installed_app_id` argument,
    /// including info about each cell installed by this app.
    ///
    /// Requires `installed_app_id`, because an app interface can be the interface to multiple
    /// apps at the same time.
    ///
    /// # Returns
    ///
    /// [`AppResponse::AppInfo`]
    AppInfo {
        /// The app ID for which to get information
        installed_app_id: InstalledAppId,
    },
    /// Is currently unimplemented and will return
    /// an [`AppResponse::Unimplemented`].
    Crypto(Box<CryptoRequest>),
    /// Call a zome function. See [`ZomeCall`]
    /// to understand the data that must be provided.
    ///
    /// # Returns
    ///
    /// [`AppResponse::ZomeCall`]
    ZomeCall(Box<SignedSerializedZomeCall>),

    #[deprecated = "use ZomeCall"]
    ZomeCallInvocation(Box<SignedSerializedZomeCall>),

    /// Is currently unimplemented and will return
    /// an [`AppResponse::Unimplemented`].
    SignalSubscription(SignalSubscription),
}

/// Represents the possible responses to an [`AppRequest`].
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, SerializedBytes)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum AppResponse {
    /// This request is unimplemented
    Unimplemented(AppRequest),

    /// Can occur in response to any [`AppRequest`].
    ///
    /// There has been an error during the handling of the request.
    Error(ExternalApiWireError),

    /// The succesful response to an [`AppRequest::AppInfo`].
    ///
    /// Option will be `None` if there is no installed app with the given `installed_app_id`.
    /// Check out [`InstalledApp`] for details on when the option is `Some<InstalledAppInfo>`
    AppInfo(Option<InstalledAppInfo>),

    /// The successful response to an [`AppRequest::ZomeCall`].
    ///
    /// Note that [`ExternIO`] is simply a structure of [`struct@SerializedBytes`], so the client will have
    /// to decode this response back into the data provided by the zome using a [msgpack] library to utilize it.
    ///
    /// [msgpack]: https://msgpack.org/
    ZomeCall(Box<ExternIO>),

    #[deprecated = "use ZomeCall"]
    ZomeCallInvocation(Box<ExternIO>),
}

/// The data provided over an app interface in order to make a zome call
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SignedSerializedZomeCall {
    pub provenance: AgentPubKey,
    pub signature: Signature,
    pub serialized_zome_call: SerializedBytes,
}

impl SignedSerializedZomeCall {
    pub async fn try_from_unsigned_zome_call(
        keystore: &MetaLairClient,
        zome_call: ZomeCallUnsigned,
    ) -> LairResult<Self> {
        let signature = zome_call.sign(keystore).await?;
        Ok(Self {
            provenance: zome_call.provenance.clone(),
            signature,
            serialized_zome_call: SerializedBytes::try_from(zome_call).map_err(|e| one_err::OneErr::new(e.to_string()))?,
        })
    }

    pub async fn resign_zome_call(
        self,
        keystore: &MetaLairClient,
        agent_key: AgentPubKey,
    ) -> LairResult<Self> {
        let mut zome_call_unsigned: ZomeCallUnsigned = ZomeCallUnsigned::try_from(self.serialized_zome_call).map_err(|e| one_err::OneErr::new(e.to_string()))?;
        zome_call_unsigned.provenance = agent_key;
        SignedSerializedZomeCall::try_from_unsigned_zome_call(keystore, zome_call_unsigned).await
    }
}

#[allow(missing_docs)]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "data")]
pub enum CryptoRequest {
    Sign(String),
    Decrypt(String),
    Encrypt(String),
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, SerializedBytes)]
/// Info about an installed app, returned as part of [`AppResponse::AppInfo`]
pub struct InstalledAppInfo {
    /// The unique identifier for an installed app in this conductor
    pub installed_app_id: InstalledAppId,
    /// Info about the cells installed in this app
    pub cell_data: Vec<InstalledCell>,
    /// The app's current status, in an API-friendly format
    pub status: InstalledAppInfoStatus,
}

impl InstalledAppInfo {
    pub fn from_installed_app(app: &InstalledApp) -> Self {
        let installed_app_id = app.id().clone();
        let status = app.status().clone().into();
        let cell_data = app
            .provisioned_cells()
            .map(|(role_id, id)| InstalledCell::new(id.clone(), role_id.clone()))
            .collect();
        Self {
            installed_app_id,
            cell_data,
            status,
        }
    }
}

impl From<&InstalledApp> for InstalledAppInfo {
    fn from(app: &InstalledApp) -> Self {
        Self::from_installed_app(app)
    }
}

/// A flat, slightly more API-friendly representation of [`InstalledAppInfo`]
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, SerializedBytes)]
#[serde(rename_all = "snake_case")]
pub enum InstalledAppInfoStatus {
    Paused { reason: PausedAppReason },
    Disabled { reason: DisabledAppReason },
    Running,
}

impl From<AppStatus> for InstalledAppInfoStatus {
    fn from(i: AppStatus) -> Self {
        match i {
            AppStatus::Running => InstalledAppInfoStatus::Running,
            AppStatus::Disabled(reason) => InstalledAppInfoStatus::Disabled { reason },
            AppStatus::Paused(reason) => InstalledAppInfoStatus::Paused { reason },
        }
    }
}

impl From<InstalledAppInfoStatus> for AppStatus {
    fn from(i: InstalledAppInfoStatus) -> Self {
        match i {
            InstalledAppInfoStatus::Running => AppStatus::Running,
            InstalledAppInfoStatus::Disabled { reason } => AppStatus::Disabled(reason),
            InstalledAppInfoStatus::Paused { reason } => AppStatus::Paused(reason),
        }
    }
}

#[test]
fn status_serialization() {
    use kitsune_p2p::dependencies::kitsune_p2p_types::dependencies::serde_json;

    let status: InstalledAppInfoStatus =
        AppStatus::Disabled(DisabledAppReason::Error("because".into())).into();

    assert_eq!(
        serde_json::to_string(&status).unwrap(),
        "{\"disabled\":{\"reason\":{\"error\":\"because\"}}}"
    );

    let status: InstalledAppInfoStatus =
        AppStatus::Paused(PausedAppReason::Error("because".into())).into();

    assert_eq!(
        serde_json::to_string(&status).unwrap(),
        "{\"paused\":{\"reason\":{\"error\":\"because\"}}}"
    );

    let status: InstalledAppInfoStatus = AppStatus::Disabled(DisabledAppReason::User).into();

    assert_eq!(
        serde_json::to_string(&status).unwrap(),
        "{\"disabled\":{\"reason\":\"user\"}}"
    );
}
