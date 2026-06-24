{{- define "hydracache.fullname" -}}
{{- default "hydracache" .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
