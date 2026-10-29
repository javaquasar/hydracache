function Get-OperatorControlSelection {
    param([Parameter(Mandatory)][ValidateSet('reference','candidate')][string]$Phase)
    if($Phase -ceq 'reference') {
        return ,([string[]]@('model::operator_controls::legacy_counterexamples','--exact'))
    }
    return ,([string[]]@('model::candidate_controls::candidate_'))
}
