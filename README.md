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
  arn: arn:aws:iam::XXXXXXXXXXXX:user/KubernetesAdmin
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
