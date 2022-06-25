[![Artifact HUB](https://img.shields.io/endpoint?url=https://artifacthub.io/badge/repository/aws-eks-iam-auth-controller)](https://artifacthub.io/packages/search?repo=aws-eks-iam-auth-controller)

![OCI Images](https://github.com/rustrial/aws-eks-iam-auth-controller/workflows/oci-image/badge.svg)
![Publish Charts](https://github.com/rustrial/aws-eks-iam-auth-controller/workflows/publish-charts/badge.svg)

# AWS EKS iam-auth-controller

[Kubernetes Controller](https://kubernetes.io/docs/concepts/architecture/controller/) tracking
[`IAMIdentityMapping`](./deploy/helm/rustrial-aws-eks-iam-auth-controller/crds/iamidentitymappings.yml)
[Custom Resource](https://kubernetes.io/docs/concepts/extend-kubernetes/api-extension/custom-resources/)
objects to maintain the AWS EKS [`aws-auth` ConfigMap](https://docs.aws.amazon.com/eks/latest/userguide/add-user-role.html).

## Background & Motivation for this project

[AWS EKS](https://aws.amazon.com/eks) uses the
[`aws-auth`](https://docs.aws.amazon.com/eks/latest/userguide/add-user-role.html) ConfigMap in the
`kube-system` namespace to map authenticated identities to Kubernetes username and groups. Using a single ConfigMap
makes it hard and error prone to manage identity mappings using GitOps approach.
The _Kubernetes SIG's_ [AWS IAM Authenticator for Kubernetes](https://github.com/kubernetes-sigs/aws-iam-authenticator)
addresses this by providing a `IAMIdentityMapping` _Custom Resource_. However, that _Custom Resource_ is still in
alpha stage and is not yet enabled on the EKS control plane (master nodes).

This _Kubernetes Controller_ closes the gap by implementing a _Custom Resource Controller_,
updating the `aws-auth` ConfigMap from `IAMIdentityMapping` objects.
Once [#550](https://github.com/aws/containers-roadmap/issues/550) or
[#512](https://github.com/aws/containers-roadmap/issues/512) is resolved this controller will no longer be needed.

## Examples

```yaml
---
apiVersion: iamauthenticator.k8s.aws/v1alpha1
kind: IAMIdentityMapping
metadata:
  name: kubernetes-admin-user
spec:
  arn: arn:aws:iam::{{ accountId }}:user/KubernetesAdmin
  username: kubernetes-admin
  groups:
    - system:masters
---
apiVersion: iamauthenticator.k8s.aws/v1alpha1
kind: IAMIdentityMapping
metadata:
  name: kubernetes-admin-role
spec:
  arn: arn:aws:iam::XXXXXXXXXXXX:role/KubernetesAdmin
  username: kubernetes-admin
  groups:
    - system:masters
```

## Getting Started

**Add Helm Repository**

_AWS EKS iam-auth-controller_ can be installed via Helm Chart, which by default will use the prebuilt OCI Images for Linux (`amd64` and `arm64`) from [DockerHub](https://hub.docker.com/r/rustrial/aws-eks-iam-auth-controller).

```shell
helm repo add aws-eks-iam-auth-controller https://rustrial.github.io/aws-eks-iam-auth-controller
```

**Install Helm Chart**

```shell
helm install my-rustrial-aws-eks-iam-auth-controller aws-eks-iam-auth-controller/rustrial-aws-eks-iam-auth-controller --version 0.1.0
```

### Adding default IAMIdentityMapping objects for EKS Nodes

As it is implemented today, the controller does only reconcile `IAMIdentityMapping` objects, and will overwrite (remove) all entries in `aws-auth` which have no corresponding `IAMIdentityMapping` object.
To enable your EKS worker nodes respectively Fargate nodes to join your cluster, you have to deploy 
the corresponding `IAMIdentityMapping` objects like this:

```yaml
kubectl apply -f- <<EOF
---
apiVersion: iamauthenticator.k8s.aws/v1alpha1
kind: IAMIdentityMapping
metadata:
  name: aws-ec2-nodes
spec:
  arn: 'arn:aws:iam::999999999999:role/your-ec2-node-role-name-here'
  groups:
    - 'system:bootstrappers'
    - 'system:nodes'
  username: 'system:node:{{EC2PrivateDNSName}}'
---
apiVersion: iamauthenticator.k8s.aws/v1alpha1
kind: IAMIdentityMapping
metadata:
  name: aws-fargate-nodes
spec:
  arn: 'arn:aws:iam::999999999999:role/your-fargate-node-role-name-here'
  groups:
    - 'system:bootstrappers'
    - 'system:nodes'
    - 'system:node-proxier'
  username: 'system:node:{{SessionName}}'
EOF
```

---

## License

Licensed under either of

- Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- The Unlicense
  ([UNLICENSE](LUNLICENSE) or https://opensource.org/licenses/unlicense)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
triple licensed as above, without any additional terms or conditions. See the
[WAIVER](WAIVER) and [CONTRIBUTING.md](CONTRIBUTING.md) files for more information.
