{{- define "hydracache.fullname" -}}
{{- default "hydracache" .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "hydracache.headless" -}}
{{- include "hydracache.fullname" . | trunc 54 | trimSuffix "-" -}}-headless
{{- end -}}

{{/*
Seed list: every StatefulSet ordinal's stable DNS name on the cluster port.
*/}}
{{- define "hydracache.seeds" -}}
{{- $name := include "hydracache.fullname" . -}}
{{- $headless := include "hydracache.headless" . -}}
{{- $port := .Values.service.clusterPort -}}
{{- $seeds := list -}}
{{- range $ordinal := until (int .Values.replicaCount) -}}
{{- $seeds = append $seeds (printf "%s-%d.%s:%v" $name $ordinal $headless $port) -}}
{{- end -}}
{{- join "," $seeds -}}
{{- end -}}

{{/*
Fail at render time for configurations the server refuses to start with.
*/}}
{{- define "hydracache.validate" -}}
{{- if lt (int .Values.replicaCount) 1 -}}
{{- fail "replicaCount must be at least 1" -}}
{{- end -}}
{{- if .Values.tls.enabled -}}
{{- if not .Values.tls.secretName -}}
{{- fail "tls.enabled requires tls.secretName: a Secret with tls.crt, tls.key and ca.crt" -}}
{{- end -}}
{{- if or (not .Values.clusterAuth.keyId) (not .Values.clusterAuth.secretName) -}}
{{- fail "tls.enabled requires clusterAuth.keyId and clusterAuth.secretName: a TLS member rejects unauthenticated raft peers and the cluster cannot form" -}}
{{- end -}}
{{- else if not .Values.tls.acknowledgeInsecure -}}
{{- fail "tls.enabled=false serves plaintext on non-loopback listeners; set tls.acknowledgeInsecure=true to accept that explicitly" -}}
{{- end -}}
{{- if ne (empty .Values.clusterAuth.keyId) (empty .Values.clusterAuth.secretName) -}}
{{- fail "clusterAuth.keyId and clusterAuth.secretName must be set together" -}}
{{- end -}}
{{- if and .Values.backup.enabled (not .Values.backup.location) -}}
{{- fail "backup.enabled requires backup.location" -}}
{{- end -}}
{{- end -}}
