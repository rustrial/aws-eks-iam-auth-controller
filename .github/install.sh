#!/bin/bash

set -e

IMG=test/aws-eks-iam-auth-controller

docker image build -t test/aws-eks-iam-auth-controller:latest .

kind load docker-image test/aws-eks-iam-auth-controller:latest --name "${KIND:-kind}"

helm upgrade aws-eks-iam-auth-controller charts/rustrial-aws-eks-iam-auth-controller \
    --install -n kube-system \
    --create-namespace \
    --set fullnameOverride=aws-eks-iam-auth-controller \
    --set image.repository=test/aws-eks-iam-auth-controller \
    --set image.tag=latest
