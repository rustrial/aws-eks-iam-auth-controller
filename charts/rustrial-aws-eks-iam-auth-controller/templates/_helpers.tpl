{{/*
Expand the name of the chart.
*/}}
{{- define "rustrial-aws-eks-iam-auth-controller.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "rustrial-aws-eks-iam-auth-controller.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "rustrial-aws-eks-iam-auth-controller.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "rustrial-aws-eks-iam-auth-controller.labels" -}}
helm.sh/chart: {{ include "rustrial-aws-eks-iam-auth-controller.chart" . }}
{{ include "rustrial-aws-eks-iam-auth-controller.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "rustrial-aws-eks-iam-auth-controller.selectorLabels" -}}
app.kubernetes.io/name: {{ include "rustrial-aws-eks-iam-auth-controller.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Pod labels
*/}}
{{- define "rustrial-aws-eks-iam-auth-controller.podLabels" -}}
{{ include "rustrial-aws-eks-iam-auth-controller.selectorLabels" . }}
{{- if .Values.podLabels }}
{{ .Values.podLabels | toYaml }}
{{- end -}}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "rustrial-aws-eks-iam-auth-controller.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "rustrial-aws-eks-iam-auth-controller.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}
