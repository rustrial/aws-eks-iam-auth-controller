#[macro_use]
extern crate log;

use anyhow::Context;
use futures::StreamExt;
use k8s_openapi::{api::core::v1::ConfigMap, apimachinery::pkg::apis::meta::v1::ObjectMeta};
use kube::{
    api::{ListParams, Patch, PatchParams},
    Api, Client, CustomResource,
};
use kube_runtime::{
    controller::{Action, Context as Ctx, Controller},
    reflector::Store,
};
use hyper::{Client as HyperClient, body::Buf};
use log::info;
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, Deserializer};
use std::{
    fmt,
    collections::BTreeMap,
    collections::HashMap,
    sync::Arc,
    time::Instant,
};
use tokio::{
    time::Duration,
    time::timeout,
    sync::OnceCell,
};
use regex::Regex;

const AWS_AUTH: &str = "aws-auth";

const KUBE_SYSTEM: &str = "kube-system";

const INSTANCE_METADATA_ENDPOINT: &str = "http://169.254.169.254/latest/dynamic/instance-identity/document";

static INSTANCE_METADATA: OnceCell<HashMap<String, String>> = OnceCell::const_new();

#[derive(thiserror::Error, Debug)]
enum CrdError {
    #[error("{0}")]
    Any(String),
}

impl From<anyhow::Error> for CrdError {
    fn from(e: anyhow::Error) -> Self {
        CrdError::Any(format!("{}", e))
    }
}

pub struct Template {
    src: String,
    matches: Vec<(usize, usize)>,
}

impl Template {
    pub fn new(template: &str) -> Self {
        let regex = Regex::new(r"\{\{([^}]*)\}\}").unwrap();

        Template {
            src: template.to_owned(),
            matches: regex
                .find_iter(template)
                .map(|m| (m.start(), m.end()))
                .collect(),
        }
    }

    pub fn render(&self, vals: &HashMap<String, String>) -> String {
        let mut parts: Vec<&str> = vec![];
        let template_str = &self.src;

        let first = match self.matches.first() {
            Some((start, _)) => *start,
            _ => return template_str.clone(),
        };

        if first > 0 {
            parts.push(&template_str[0..first])
        }

        let mut prev_end: Option<usize> = None;

        for (start, end) in self.matches.iter() {
            if let Some(last_end) = prev_end {
                parts.push(&template_str[last_end..*start])
            }
            let arg = &template_str[*start..*end];
            let arg_name = &arg[2..arg.len() - 2];
            match vals.get(arg_name) {
                Some(s) => parts.push(s),
                _ => parts.push(arg),
            }

            prev_end = Some(*end);
        }

        let template_len = template_str.len();
        if let Some(last_pos) = prev_end {
            if last_pos < template_len {
                parts.push(&template_str[last_pos..template_len])
            }
        }

        parts.join("")
    }
}

/// Custom Resource as defined by the
/// [aws-iam-authenticator project](https://github.com/kubernetes-sigs/aws-iam-authenticator/blob/master/deploy/iamidentitymapping.yaml).
#[derive(CustomResource, Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq)]
#[kube(
    group = "iamauthenticator.k8s.aws",
    version = "v1alpha1",
    kind = "IAMIdentityMapping",
    derive = "PartialEq",
    status = "IAMIdentityMappingStatus"
)]
struct IAMIdentityMappingSpec {
    arn: String,
    username: String,
    groups: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
struct IAMIdentityMappingStatus {
    status: String,
}

#[derive(Serialize, Debug, PartialEq, Clone)]
struct ARN(String);

impl fmt::Display for ARN {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.0)
    }
}

impl<'de> Deserialize<'de> for ARN {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        let template = Template::new(s);
        Ok(ARN(template.render(&INSTANCE_METADATA.get().unwrap())))
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct MapRole {
    pub username: String,
    pub groups: Option<Vec<String>>,
    pub rolearn: ARN,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct MapUser {
    pub username: String,
    pub groups: Option<Vec<String>>,
    pub userarn: ARN,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct InstanceMetadata {
    pub account_id: String,
}

impl InstanceMetadata {
    async fn get_metadata() -> Result<InstanceMetadata, Box<dyn std::error::Error>> {
        let client = HyperClient::new();
        let url = INSTANCE_METADATA_ENDPOINT.parse::<hyper::Uri>()?;
        let response = client.get(url).await?;
        let body = hyper::body::aggregate(response).await?;
        Ok(serde_json::from_reader(body.reader())?)
    }

    pub fn get(&self) -> HashMap<String, String> {
        let metadata_str = serde_json::to_string(self).unwrap();
        serde_json::from_str(metadata_str.as_str()).unwrap()
    }
}

/// Controller triggers this whenever our main object or our children changed
async fn reconcile(mapping: Arc<IAMIdentityMapping>, ctx: Ctx<Data>) -> Result<Action, CrdError> {
    let start = Instant::now();
    info!("reconile {:?}", mapping);
    let client = ctx.get_ref().client.clone();
    let cm_api = Api::<ConfigMap>::namespaced(client.clone(), KUBE_SYSTEM);
    let cm = cm_api.get(AWS_AUTH).await;
    info!("Got existing ConfigMap: {:?}", cm);
    let cm = cm.ok();

    let (roles, users) = cm
        .map(|v| v.data)
        .flatten()
        .map(|d| {
            (
                d.get("mapRoles")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "[]".to_string()),
                d.get("mapUsers")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "[]".to_string()),
            )
        })
        .unwrap_or_else(|| ("[]".to_string(), "[]".to_string()));
    let mut roles: Vec<MapRole> =
        serde_yaml::from_str(roles.as_str()).context("Error while deserializing mapRoles")?;
    let mut users: Vec<MapUser> =
        serde_yaml::from_str(users.as_str()).context("Error while deserializing mapUsers")?;

    let state: Vec<Arc<IAMIdentityMapping>> = ctx.get_ref().store.clone().state();
    // Remove all ConfitMap entries, which have no corresponding CustomResource.
    roles.retain(|r| state.iter().find(|v| r.rolearn.to_string() == v.spec.arn).is_some());
    users.retain(|r| state.iter().find(|v| r.userarn.to_string() == v.spec.arn).is_some());
    // Upsert (add/update) ConfigMap entries for CustomerResources.
    for item in state {
        let spec: &IAMIdentityMappingSpec = &item.spec;
        if spec.arn.contains(":role/") {
            // optionally, remove already existing ConfigMap entry.
            roles.retain(|r| r.rolearn.to_string() != spec.arn);
            roles.push(MapRole {
                rolearn: ARN(spec.arn.clone()),
                username: spec.username.clone(),
                groups: spec.groups.clone(),
            });
        } else {
            // optionally, remove already existing ConfigMap entry.
            users.retain(|r| r.userarn.to_string() != spec.arn);
            users.push(MapUser {
                userarn: ARN(spec.arn.clone()),
                username: spec.username.clone(),
                groups: spec.groups.clone(),
            });
        }
    }
    let mut contents = BTreeMap::new();
    contents.insert(
        "mapRoles".to_string(),
        serde_yaml::to_string(&roles).context("Error while serializing mapRoles")?,
    );
    contents.insert(
        "mapUsers".to_string(),
        serde_yaml::to_string(&users).context("Error while serializing mapUsers")?,
    );
    let cm = ConfigMap {
        metadata: ObjectMeta {
            name: Some(AWS_AUTH.to_string()),
            namespace: Some(KUBE_SYSTEM.to_string()),
            ..ObjectMeta::default()
        },
        data: Some(contents),
        ..Default::default()
    };
    info!("ConfigMap changeset: {:?}", cm);
    cm_api
        .patch(
            AWS_AUTH,
            &PatchParams {
                field_manager: Some("aws-eks-iam-auth-controller.rustrial.org".to_string()),
                dry_run: false,
                force: true,
            },
            &Patch::Apply(cm),
        )
        .await
        .context("Failed to create ConfigMap")?;
    let duration = Instant::now() - start;
    histogram!("reconcile_duration_ns", duration.as_nanos() as f64);
    Ok(Action::requeue(Duration::from_secs(900)))
}

/// The controller triggers this on reconcile errors
fn error_policy(_error: &CrdError, _ctx: Ctx<Data>) -> Action {
    Action::requeue(Duration::from_secs(10))
}

// Data we want access to in error/reconcile calls
struct Data {
    client: Client,
    store: Store<IAMIdentityMapping>,
}

async fn scheduled_statistics(store: Store<IAMIdentityMapping>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        gauge!("custom_resource_count", store.state().len() as f64);
        trace!("custom_resource_count {}", store.state().len());
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let metadata: InstanceMetadata = match timeout(Duration::from_secs(5), InstanceMetadata::get_metadata()).await {
        Ok(to) => to.unwrap_or_else(|err| {
            warn!("Failed to get metadata: {:?}", err);
            Default::default()
        }),
        Err(_) => {
            warn!("Failed to get metadata within timeout. Setting with default values!");
            Default::default()
        }
    };
    INSTANCE_METADATA.set(metadata.get()).unwrap();
    let metrics_builder = PrometheusBuilder::new();
    metrics_builder.install()?;
    let client = Client::try_default().await?;
    let iam_identity_mappings = Api::<IAMIdentityMapping>::all(client.clone());
    let controller = Controller::new(iam_identity_mappings, ListParams::default());
    let store = controller.store();
    let schedule = tokio::spawn(scheduled_statistics(store.clone()));
    let controller = controller
        .run(reconcile, error_policy, Ctx::new(Data {client, store}))
        .for_each(|res| async move {
            match res {
                Ok(o) => {
                    counter!("reconcile_success", 1);
                    info!("reconciled {:?}", o)
                }
                Err(e) => {
                    counter!("reconcile_failure", 1);
                    warn!("reconcile failed: {}", e)
                }
            }
        });
    tokio::select! {
       _ = schedule => (),
       _ = controller => (),
    };
    Ok(())
}
